use std::sync::Arc;

use anyhow::Context as _;
use poise::{serenity_prelude as serenity, CreateReply};
use scraper::{Html, Selector};
use serenity::prelude::*;
use shuttle_runtime::SecretStore;

struct Data {
    client: reqwest::Client,
    hanja: Hanja,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

struct Hanja {
    read: Selector,
    ruby: Selector,
    reading: Selector,
    refer: Selector,
}

impl Hanja {
    fn new() -> Self {
        Self {
            read: Selector::parse(".txt_read").unwrap(),
            ruby: Selector::parse(".desc_ruby").unwrap(),
            reading: Selector::parse(".desc_ex").unwrap(),
            refer: Selector::parse(".txt_refer.on").unwrap(),
        }
    }
}

/// Search hanja
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    required_permissions = "SEND_MESSAGES"
)]
async fn hanja(ctx: Context<'_>, hanja: String) -> Result<(), Error> {
    struct HanjaInfo {
        reading: String,
        description: String,
    }
    let result = ctx
        .reply(format!(
            "Searching for {} <a:Loading:1363125483667193998>",
            hanja
        ))
        .await?;
    let Some(url_back) = ('entry: {
        let search_list = ctx
            .data()
            .client
            .get("https://dic.daum.net/search.do")
            .query(&[("dic", "hanja"), ("q", &hanja)])
            .send()
            .await?
            .text()
            .await?;

        if let Some((_, link_start)) = search_list.split_once("/word/view.do?wordid=") {
            if let Some((url_back, rest)) = link_start.split_once('"') {
                match rest.split_once(r#"class="txt_emph1">"#) {
                    Some((_, x)) if x.starts_with(&hanja) => {
                        break 'entry Some(url_back.to_string())
                    }
                    _ => {}
                }
            }
        }
        None
    }) else {
        result
            .edit(ctx, CreateReply::default().content("No result"))
            .await?;
        return Ok(());
    };

    let info = {
        let referer = format!("https://dic.daum.net/word/view.do?wordid={url_back}");
        let response = ctx.data().client.get(&referer).send().await?.text().await?;

        let reading = {
            let document = Html::parse_document(&response);
            document
                .select(&ctx.data().hanja.read)
                .next()
                .unwrap()
                .text()
                .collect::<String>()
        };

        let response = ctx
            .data()
            .client
            .get(format!(
                "https://dic.daum.net/word/view_supword.do?suptype=KUMSUNG_HH&wordid={url_back}"
            ))
            .header("Referer", referer)
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_fragment(&response);
        let mut description = String::new();
        let mut children = document
            .root_element()
            .child_elements()
            .flat_map(|elem| elem.child_elements());
        while let Some(child) = children.next() {
            fn extract_text(text: scraper::element_ref::Text) -> String {
                text.collect::<String>().trim().to_string()
            }

            let class = child.attr("class");
            if class == Some("wrap_ex") {
                description.push_str(&extract_text(child.text()));
                if let Some(child) = children.next() {
                    description.push_str(" ");
                    description.push_str(&extract_text(child.text()));
                }
                description.push_str("\n");
            } else if class == Some("item_example") {
                for li in child.child_elements() {
                    if let Some(ruby) = li.select(&ctx.data().hanja.ruby).next() {
                        description.push_str("> ");
                        let mut from = None;
                        let mut phrase = String::new();
                        for s in ruby.text() {
                            if s.starts_with('\u{00a0}') && s.ends_with('\u{00a0}') {
                                from = Some(s.trim());
                            } else {
                                phrase.push_str(s);
                            }
                        }
                        description.push_str(phrase.trim());
                        if let Some(example) = li.select(&ctx.data().hanja.reading).next() {
                            description.push_str("(");
                            description.push_str(&extract_text(example.text()));
                            description.push_str(")");
                        }
                        if let Some(from) = from {
                            description.push_str(" 《");
                            description.push_str(from);
                            description.push_str("》");
                        }
                        description.push_str("\n");
                    }
                }
            } else if class == Some("ex_refer") {
                description.push_str("<:rui:1363124010136764516> ");
                for refer in child.select(&ctx.data().hanja.refer) {
                    description.push_str(&extract_text(refer.text()));
                }
                description.push_str("\n");
            }
        }
        HanjaInfo {
            reading,
            description,
        }
    };
    result
        .edit(
            ctx,
            CreateReply::default().content(format!(
                "# {hanja}\n**{reading}**\n{description}",
                reading = info.reading.trim(),
                description = info.description
            )),
        )
        .await?;
    Ok(())
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping(), hanja()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("gaji ".to_string()),
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    std::time::Duration::from_secs(3600),
                ))),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    client: reqwest::Client::new(),
                    hanja: Hanja::new(),
                })
            })
        })
        .build();

    let client = Client::builder(&token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");

    Ok(client.into())
}
