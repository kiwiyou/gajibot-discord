use std::sync::Arc;

use tracing::{event, info, Level};
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

	#[tracing::instrument(skip(self, shard), fields(discord.shard = shard.id().number()))]
	pub async fn run(&self, shard: &mut twilight_gateway::Shard) {
		info!("begin handling event");
		while let Some(Ok(event)) = shard.next_event(EventTypeFlags::MESSAGE_CREATE).await {
			event!(
				Level::INFO,
				event.type = event.kind().name(),
				event.guild.id = event.guild_id().map(|id| id.get()),
				"event received"
			);
			match event {
				Event::GatewayClose(_) => break,
				Event::MessageCreate(event) => {
					event!(
						Level::INFO,
						event.channel.id = event.channel_id.get(),
						event.message.id = event.id.get(),
						"message received"
					);
					if let Some(command) = self.extract_daum(&event) {
						tokio::spawn(command.handle());
					}
				}
				_ => {}
			}
		}
	}
}
