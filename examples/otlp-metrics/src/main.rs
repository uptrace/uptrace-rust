use std::error::Error;
use std::thread;
use std::time::Duration;

use opentelemetry::sdk::export::metrics::aggregation::delta_temporality_selector;
use opentelemetry::sdk::metrics::controllers::BasicController;
use opentelemetry::sdk::metrics::selectors;
use opentelemetry::sdk::resource::{
    EnvResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector,
};
use opentelemetry::sdk::Resource;
use opentelemetry::{global, metrics, Context};
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use tonic::metadata::MetadataMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("using DSN: {}", dsn);

    let _ = init_metrics(dsn)?;
    let meter = global::meter("app_or_crate_name");
    let histogram = meter.f64_histogram("ex.com.three").init();

    let cx = Context::new();
    for _i in 1..100000 {
        histogram.record(&cx, 1.3, &[]);
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn init_metrics(dsn: String) -> metrics::Result<BasicController> {
    let resource = Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(SdkProvidedResourceDetector),
            Box::new(EnvResourceDetector::new()),
            Box::new(TelemetryResourceDetector),
        ],
    );

    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    let export_config = ExportConfig {
        endpoint: "https://otlp.uptrace.dev:4317".to_string(),
        timeout: Duration::from_secs(10),
        protocol: Protocol::Grpc,
    };
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_export_config(export_config)
        .with_metadata(metadata);

    opentelemetry_otlp::new_pipeline()
        .metrics(
            selectors::simple::inexpensive(),
            delta_temporality_selector(),
            opentelemetry::runtime::Tokio,
        )
        .with_exporter(exporter)
        .with_period(Duration::from_secs(15))
        .with_timeout(Duration::from_secs(5))
        .with_resource(resource)
        .build()
}
