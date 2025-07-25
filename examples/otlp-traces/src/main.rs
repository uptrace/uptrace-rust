use std::error::Error;
use std::time::Duration;
use std::thread;

use tonic::metadata::MetadataMap;

use opentelemetry::{global, trace::Tracer, trace::TraceContextExt, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{
    trace::{SdkTracerProvider},
    Resource,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("using DSN: {}", dsn);

    let provider = build_tracer_provider(dsn)?;
    global::set_tracer_provider(provider.clone());

    let tracer = global::tracer("example");

    tracer.in_span("root-span", |cx| {
        thread::sleep(Duration::from_millis(5));

        tracer.in_span("GET /posts/:id", |cx| {
            thread::sleep(Duration::from_millis(10));

            let span = cx.span();
            span.set_attribute(KeyValue::new("http.method", "GET"));
            span.set_attribute(KeyValue::new("http.route", "/posts/:id"));
            span.set_attribute(KeyValue::new("http.url", "http://localhost:8080/posts/123"));
            span.set_attribute(KeyValue::new("http.status_code", 200));
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

    // Flush and shut down
    provider.force_flush()?;
    provider.shutdown()?;

    Ok(())
}

fn build_tracer_provider(dsn: String) -> Result<SdkTracerProvider, Box<dyn Error + Send + Sync + 'static>> {
    let resource = Resource::builder()
        .with_attributes(vec![KeyValue::new("service.name", "example-service")])
        .build();

    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint("https://api.uptrace.dev:4317")
        .with_metadata(metadata)
        .build()?;

    // Assemble the tracer provider
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    Ok(provider)
}
