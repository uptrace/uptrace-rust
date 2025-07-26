# Uptrace for Rust

![build workflow](https://github.com/uptrace/uptrace-rust/actions/workflows/build.yml/badge.svg)
[![Documentation](https://img.shields.io/badge/uptrace-documentation-informational)](https://uptrace.dev/get/opentelemetry-rust)
[![Chat](https://img.shields.io/badge/-telegram-red?color=white&logo=telegram&logoColor=black)](https://t.me/uptrace)

<a href="https://uptrace.dev/get/opentelemetry-rust">
  <img src="https://uptrace.dev/devicon/rust-plain.svg" height="200px" />
</a>

## Introduction

`uptrace-rust` is a lightweight wrapper around
[opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust).  
It provides a convenient way to configure OpenTelemetry for exporting
[traces](https://uptrace.dev/opentelemetry/distributed-tracing),
[logs](https://uptrace.dev/opentelemetry/logs), and
[metrics](https://uptrace.dev/opentelemetry/metrics) to
[Uptrace](https://uptrace.dev/).

> **Note:** This wrapper is currently **not production-ready**.  
> For now, please refer to the
> [official documentation](https://uptrace.dev/get/opentelemetry-rust) and the
> following examples to learn how to configure OpenTelemetry:

- [OTLP Traces Example](examples/otlp-traces)
- [OTLP Logs Example](examples/otlp-logs)
- [OTLP Metrics Example](examples/otlp-metrics)

We are actively working on a new API to simplify OpenTelemetry configuration.
Here’s a preview of what it will look like:

```rust
let uptrace = Uptrace::builder()
    .with_dsn("your_dsn_here")
    .with_traces(TracesConfig::builder().with_sampler(...).build())
    .with_logs(LogsConfig::builder().build())
    .with_metrics(MetricsConfig::builder().build())
    .build()?;

println!("Trace URL: {}", uptrace.trace_url(cx));

uptrace.force_flush()?;
uptrace.shutdown()?;
```

If you’re interested in testing or contributing,
[reach out to us on Telegram](https://t.me/uptrace).
