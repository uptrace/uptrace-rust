use std::thread;
use std::time::Duration;

use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, KeyValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads the DSN from the UPTRACE_DSN env var (format: https://uptrace.dev/get#dsn).
    let uptrace = uptrace::Uptrace::builder()
        .with_service_name("myservice")
        .with_service_version("1.0.0")
        .with_deployment_environment("production")
        .build()?;

    let tracer = global::tracer("app_or_crate_name");
    let meter = global::meter("app_or_crate_name");
    let requests = meter.u64_counter("app.requests").build();

    tracer.in_span("main-operation", |cx| {
        tracer.in_span("GET /posts/:id", |cx| {
            thread::sleep(Duration::from_millis(10));

            let span = cx.span();
            span.set_attribute(KeyValue::new("http.request.method", "GET"));
            span.set_attribute(KeyValue::new("http.route", "/posts/:id"));
            span.set_attribute(KeyValue::new("http.response.status_code", 200));
            requests.add(1, &[KeyValue::new("http.route", "/posts/:id")]);
        });

        tracer.in_span("SELECT", |cx| {
            thread::sleep(Duration::from_millis(20));

            let span = cx.span();
            span.set_attribute(KeyValue::new("db.system.name", "mysql"));
            span.set_attribute(KeyValue::new(
                "db.query.text",
                "SELECT * FROM posts LIMIT 100",
            ));
        });

        // Records the error on the current span.
        uptrace.report_error(&std::io::Error::other("something went wrong"));

        println!("View trace: {}", uptrace.trace_url(cx.span().span_context()));
    });

    // Export buffered data before exiting.
    uptrace.shutdown()?;
    Ok(())
}
