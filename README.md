# Uptrace for Rust

![build workflow](https://github.com/uptrace/uptrace-rust/actions/workflows/build.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/uptrace.svg)](https://crates.io/crates/uptrace)
[![docs.rs](https://img.shields.io/docsrs/uptrace)](https://docs.rs/uptrace)
[![Documentation](https://img.shields.io/badge/uptrace-documentation-informational)](https://uptrace.dev/get/opentelemetry-rust)
[![Chat](https://img.shields.io/badge/-telegram-red?color=white&logo=telegram&logoColor=black)](https://t.me/uptrace)

<a href="https://uptrace.dev/get/opentelemetry-rust">
  <img src="https://uptrace.dev/devicon/rust-plain.svg" height="200px" />
</a>

## Introduction

`uptrace` is an [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
distribution that configures OpenTelemetry to export
[traces](https://uptrace.dev/opentelemetry/distributed-tracing),
[logs](https://uptrace.dev/opentelemetry/logs), and
[metrics](https://uptrace.dev/opentelemetry/metrics) to [Uptrace](https://uptrace.dev/).

It sets up OTLP exporters with gzip compression, batching tuned for Uptrace, delta metrics
temporality, time-ordered trace IDs, host/OS/process resource detection, and the W3C TraceContext +
Baggage propagator.

## Quickstart

```shell
cargo add uptrace opentelemetry
```

```rust
use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, Tracer};

fn main() -> Result<(), uptrace::Error> {
    let uptrace = uptrace::Uptrace::builder()
        // Defaults to the UPTRACE_DSN env var.
        .with_dsn("https://<project_secret>@api.uptrace.dev?grpc=4317")
        .with_service_name("myservice")
        .with_service_version("1.0.0")
        .with_deployment_environment("production")
        .build()?;

    let tracer = global::tracer("app_or_crate_name");
    tracer.in_span("main", |cx| {
        println!("View trace: {}", uptrace.trace_url(cx.span().span_context()));
    });

    // Export buffered data before exiting.
    uptrace.shutdown()
}
```

Traces and metrics are registered globally (`opentelemetry::global`). OpenTelemetry Rust has no
global logger provider, so logs are sent through a bridge: use `uptrace.log_layer()` with the
`tracing` feature, or pass `uptrace.logger_provider()` to any OpenTelemetry log appender.

## Configuration

```rust
use std::time::Duration;
use opentelemetry_sdk::trace::Sampler;
use uptrace::{MetricsConfig, TracesConfig};

let uptrace = uptrace::Uptrace::builder()
    .with_traces(TracesConfig::default().with_sampler(Sampler::ParentBased(Box::new(
        Sampler::TraceIdRatioBased(0.1),
    ))))
    .with_metrics(MetricsConfig::default().with_interval(Duration::from_secs(30)))
    .without_logs()
    .build()?;
```

| Builder method                                  | Description                                                |
| ----------------------------------------------- | ---------------------------------------------------------- |
| `with_dsn`                                      | Uptrace DSN; defaults to `UPTRACE_DSN`.                    |
| `with_service_name`, `with_service_version`     | `service.name` and `service.version` resource attributes.  |
| `with_deployment_environment`                   | `deployment.environment` resource attribute.               |
| `with_resource_attributes`, `with_resource_detector`, `with_resource` | Customize the resource.          |
| `with_traces`, `with_metrics`, `with_logs`      | Configure each signal.                                     |
| `without_traces`, `without_metrics`, `without_logs` | Disable a signal.                                      |
| `with_protocol`                                 | `Protocol::Http` or `Protocol::Grpc`.                      |
| `with_propagator`                               | Global propagator; defaults to TraceContext + Baggage.     |
| `with_global(false)`                            | Don't register providers globally.                         |
| `with_panic_hook`                               | Record panics as span exceptions.                          |

The `Uptrace` handle provides `trace_url`, `current_trace_url`, `report_error`, `force_flush`, and
`shutdown`.

### Cargo features

| Feature                   | Description                                                            |
| ------------------------- | ---------------------------------------------------------------------- |
| `http` (default)          | Export using OTLP/HTTP (protobuf). No async runtime required.          |
| `grpc`                    | Export using OTLP/gRPC (tonic). Requires a Tokio runtime.              |
| `tracing`                 | `tracing_layer()` and `log_layer()` for the `tracing` crate.           |
| `internal-logs` (default) | Report internal diagnostics via `tracing`.                             |

To use gRPC only: `uptrace = { version = "0.33", default-features = false, features = ["grpc"] }`.

### Environment variables

| Variable                   | Description                                              |
| -------------------------- | -------------------------------------------------------- |
| `UPTRACE_DSN`              | DSN used when `with_dsn` is not called.                  |
| `UPTRACE_DISABLED`         | When set, `build()` returns a no-op `Uptrace`.           |
| `OTEL_RESOURCE_ATTRIBUTES` | Extra resource attributes, e.g. `service.name=myservice`. |
| `OTEL_SERVICE_NAME`        | Service name.                                            |
| `OTEL_TRACES_SAMPLER`      | Trace sampler, e.g. `parentbased_traceidratio`.          |
| `OTEL_BSP_*`, `OTEL_BLRP_*`| Batch span/log processor settings.                       |

## Examples

- [Basic](examples/basic): traces and metrics.
- [tracing](examples/tracing): spans and logs from the `tracing` crate.
- [gRPC](examples/grpc): OTLP/gRPC transport.

Examples that configure OpenTelemetry directly, without this crate:

- [OTLP Traces](examples/otlp-traces)
- [OTLP Logs](examples/otlp-logs)
- [OTLP Metrics](examples/otlp-metrics)
