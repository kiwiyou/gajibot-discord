use gajibot::command::CommandHandler;
use twilight_gateway::{Intents, ShardId};

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
        .block_on(run_bot())
}

async fn run_bot() {
    let config = gajibot::config::Config::load();
    let mut shard = twilight_gateway::Shard::new(
        ShardId::ONE,
        config.token.clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    );
    CommandHandler::new(config).run(&mut shard).await
}
