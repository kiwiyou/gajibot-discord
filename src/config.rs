use std::env::var;

pub struct Config {
	pub token: String,
	pub daum: Daum,
}

impl Config {
	pub fn load() -> Self {
		Self {
			token: var("TOKEN").unwrap(),
			daum: Daum {
				idk_emoji: var("DAUM_IDK_EMOJI").ok(),
				loading_emoji: var("DAUM_LOADING_EMOJI").ok(),
				maybe_url: var("DAUM_MAYBE_URL").ok(),
			},
		}
	}
}

pub struct Daum {
	pub idk_emoji: Option<String>,
	pub loading_emoji: Option<String>,
	pub maybe_url: Option<String>,
}
