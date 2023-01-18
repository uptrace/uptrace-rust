use std::{thread, time::Duration};

use opentelemetry::{
    global,
    trace::{Span, TraceContextExt, Tracer},
    Key, KeyValue,
};
use uptrace::UptraceBuilder;

fn foo() {
    let tracer = global::tracer("component-foo");
    let mut span = tracer.start("foo");
    span.set_attribute(Key::new("span.type").string("http"));
    span.set_attribute(Key::new("sql.query").string("http://www.baidu.com/"));
    thread::sleep(Duration::from_millis(6));
    span.end()
}

fn bar() {
    let tracer = global::tracer("component-bar");
    let mut span = tracer.start("bar");
    span.set_attribute(Key::new("span.type").string("sql"));
    span.set_attribute(Key::new("sql.query").string("SELECT * FROM table"));
    thread::sleep(Duration::from_millis(6));
    foo();
    span.end()
}

#[tokio::main]
async fn main() {
    UptraceBuilder::new()
        .with_dns("http://project2_secret_token@localhost:14317/2")
        .with_service_name("lol")
        .with_disable_metrics()
        .install_simple()
        .unwrap();

    let tracer = global::tracer("rust-service");
    let mut span = tracer.start("my_span");
    span.set_attribute(KeyValue::new("http.client_ip", "83.164.160.102"));
    span.set_attribute(KeyValue::new("now", 1));
    span.end();

    tracer.in_span("foo", |cx| {
        let span = cx.span();
        span.set_attribute(Key::new("span.type").string("web"));
        span.set_attribute(Key::new("http.url").string("http://localhost:8080/foo"));
        span.set_attribute(Key::new("http.method").string("GET"));
        span.set_attribute(Key::new("http.status_code").i64(200));

        thread::sleep(Duration::from_millis(6));
        bar();
        thread::sleep(Duration::from_millis(6));
        println!("{:?}", span.span_context().trace_id().to_string());

        foo();
    });

    thread::sleep(Duration::from_secs(150));

    println!("{:?}", span.span_context().trace_id().to_string());
    global::shutdown_tracer_provider();
}
