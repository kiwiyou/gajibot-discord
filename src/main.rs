fn main() {
    let config = gajibot::Config::load();

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
        .block_on(async { gajibot::serenity_client(&config).await.start().await })
        .unwrap();
}
