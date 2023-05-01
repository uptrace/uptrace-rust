# Uptrace for Rust

![build workflow](https://github.com/uptrace/uptrace-rust/actions/workflows/build.yml/badge.svg)
[![Documentation](https://img.shields.io/badge/uptrace-documentation-informational)](https://uptrace.dev/get/opentelemetry-rust.html)
[![Chat](https://img.shields.io/badge/-telegram-red?color=white&logo=telegram&logoColor=black)](https://t.me/uptrace)

<a href="https://uptrace.dev/get/opentelemetry-rust.html">
  <img src="https://uptrace.dev/get/devicon/rust-plain.svg" height="200px" />
</a>

## Introduction

uptrace-rust is an OpenTelemery Rust distribution configured to export
[traces](https://uptrace.dev/opentelemetry/distributed-tracing.html) and
[metrics](https://uptrace.dev/opentelemetry/metrics.html) to [Uptrace](https://uptrace.dev/).

## Quickstart

Install uptrace-rust:

```bash
cargo add uptrace
```

Run the [basic example](examples/basic.rs) below using the DSN from the Uptrace project settings page.

```shell
UPTRACE_DSN=http://project2_secret_token@localhost:14317/2 cargo run --example basic
```

```rust
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
        //.with_dsn("")
        .with_service_name("myservice")
        .with_service_version("1.0.0")
        .with_deployment_environment("testing")
        .configure_opentelemetry()
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
            span.set_attribute(KeyValue::new(
                "db.statement",
                "SELECT * FROM table LIMIT 100",
            ));
        });

        let span = cx.span();
        println!(
            "https://app.uptrace.dev/traces/{}",
            span.span_context().trace_id().to_string()
        );
    });

    global::shutdown_tracer_provider();
}
```
