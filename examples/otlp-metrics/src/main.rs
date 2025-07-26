use std::time::Duration;

use tonic::metadata::MetadataMap;

use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_resource_detectors::{
    HostResourceDetector, OsResourceDetector, ProcessResourceDetector,
};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
use opentelemetry_sdk::Resource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Read Uptrace DSN from environment (format: https://uptrace.dev/get#dsn)
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("Using DSN: {}", dsn);

    // Initialize the OpenTelemetry MeterProvider
    let provider = init_meter_provider(dsn)?;
    global::set_meter_provider(provider.clone());

    // Create a meter and a histogram instrument
    let meter = global::meter("app_or_crate_name");
    let histogram = meter.f64_histogram("ex.com.three").build();

    // Record some sample metrics
    for i in 1..100000 {
        histogram.record(0.5 + (i as f64) * 0.01, &[]);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Flush and shutdown the provider to ensure all data is exported
    provider.force_flush()?;
    provider.shutdown()?;

    Ok(())
}

fn init_meter_provider(
    dsn: String,
) -> Result<SdkMeterProvider, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Configure gRPC metadata with Uptrace DSN
    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    // Create OTLP metric exporter
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint("https://api.uptrace.dev:4317")
        .with_metadata(metadata)
        .with_temporality(Temporality::Delta)
        .build()?;

    // Create periodic reader for exporting metrics
    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(15))
        .build();

    // Build the MeterProvider with reader
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(build_resource())
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
