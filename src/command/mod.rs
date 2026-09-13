use std::sync::Arc;

use twilight_gateway::{Event, EventTypeFlags, StreamExt};

use crate::{config::Config, integration::DaumClient};

mod daum;

pub struct CommandHandler {
	config: Arc<Config>,
	client: Arc<twilight_http::Client>,
	daum_client: Arc<DaumClient>,
}

impl CommandHandler {
	/// Should be called inside tokio runtime
	pub fn new(config: Config) -> Self {
		let token = config.token.clone();
		Self {
			config: Arc::new(config),
			client: Arc::new(twilight_http::Client::new(token)),
			daum_client: Arc::new(DaumClient::default()),
		}
	}

	pub async fn run(&self, shard: &mut twilight_gateway::Shard) {
		while let Some(Ok(event)) = shard.next_event(EventTypeFlags::MESSAGE_CREATE).await {
			match event {
				Event::GatewayClose(_) => break,
				Event::MessageCreate(event) => {
					if let Some(command) = self.extract_daum(&event) {
						tokio::spawn(command.handle());
					}
				}
				_ => {}
			}
		}
	}
}
