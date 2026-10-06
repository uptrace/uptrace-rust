//! OpenTelemetry Rust distribution for [Uptrace].
//!
//! This crate configures the [OpenTelemetry Rust SDK](opentelemetry_sdk) to
//! export traces, metrics, and logs to Uptrace with sensible defaults:
//! OTLP exporters with gzip compression, delta metrics temporality,
//! time-ordered trace IDs, host/OS/process resource detection, and the
//! W3C TraceContext + Baggage propagator.
//!
//! # Quickstart
//!
//! ```no_run
//! use opentelemetry::global;
//! use opentelemetry::trace::{TraceContextExt, Tracer};
//!
//! fn main() -> Result<(), uptrace::Error> {
//!     let uptrace = uptrace::Uptrace::builder()
//!         // Defaults to the UPTRACE_DSN env var.
//!         .with_dsn("https://<token>@api.uptrace.dev?grpc=4317")
//!         .with_service_name("myservice")
//!         .with_service_version("1.0.0")
//!         .with_deployment_environment("production")
//!         .build()?;
//!
//!     let tracer = global::tracer("app_or_crate_name");
//!     tracer.in_span("main", |cx| {
//!         println!("trace: {}", uptrace.trace_url(cx.span().span_context()));
//!     });
//!
//!     uptrace.shutdown()
//! }
//! ```
//!
//! # Cargo features
//!
//! - `http` (default): export using OTLP/HTTP (protobuf).
//! - `grpc`: export using OTLP/gRPC; requires a Tokio runtime. Choose the
//!   transport with [`UptraceBuilder::with_protocol`] when both are enabled.
//! - `tracing`: [`Uptrace::tracing_layer`] and [`Uptrace::log_layer`] for the
//!   [`tracing`](https://docs.rs/tracing) crate.
//! - `internal-logs` (default): report internal diagnostics via `tracing`.
//!
//! # Environment variables
//!
//! - `UPTRACE_DSN`: the DSN used when [`UptraceBuilder::with_dsn`] is not called.
//! - `UPTRACE_DISABLED`: when set, [`UptraceBuilder::build`] returns a no-op [`Uptrace`].
//! - Standard `OTEL_*` variables such as `OTEL_RESOURCE_ATTRIBUTES`,
//!   `OTEL_SERVICE_NAME`, `OTEL_TRACES_SAMPLER`, and `OTEL_BSP_*`.
//!
//! [Uptrace]: https://uptrace.dev/
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

#[cfg(not(any(feature = "http", feature = "grpc")))]
compile_error!("uptrace requires at least one transport feature: `http` or `grpc`");

mod config;
mod dsn;
mod error;
mod exporter;
mod id_generator;
mod providers;
mod resource;
#[cfg(feature = "tracing")]
mod tracing_support;
mod uptrace;

pub use config::{LogsConfig, MetricsConfig, TracesConfig};
pub use dsn::Dsn;
pub use error::Error;
pub use exporter::Protocol;
pub use id_generator::UptraceIdGenerator;
pub use uptrace::{Uptrace, UptraceBuilder};

/// Re-exported so users can depend on the exact version this crate is built with.
pub use opentelemetry;
/// Re-exported so users can depend on the exact version this crate is built with.
pub use opentelemetry_sdk;
