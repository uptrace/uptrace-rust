use std::{thread, time::Duration};

use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
    Key, KeyValue,
};
use uptrace::UptraceBuilder;

#[tokio::main]
async fn main() {
    UptraceBuilder::new()
        .with_service_name("myservice")
        .with_service_version("1.0.0")
        .with_deployment_environment("testing")
        .with_disable_metrics()
        .install_simple()
        .unwrap();

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
            span.set_attribute(KeyValue::new("db.statement", "SELECT * FROM table"));
        });

        let span = cx.span();
        println!("{:?}", span.span_context().trace_id().to_string());
    });

    global::shutdown_tracer_provider();
}
