use std::env::var;

pub struct Config {
    pub token: String,
    pub emoji: Emoji,
    pub url_maybe: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        Self {
            token: var("BOT_TOKEN").unwrap(),
            emoji: Emoji {
                idk: var("BOT_EMOJI_IDK").ok(),
                loading: var("BOT_EMOJI_LOADING").ok(),
            },
            url_maybe: var("BOT_URL_MAYBE").ok(),
        }
    }
}

pub struct Emoji {
    pub idk: Option<String>,
    pub loading: Option<String>,
}
