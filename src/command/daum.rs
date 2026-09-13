use std::fmt::Write;
use std::sync::Arc;

use twilight_http::Client;
use twilight_model::{
    channel::message::{embed::EmbedFooter, Embed, MessageFlags},
    gateway::payload::incoming::MessageCreate,
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
};

use crate::{
    command::CommandHandler,
    config::Config,
    integration::{DaumClient, GeneralSearchHit},
};

pub struct DaumCommand {
    query: String,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
    config: Arc<Config>,
    client: Arc<Client>,
    daum: Arc<DaumClient>,
}

impl CommandHandler {
    pub fn extract_daum(&self, event: &MessageCreate) -> Option<DaumCommand> {
        let text = &event.content;
        text.strip_prefix("d ")
            .or_else(|| text.strip_prefix("D "))
            .map(|query| DaumCommand {
                query: query.to_owned(),
                channel_id: event.channel_id,
                message_id: event.id,
                config: self.config.clone(),
                client: self.client.clone(),
                daum: self.daum_client.clone(),
            })
    }
}

impl DaumCommand {
    pub async fn handle(self) {
        let Self {
            query,
            channel_id,
            message_id,
            config,
            client,
            daum,
        } = self;
        let reply = client
            .create_message(channel_id)
            .reply(message_id)
            .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
            .content(&format!(
                "{query} {loading}",
                loading = config
                    .daum
                    .loading_emoji
                    .as_deref()
                    .unwrap_or("Searching..."),
            ))
            .await
            .unwrap()
            .model()
            .await
            .unwrap();
        let edit = client.update_message(reply.channel_id, reply.id);
        match daum.search_general(&query).await {
            None => {
                let idk = config.daum.idk_emoji.as_deref().unwrap_or("No result");
                edit.content(Some(idk)).await.unwrap();
            }
            Some(search) => {
                let mut embed = Embed {
                    author: None,
                    color: Some(0x0090ff),
                    description: None,
                    fields: vec![],
                    footer: None,
                    image: None,
                    kind: "rich".into(),
                    provider: None,
                    thumbnail: None,
                    timestamp: None,
                    title: None,
                    url: None,
                    video: None,
                };
                match search.hit {
                    GeneralSearchHit::En {
                        lemma,
                        definitions,
                        phonetics,
                    } => {
                        let mut content = String::new();
                        if let Some((ame, bre)) = phonetics {
                            content.push_str("-# ");
                            if ame == bre {
                                content.push_str(&bre);
                            } else {
                                write!(&mut content, "美 {ame} 英 {bre}").unwrap();
                            }
                            content.push('\n');
                        }
                        if let Some((first, defs)) = definitions.split_first() {
                            if defs.is_empty() {
                                content.push_str(first);
                            } else {
                                write!(&mut content, r"1\. {first}").unwrap();
                                for (i, def) in defs.iter().enumerate() {
                                    write!(&mut content, " {num}. {def}", num = i + 2).unwrap();
                                }
                            }
                        }
                        embed.title = Some(lemma);
                        embed.description = Some(content);
                    }
                    GeneralSearchHit::Ko {
                        lemma,
                        definitions,
                        foreign,
                        phonetics,
                    } => {
                        let mut title = String::new();
                        title.push_str(&lemma);
                        if let Some(foreign) = foreign {
                            write!(&mut title, " `{foreign}`").unwrap();
                        }
                        let mut description = String::new();
                        if let Some(phonetics) = phonetics {
                            writeln!(&mut description, "-# {phonetics}").unwrap();
                        }
                        if let Some((first, defs)) = definitions.split_first() {
                            if defs.is_empty() {
                                description.push_str(first);
                            } else {
                                write!(&mut description, "1. {first}").unwrap();
                                for (i, def) in defs.iter().enumerate() {
                                    write!(&mut description, "\n{num}. {def}", num = i + 2)
                                        .unwrap();
                                }
                            }
                        }
                        embed.title = Some(title);
                        embed.description = Some(description);
                    }
                };
                if !search.typo.is_empty() {
                    embed.footer = Some(EmbedFooter {
                        icon_url: config.daum.maybe_url.clone(),
                        proxy_icon_url: None,
                        text: search.typo.join(", "),
                    });
                }
                edit.content(None).embeds(Some(&[embed])).await.unwrap();
            }
        }
    }
}
