mod config;
mod daum;

pub use config::Config;
use serenity::builder::{CreateEmbed, EditMessage};
use serenity::model::channel::Message;
use serenity::model::Colour;
use serenity::prelude::{Context, EventHandler, GatewayIntents};

struct Handler {
    http_agent: ureq::Agent,
    idk: String,
    loading: String,
}

impl Handler {
    fn new(config: &Config) -> Self {
        Self {
            http_agent: ureq::Agent::new_with_defaults(),
            idk: config.emoji.idk.clone().unwrap_or("idk".into()),
            loading: config.emoji.loading.clone().unwrap_or("loading".into()),
        }
    }
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        use std::fmt::Write;

        let text = &msg.content;
        if let Some(query) = text.strip_prefix("d ").or_else(|| text.strip_prefix("D ")) {
            let mut reply = msg
                .reply_mention(
                    &ctx.http,
                    format!("{query} {loading}", loading = self.loading),
                )
                .await
                .unwrap();
            let edit = match daum::search_general(&self.http_agent, query) {
                None => EditMessage::new().content(self.idk.clone()),
                Some(daum::GeneralSearch::En {
                    lemma,
                    definitions,
                    phonetics,
                }) => {
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
                            write!(&mut content, "1. {first}").unwrap();
                            for (i, def) in defs.iter().enumerate() {
                                write!(&mut content, " {num}. {def}", num = i + 2).unwrap();
                            }
                        }
                    }
                    EditMessage::new().content("").add_embed(
                        CreateEmbed::new()
                            .title(lemma)
                            .colour(Colour::new(0x0090ff))
                            .description(content),
                    )
                }
                Some(daum::GeneralSearch::Ko {
                    lemma,
                    definitions,
                    foreign,
                    phonetics,
                }) => {
                    let mut title = String::new();
                    title.push_str(&lemma);
                    if let Some(foreign) = foreign {
                        write!(&mut title, " `{foreign}`").unwrap();
                    }
                    let mut content = String::new();
                    if let Some(phonetics) = phonetics {
                        writeln!(&mut content, "-# {phonetics}").unwrap();
                    }
                    if let Some((first, defs)) = definitions.split_first() {
                        if defs.is_empty() {
                            content.push_str(first);
                        } else {
                            write!(&mut content, "1. {first}").unwrap();
                            for (i, def) in defs.iter().enumerate() {
                                write!(&mut content, "\n{num}. {def}", num = i + 2).unwrap();
                            }
                        }
                    }
                    EditMessage::new().content("").add_embed(
                        CreateEmbed::new()
                            .title(title)
                            .colour(Colour::new(0x0090ff))
                            .description(content),
                    )
                }
            };
            reply.edit(&ctx.http, edit).await.unwrap();
        }
    }
}

pub async fn serenity_client(config: &config::Config) -> serenity::Client {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    serenity::Client::builder(&config.bot_token, intents)
        .event_handler(Handler::new(config))
        .await
        .unwrap()
}
