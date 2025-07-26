use tonic::metadata::MetadataMap;

use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_resource_detectors::{
    HostResourceDetector, OsResourceDetector, ProcessResourceDetector,
};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::Resource;

use tracing::error;
use tracing_subscriber::{prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Read Uptrace DSN from environment (format: https://uptrace.dev/get#dsn)
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("Using DSN: {}", dsn);

    // Initialize the OpenTelemetry LoggerProvider
    let provider = init_logger_provider(dsn)?;

    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());
    let otel_layer = layer::OpenTelemetryTracingBridge::new(&provider).with_filter(filter_otel);

    // Create a tracing::Fmt layer to print logs to stdout
    // Default filter is `info` level and above, with `debug` and above for OpenTelemetry crates
    let filter_fmt = EnvFilter::new("info").add_directive("opentelemetry=debug".parse().unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(filter_fmt);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    // Emit a test log event (this will be exported to Uptrace)
    error!(
        name: "my-event-name",
        target: "my-system",
        event_id = 20,
        user_name = "otel",
        user_email = "otel@opentelemetry.io",
        message = "This is an example message"
    );

    // Flush and shutdown the provider to ensure all data is exported
    provider.force_flush()?;
    provider.shutdown()?;

    Ok(())
}

fn init_logger_provider(
    dsn: String,
) -> Result<SdkLoggerProvider, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Configure gRPC metadata with Uptrace DSN
    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    // Configure the OTLP log exporter (gRPC + TLS)
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint("https://api.uptrace.dev:4317")
        .with_metadata(metadata)
        .build()?;

    // Build the logger provider with resource attributes
    let provider = SdkLoggerProvider::builder()
        .with_resource(build_resource())
        .with_batch_exporter(exporter)
        .build();

    Ok(provider)
}

fn build_resource() -> Resource {
    Resource::builder()
        .with_detector(Box::new(OsResourceDetector))
        .with_detector(Box::new(HostResourceDetector::default()))
        .with_detector(Box::new(ProcessResourceDetector))
        .with_attributes([
            KeyValue::new("service.version", "1.2.3"),
            KeyValue::new("deployment.environment", "production"),
        ])
        .build()
}
