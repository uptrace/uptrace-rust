use opentelemetry::trace::TracerProvider as _;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
use opentelemetry_sdk::trace::SdkTracer;
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;

use crate::Uptrace;

impl Uptrace {
    /// Returns a [`tracing_subscriber::Layer`] that exports `tracing` spans to Uptrace.
    ///
    /// Returns `None` when tracing is disabled; `Option<Layer>` is itself a layer,
    /// so the result can be passed to `.with(...)` unconditionally.
    pub fn tracing_layer<S>(&self) -> Option<OpenTelemetryLayer<S, SdkTracer>>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let tracer = self.tracer_provider()?.tracer(env!("CARGO_PKG_NAME"));
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    }

    /// Returns a [`tracing_subscriber::Layer`] that exports `tracing` events to
    /// Uptrace as logs.
    ///
    /// Returns `None` when logs are disabled. To avoid exporting the exporter's
    /// own logs, filter out `hyper`, `h2`, `tonic`, and `reqwest` events:
    ///
    /// ```no_run
    /// use tracing_subscriber::{prelude::*, EnvFilter};
    ///
    /// # fn main() -> Result<(), uptrace::Error> {
    /// let uptrace = uptrace::Uptrace::builder().build()?;
    /// let filter = EnvFilter::new("info")
    ///     .add_directive("hyper=off".parse().unwrap())
    ///     .add_directive("h2=off".parse().unwrap())
    ///     .add_directive("tonic=off".parse().unwrap())
    ///     .add_directive("reqwest=off".parse().unwrap());
    /// tracing_subscriber::registry()
    ///     .with(uptrace.tracing_layer())
    ///     .with(uptrace.log_layer().with_filter(filter))
    ///     .init();
    /// # Ok(())
    /// # }
    /// ```
    pub fn log_layer(&self) -> Option<OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>> {
        self.logger_provider().map(OpenTelemetryTracingBridge::new)
    }
}
