#![allow(clippy::single_component_path_imports)]

use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_sqs::types::Message;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use sqs_consumer::{SqsClientConfig, SqsConsumer};
use tracing;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

#[tokio::main(worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| format!("Failed to parse env filter: {e}"))?;

    let collector = tracing_subscriber::registry().with(fmt::layer()).with(env_filter);
    tracing::subscriber::set_global_default(collector)
        .map_err(|e| format!("failed to set global tracing default: {e}"))?;

    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .build()?;

    let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .build();
    opentelemetry::global::set_meter_provider(meter_provider.clone());

    let mut aws_config = aws_config::from_env();
    if let Ok(endpoint) = std::env::var("LOCALSTACK_ENDPOINT_URL") {
        tracing::info!("using local stack endpoint: {}", endpoint);
        aws_config = aws_config.endpoint_url(endpoint);
    }
    let aws_config = aws_config
        .region(RegionProviderChain::default_provider().or_else(Region::new("us-east-1")))
        .load()
        .await;
    let queue_url = std::env::var("AWS_SQS_QUEUE_URL").map_err(|_| "AWS_SQS_QUEUE_URL must be set")?;

    let config = SqsClientConfig::new(aws_config, queue_url);
    let consumer = SqsConsumer::new(process_message, config);
    consumer.start().await?;

    meter_provider
        .shutdown()
        .map_err(|e| format!("failed to shutdown meter provider: {e}"))?;
    Ok(())
}

async fn process_message(_message: Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let t = rand::random_range(3..12);

    tokio::time::sleep(std::time::Duration::from_secs(t)).await;
    tracing::info!("look mom an async function: {:?}", t);
    Ok(())
}
