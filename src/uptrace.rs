use std::backtrace::Backtrace;
use std::fmt;
use std::panic::PanicHookInfo;

use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::trace::{SpanContext, Status, TraceContextExt, Tracer, TracerProvider as _};
use opentelemetry::{global, Context, KeyValue};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::resource::ResourceDetector;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::attribute::{
    EXCEPTION_MESSAGE, EXCEPTION_STACKTRACE, EXCEPTION_TYPE,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};

use crate::config::env_is_set;
use crate::exporter::Protocol;
use crate::{providers, resource, Dsn, Error, LogsConfig, MetricsConfig, TracesConfig};

const TRACER_NAME: &str = "uptrace-rust";
const DUMMY_SPAN_NAME: &str = "__dummy__";
const FALLBACK_DSN: &str = "https://<token>@api.uptrace.dev";

/// Builder for [`Uptrace`]. Created with [`Uptrace::builder`].
///
/// Traces, metrics, and logs are all enabled by default.
pub struct UptraceBuilder {
    dsn: Option<String>,
    protocol: Protocol,

    resource: Option<Resource>,
    resource_attributes: Vec<KeyValue>,
    resource_detectors: Vec<Box<dyn ResourceDetector + Send + Sync>>,

    traces: Option<TracesConfig>,
    metrics: Option<MetricsConfig>,
    logs: Option<LogsConfig>,

    install_global: bool,
    install_propagator: Box<dyn FnOnce() + Send>,
    panic_hook: bool,
}

impl Default for UptraceBuilder {
    fn default() -> Self {
        Self {
            dsn: None,
            protocol: Protocol::default(),

            resource: None,
            resource_attributes: Vec::new(),
            resource_detectors: Vec::new(),

            traces: Some(TracesConfig::default()),
            metrics: Some(MetricsConfig::default()),
            logs: Some(LogsConfig::default()),

            install_global: true,
            install_propagator: Box::new(|| {
                global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
                    Box::new(TraceContextPropagator::new()),
                    Box::new(BaggagePropagator::new()),
                ]))
            }),
            panic_hook: false,
        }
    }
}

impl UptraceBuilder {
    /// Sets the DSN used to connect to Uptrace, for example,
    /// `https://<token>@api.uptrace.dev?grpc=4317`.
    ///
    /// The default is the `UPTRACE_DSN` env var.
    pub fn with_dsn(mut self, dsn: impl Into<String>) -> Self {
        self.dsn = Some(dsn.into());
        self
    }

    /// Selects the OTLP transport when both `http` and `grpc` features are enabled.
    /// The default is [`Protocol::Http`] when available.
    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        self
    }

    /// Sets the `service.name` resource attribute.
    pub fn with_service_name(self, name: impl Into<String>) -> Self {
        self.with_resource_attributes([KeyValue::new(SERVICE_NAME, name.into())])
    }

    /// Sets the `service.version` resource attribute, for example, `1.0.0`.
    pub fn with_service_version(self, version: impl Into<String>) -> Self {
        self.with_resource_attributes([KeyValue::new(SERVICE_VERSION, version.into())])
    }

    /// Sets the `deployment.environment` resource attribute, for example, `production`.
    pub fn with_deployment_environment(self, env: impl Into<String>) -> Self {
        self.with_resource_attributes([KeyValue::new("deployment.environment", env.into())])
    }

    /// Adds resource attributes that describe the entity producing telemetry.
    ///
    /// They take precedence over detected attributes and `OTEL_RESOURCE_ATTRIBUTES`.
    pub fn with_resource_attributes(mut self, attrs: impl IntoIterator<Item = KeyValue>) -> Self {
        self.resource_attributes.extend(attrs);
        self
    }

    /// Adds a resource detector, e.g. one from `opentelemetry-resource-detectors`.
    ///
    /// Host, OS, process, and env (`OTEL_RESOURCE_ATTRIBUTES`) detectors are
    /// always enabled.
    pub fn with_resource_detector<D>(mut self, detector: D) -> Self
    where
        D: ResourceDetector + Send + Sync + 'static,
    {
        self.resource_detectors.push(Box::new(detector));
        self
    }

    /// Uses the given resource as-is, ignoring resource attributes and
    /// detectors configured with other methods.
    pub fn with_resource(mut self, resource: Resource) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Configures tracing.
    pub fn with_traces(mut self, config: TracesConfig) -> Self {
        self.traces = Some(config);
        self
    }

    /// Disables tracing.
    pub fn without_traces(mut self) -> Self {
        self.traces = None;
        self
    }

    /// Configures metrics.
    pub fn with_metrics(mut self, config: MetricsConfig) -> Self {
        self.metrics = Some(config);
        self
    }

    /// Disables metrics.
    pub fn without_metrics(mut self) -> Self {
        self.metrics = None;
        self
    }

    /// Configures logs.
    pub fn with_logs(mut self, config: LogsConfig) -> Self {
        self.logs = Some(config);
        self
    }

    /// Disables logs.
    pub fn without_logs(mut self) -> Self {
        self.logs = None;
        self
    }

    /// Sets the global text map propagator. The default is W3C TraceContext + Baggage.
    pub fn with_propagator<P>(mut self, propagator: P) -> Self
    where
        P: TextMapPropagator + Send + Sync + 'static,
    {
        self.install_propagator = Box::new(move || global::set_text_map_propagator(propagator));
        self
    }

    /// Controls whether the tracer provider, meter provider, and propagator
    /// are registered globally (`opentelemetry::global`). The default is `true`.
    pub fn with_global(mut self, install: bool) -> Self {
        self.install_global = install;
        self
    }

    /// Installs a panic hook that records panics as `exception` span events and
    /// flushes spans before the previous hook runs. Requires tracing to be enabled.
    pub fn with_panic_hook(mut self) -> Self {
        self.panic_hook = true;
        self
    }

    /// Configures OpenTelemetry to export data to Uptrace.
    ///
    /// Returns a disabled (no-op) [`Uptrace`] when the `UPTRACE_DISABLED` env var
    /// is set or the DSN is a `<token>` placeholder.
    ///
    /// # Errors
    ///
    /// Returns an error when the DSN is missing or invalid, or an exporter
    /// can't be created.
    pub fn build(self) -> Result<Uptrace, Error> {
        if env_is_set("UPTRACE_DISABLED") {
            opentelemetry::otel_warn!(
                name: "Uptrace.Disabled",
                message = "UPTRACE_DISABLED is set: Uptrace is disabled"
            );
            return Ok(Uptrace::disabled(fallback_dsn()));
        }

        let dsn = match self.dsn {
            Some(dsn) => dsn,
            None => std::env::var("UPTRACE_DSN").unwrap_or_default(),
        };
        let dsn = Dsn::parse(&dsn)?;
        if dsn.is_dummy() {
            opentelemetry::otel_warn!(
                name: "Uptrace.Disabled",
                message = "dummy Uptrace DSN detected: Uptrace is disabled"
            );
            return Ok(Uptrace::disabled(dsn));
        }

        let resource = self.resource.unwrap_or_else(|| {
            resource::build_resource(self.resource_detectors, self.resource_attributes)
        });

        let tracer_provider = self
            .traces
            .map(|c| providers::tracer_provider(&dsn, self.protocol, resource.clone(), c))
            .transpose()?;
        let meter_provider = self
            .metrics
            .map(|c| providers::meter_provider(&dsn, self.protocol, resource.clone(), c))
            .transpose()?;
        let logger_provider = self
            .logs
            .map(|c| providers::logger_provider(&dsn, self.protocol, resource, c))
            .transpose()?;

        if self.install_global {
            if let Some(provider) = &tracer_provider {
                global::set_tracer_provider(provider.clone());
            }
            if let Some(provider) = &meter_provider {
                global::set_meter_provider(provider.clone());
            }
            (self.install_propagator)();
        }
        if self.panic_hook {
            if let Some(provider) = &tracer_provider {
                install_panic_hook(provider.clone());
            }
        }

        Ok(Uptrace {
            dsn,
            tracer_provider,
            meter_provider,
            logger_provider,
        })
    }
}

impl fmt::Debug for UptraceBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UptraceBuilder")
            .field("protocol", &self.protocol)
            .field("resource_attributes", &self.resource_attributes)
            .field("traces", &self.traces)
            .field("metrics", &self.metrics)
            .field("logs", &self.logs)
            .field("install_global", &self.install_global)
            .field("panic_hook", &self.panic_hook)
            .finish_non_exhaustive()
    }
}

/// Configured OpenTelemetry providers that export data to Uptrace.
///
/// Call [`Uptrace::shutdown`] before the program exits to flush buffered
/// telemetry. Cloning is cheap; clones share the same providers.
#[derive(Clone, Debug)]
#[must_use = "call `shutdown` before exiting to flush buffered telemetry"]
pub struct Uptrace {
    dsn: Dsn,
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
    logger_provider: Option<SdkLoggerProvider>,
}

impl Uptrace {
    /// Returns a builder to configure OpenTelemetry for Uptrace.
    pub fn builder() -> UptraceBuilder {
        UptraceBuilder::default()
    }

    fn disabled(dsn: Dsn) -> Self {
        Self {
            dsn,
            tracer_provider: None,
            meter_provider: None,
            logger_provider: None,
        }
    }

    /// The DSN in use.
    pub fn dsn(&self) -> &Dsn {
        &self.dsn
    }

    /// Whether any telemetry is exported to Uptrace.
    pub fn is_enabled(&self) -> bool {
        self.tracer_provider.is_some()
            || self.meter_provider.is_some()
            || self.logger_provider.is_some()
    }

    /// The tracer provider, unless tracing is disabled.
    pub fn tracer_provider(&self) -> Option<&SdkTracerProvider> {
        self.tracer_provider.as_ref()
    }

    /// The meter provider, unless metrics are disabled.
    pub fn meter_provider(&self) -> Option<&SdkMeterProvider> {
        self.meter_provider.as_ref()
    }

    /// The logger provider, unless logs are disabled.
    ///
    /// OpenTelemetry Rust has no global logger provider: pass it to a log bridge
    /// such as `opentelemetry-appender-tracing` or use [`Uptrace::log_layer`]
    /// (the `tracing` feature).
    pub fn logger_provider(&self) -> Option<&SdkLoggerProvider> {
        self.logger_provider.as_ref()
    }

    /// Returns the URL of the trace in the Uptrace UI.
    pub fn trace_url(&self, span_context: &SpanContext) -> String {
        format!(
            "{}/traces/{}?span_id={}",
            self.dsn.site_url(),
            span_context.trace_id(),
            span_context.span_id()
        )
    }

    /// Returns the URL of the current span's trace in the Uptrace UI.
    pub fn current_trace_url(&self) -> String {
        let cx = Context::current();
        self.trace_url(cx.span().span_context())
    }

    /// Records an error as an `exception` event on the current span, or on a
    /// new `__dummy__` span when there is no active recording span.
    pub fn report_error(&self, err: &dyn std::error::Error) {
        let cx = Context::current();
        let span = cx.span();
        if span.is_recording() {
            span.record_error(err);
            return;
        }

        if let Some(provider) = &self.tracer_provider {
            let tracer = provider.tracer(TRACER_NAME);
            let mut span = tracer.start(DUMMY_SPAN_NAME);
            opentelemetry::trace::Span::record_error(&mut span, err);
        }
    }

    /// Exports all buffered telemetry.
    pub fn force_flush(&self) -> Result<(), Error> {
        collect([
            self.tracer_provider.as_ref().map(|p| p.force_flush()),
            self.meter_provider.as_ref().map(|p| p.force_flush()),
            self.logger_provider.as_ref().map(|p| p.force_flush()),
        ])
    }

    /// Flushes buffered telemetry and shuts down all providers.
    ///
    /// Providers can't be used after shutdown.
    pub fn shutdown(&self) -> Result<(), Error> {
        collect([
            self.tracer_provider.as_ref().map(|p| p.shutdown()),
            self.meter_provider.as_ref().map(|p| p.shutdown()),
            self.logger_provider.as_ref().map(|p| p.shutdown()),
        ])
    }
}

fn collect(results: [Option<OTelSdkResult>; 3]) -> Result<(), Error> {
    let errs: Vec<_> = results
        .into_iter()
        .flatten()
        .filter_map(Result::err)
        .collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(Error::Sdk(errs))
    }
}

fn fallback_dsn() -> Dsn {
    Dsn::parse(FALLBACK_DSN).expect("fallback DSN is valid")
}

fn install_panic_hook(provider: SdkTracerProvider) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        report_panic(&provider, info);
        prev(info);
    }));
}

fn report_panic(provider: &SdkTracerProvider, info: &PanicHookInfo<'_>) {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("Box<dyn Any>");
    let message = match info.location() {
        Some(location) => format!("{message} at {location}"),
        None => message.to_string(),
    };

    let attrs = vec![
        KeyValue::new(EXCEPTION_TYPE, "panic"),
        KeyValue::new(EXCEPTION_MESSAGE, message.clone()),
        KeyValue::new(EXCEPTION_STACKTRACE, Backtrace::force_capture().to_string()),
    ];

    let cx = Context::current();
    let span = cx.span();
    if span.is_recording() {
        span.add_event("exception", attrs);
        span.set_status(Status::error(message));
    } else {
        let mut span = provider.tracer(TRACER_NAME).start(DUMMY_SPAN_NAME);
        opentelemetry::trace::Span::add_event(&mut span, "exception", attrs);
        opentelemetry::trace::Span::set_status(&mut span, Status::error(message));
    }

    // The process is likely about to exit, so export the span right away.
    let _ = provider.force_flush();
}

#[cfg(test)]
mod tests {
    use opentelemetry::trace::{SpanId, TraceFlags, TraceId, TraceState};

    use super::*;

    #[test]
    fn trace_url() {
        let uptrace = Uptrace::disabled(Dsn::parse("http://token@localhost:14317/1").unwrap());
        let span_context = SpanContext::new(
            TraceId::from(0x0102030405060708090a0b0c0d0e0f10),
            SpanId::from(0x1112131415161718),
            TraceFlags::SAMPLED,
            false,
            TraceState::default(),
        );
        assert_eq!(
            uptrace.trace_url(&span_context),
            "http://localhost:14318/traces/0102030405060708090a0b0c0d0e0f10?span_id=1112131415161718"
        );
    }

    #[test]
    fn dummy_dsn_disables_uptrace() {
        let uptrace = Uptrace::builder()
            .with_dsn("https://<token>@api.uptrace.dev")
            .build()
            .unwrap();
        assert!(!uptrace.is_enabled());
        assert!(uptrace.tracer_provider().is_none());
        uptrace.report_error(&std::fmt::Error);
        uptrace.force_flush().unwrap();
        uptrace.shutdown().unwrap();
    }

    #[test]
    fn invalid_dsn_is_an_error() {
        let err = Uptrace::builder().with_dsn("").build().unwrap_err();
        assert!(matches!(err, Error::EmptyDsn));

        let err = Uptrace::builder()
            .with_dsn("http://localhost")
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidDsn { .. }));
    }
}
