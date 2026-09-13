use gajibot::command::CommandHandler;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use twilight_gateway::{Intents, ShardId};

fn main() {
	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_time()
		.enable_io()
		.build()
		.unwrap();

	let _guard = rt.enter();
	setup_logging();
	rt.block_on(run_bot());
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

fn setup_logging() {
	let fmt_filter = EnvFilter::new("info").add_directive("opentelemetry=debug".parse().unwrap());
	let fmt_layer = tracing_subscriber::fmt::Layer::new()
		.with_thread_names(true)
		.with_filter(fmt_filter);

	let registry = tracing_subscriber::registry().with(fmt_layer);

	#[cfg(feature = "otel")]
	let registry = {
		use hyper_rustls::{ConfigBuilderExt, HttpsConnectorBuilder};
		use opentelemetry_otlp::WithHttpConfig;

		let tls_config = rustls::ClientConfig::builder()
			.try_with_platform_verifier()
			.unwrap()
			.with_no_client_auth();
		let https_config = HttpsConnectorBuilder::new()
			.with_tls_config(tls_config)
			.https_only()
			.enable_http2()
			.build();
		let http_client = opentelemetry_http::hyper::HyperClient::new(
			https_config,
			std::time::Duration::from_secs(5),
			None,
		);

		let otlp_metric_exporter = opentelemetry_otlp::MetricExporter::builder()
			.with_http()
			.with_http_client(http_client.clone())
			.build()
			.unwrap();
		let otel_metric_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
			.with_periodic_exporter(otlp_metric_exporter)
			.build();
		opentelemetry::global::set_meter_provider(otel_metric_provider);

		let otel_propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
		opentelemetry::global::set_text_map_propagator(otel_propagator);

		let otlp_span_exporter = opentelemetry_otlp::SpanExporter::builder()
			.with_http()
			.with_http_client(http_client.clone())
			.build()
			.unwrap();
		let otel_tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
			.with_batch_exporter(otlp_span_exporter)
			.build();
		opentelemetry::global::set_tracer_provider(otel_tracer_provider);

		let otlp_log_exporter = opentelemetry_otlp::LogExporter::builder()
			.with_http()
			.with_http_client(http_client)
			.build()
			.unwrap();
		let otel_log_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
			.with_log_processor(opentelemetry_sdk::logs::log_processor_with_async_runtime::BatchLogProcessor::builder(
          otlp_log_exporter,
          opentelemetry_sdk::runtime::Tokio,
      )
      .build())
			.build();

		// prevent telemetry-induced-telemetry
		// https://github.com/open-telemetry/opentelemetry-rust/issues/2877
		let otel_filter = EnvFilter::new("info")
			.add_directive("hyper=off".parse().unwrap())
			.add_directive("tonic=off".parse().unwrap())
			.add_directive("h2=off".parse().unwrap())
			.add_directive("reqwest=off".parse().unwrap());
		let otel_layer =
			opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::builder(
				&otel_log_provider,
			)
			.build()
			.with_filter(otel_filter);
		registry.with(otel_layer)
	};

	registry.init();
}
