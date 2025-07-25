use std::error::Error;
use std::thread;
use std::time::Duration;

use opentelemetry::{global,  KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;
use tonic::metadata::MetadataMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("using DSN: {}", dsn);

    let provider = init_meter_provider(dsn)?;
    global::set_meter_provider(provider.clone());

    let meter = global::meter("app_or_crate_name");
    let histogram = meter.f64_histogram("ex.com.three").build();

    for _i in 1..100000 {
        histogram.record(1.3, &[]);
        thread::sleep(Duration::from_millis(100));
    }

    provider.force_flush()?;
    provider.shutdown()?;

    Ok(())
}

fn init_meter_provider(dsn: String) -> Result<SdkMeterProvider, Box<dyn Error + Send + Sync + 'static>> {
    let resource = Resource::builder()
        .with_attributes(vec![KeyValue::new("service.name", "example-service")])
        .build();

    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint("https://api.uptrace.dev:4317")
        //.with_export_config(export_config)
        .with_metadata(metadata).
        build()?;

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(resource)
        .build();

    Ok(provider)
}
