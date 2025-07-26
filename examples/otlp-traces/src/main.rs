use std::thread;
use std::time::Duration;

use tonic::metadata::MetadataMap;

use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_resource_detectors::{
    HostResourceDetector, OsResourceDetector, ProcessResourceDetector,
};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{
        BatchConfigBuilder, BatchSpanProcessor, RandomIdGenerator, Sampler, SdkTracerProvider,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Read Uptrace DSN from environment (format: https://uptrace.dev/get#dsn)
    let dsn = std::env::var("UPTRACE_DSN").expect("Error: UPTRACE_DSN not found");
    println!("Using DSN: {}", dsn);

    let provider = build_tracer_provider(dsn)?;
    global::set_tracer_provider(provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());

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
            "View trace: https://app.uptrace.dev/traces/{}",
            span.span_context().trace_id().to_string()
        );
    });

    // Flush and shutdown the provider to ensure all data is exported
    provider.force_flush()?;
    provider.shutdown()?;

    Ok(())
}

fn build_tracer_provider(
    dsn: String,
) -> Result<SdkTracerProvider, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Configure gRPC metadata with Uptrace DSN
    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    // Create OTLP span exporter
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint("https://api.uptrace.dev:4317")
        .with_metadata(metadata)
        .with_timeout(Duration::from_secs(10))
        .build()?;

    let batch_config = BatchConfigBuilder::default()
        .with_max_queue_size(4096)
        .with_max_export_batch_size(1024)
        .with_scheduled_delay(Duration::from_secs(5))
        .build();
    let batch = BatchSpanProcessor::builder(exporter)
        .with_batch_config(batch_config)
        .build();

    // Build the tracer provider
    let provider = SdkTracerProvider::builder()
        .with_span_processor(batch)
        .with_resource(build_resource())
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
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
