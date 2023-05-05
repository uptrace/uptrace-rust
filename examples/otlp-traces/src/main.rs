use std::error::Error;
use std::thread;
use std::time::Duration;

use opentelemetry::sdk::resource::{
    EnvResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector,
};
use opentelemetry::sdk::Resource;
use opentelemetry::trace::TraceError;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, sdk, Key, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tonic::metadata::MetadataMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("using DSN: {}", dsn);

    let _ = init_tracer(dsn)?;
    let tracer = global::tracer("app_or_crate_name");

    tracer.in_span("root-span", |cx| {
        thread::sleep(Duration::from_millis(5));

        tracer.in_span("GET /posts/:id", |cx| {
            thread::sleep(Duration::from_millis(10));

            let span = cx.span();
            span.set_attribute(Key::new("http.method").string("GET"));
            span.set_attribute(Key::new("http.route").string("/posts/:id"));
            span.set_attribute(Key::new("http.url").string("http://localhost:8080/posts/123"));
            span.set_attribute(Key::new("http.status_code").i64(200));
        });

        tracer.in_span("SELECT", |cx| {
            thread::sleep(Duration::from_millis(20));

            let span = cx.span();
            span.set_attribute(KeyValue::new("db.system", "mysql"));
            span.set_attribute(KeyValue::new(
                "db.statement",
                "SELECT * FROM posts LIMIT 100",
            ));
        });

        let span = cx.span();
        println!(
            "https://app.uptrace.dev/traces/{}",
            span.span_context().trace_id().to_string()
        );
    });

    global::shutdown_tracer_provider();
    Ok(())
}

fn init_tracer(dsn: String) -> Result<sdk::trace::Tracer, TraceError> {
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

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("https://otlp.uptrace.dev:4317")
                .with_timeout(Duration::from_secs(5))
                .with_metadata(metadata),
        )
        .with_batch_config(
            sdk::trace::BatchConfig::default()
                .with_max_queue_size(30000)
                .with_max_export_batch_size(10000)
                .with_scheduled_delay(Duration::from_millis(5000)),
        )
        .with_trace_config(sdk::trace::config().with_resource(resource))
        .install_batch(opentelemetry::runtime::Tokio)
}
