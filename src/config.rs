use std::env::var;

pub struct Config {
    pub bot_token: String,
    pub emoji: Emoji,
}

impl Config {
    pub fn load() -> Self {
        Self {
            bot_token: var("BOT_TOKEN").unwrap(),
            emoji: Emoji {
                idk: var("BOT_EMOJI_IDK").ok(),
                loading: var("BOT_EMOJI_LOADING").ok(),
            },
        }
    }
}

pub struct Emoji {
    pub idk: Option<String>,
    pub loading: Option<String>,
}
