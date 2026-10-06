use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, Tracer};

// The gRPC exporter runs on Tokio, so build Uptrace inside the runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads the DSN from the UPTRACE_DSN env var (format: https://uptrace.dev/get#dsn).
    let uptrace = uptrace::Uptrace::builder()
        .with_service_name("myservice")
        .build()?;

    let tracer = global::tracer("app_or_crate_name");
    tracer.in_span("main-operation", |cx| {
        println!("View trace: {}", uptrace.trace_url(cx.span().span_context()));
    });

    // Shutdown blocks while flushing, so run it off the async executor.
    tokio::task::spawn_blocking(move || uptrace.shutdown()).await??;
    Ok(())
}
