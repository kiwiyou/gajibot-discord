mod config;
mod daum;

pub use config::Config;
use serenity::all::CreateEmbedFooter;
use serenity::builder::{CreateEmbed, EditMessage};
use serenity::model::channel::Message;
use serenity::model::Colour;
use serenity::prelude::{Context, EventHandler, GatewayIntents};

struct Handler {
    http_agent: ureq::Agent,
    idk: String,
    loading: String,
    url_maybe: Option<String>,
}

impl Handler {
    fn new(config: &Config) -> Self {
        Self {
            http_agent: ureq::Agent::new_with_defaults(),
            idk: config.emoji.idk.clone().unwrap_or("idk".into()),
            loading: config.emoji.loading.clone().unwrap_or("loading".into()),
            url_maybe: config.url_maybe.clone(),
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
                Some(search) => {
                    let embed = match search.hit {
                        daum::GeneralSearchHit::En {
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
                                    write!(&mut content, "1. {first}").unwrap();
                                    for (i, def) in defs.iter().enumerate() {
                                        write!(&mut content, " {num}. {def}", num = i + 2).unwrap();
                                    }
                                }
                            }
                            CreateEmbed::new().title(lemma).description(content)
                        }
                        daum::GeneralSearchHit::Ko {
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
                                        write!(&mut content, "\n{num}. {def}", num = i + 2)
                                            .unwrap();
                                    }
                                }
                            }
                            CreateEmbed::new().title(title).description(content)
                        }
                    };
                    let embed = if search.typo.is_empty() {
                        embed
                    } else {
                        let footer = CreateEmbedFooter::new(search.typo.join(", "));
                        let footer = if let Some(url) = self.url_maybe.clone() {
                            footer.icon_url(url)
                        } else {
                            footer
                        };
                        embed.footer(footer)
                    }
                    .colour(Colour::new(0x0090ff));
                    EditMessage::new().content("").add_embed(embed)
                }
            };
            reply.edit(&ctx.http, edit).await.unwrap();
        }
    }
}

pub async fn serenity_client(config: &config::Config) -> serenity::Client {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    serenity::Client::builder(&config.token, intents)
        .event_handler(Handler::new(config))
        .await
        .unwrap()
}
