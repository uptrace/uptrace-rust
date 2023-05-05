//! # Quickstart
//!
//!
//! ```no_run
//! use uptrace::UptraceBuilder;
//! use opentelemetry::{global, trace::{Tracer, Span}, KeyValue};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // You can also start to tracing and metrics.
//!     UptraceBuilder::new()
//!         .with_dsn("http://project2_secret_token@localhost:14317/2")
//!         .with_service_name("lol")
//!         .configure_opentelemetry()?;
//!
//!     let tracer = global::tracer("rust-service");
//!     let mut span = tracer.start("my_span");
//!     span.set_attribute(KeyValue::new("http.client_ip", "83.164.160.102"));
//!     span.set_attribute(KeyValue::new("now", "2022-01-18 15:00:00"));
//!     span.end();
//!
//!     println!("{:?}", span.span_context().trace_id().to_string());
//!     global::shutdown_tracer_provider();
//!     Ok(())
//! }
//! ```
//!
//! [uptrace]: https://uptrace.dev/

use std::time::Duration;

pub mod dsn;
pub use dsn::Dsn;

pub mod error;
pub use error::Error;

use opentelemetry::sdk;
use opentelemetry::sdk::export::metrics::aggregation::delta_temporality_selector;
use opentelemetry::sdk::metrics::controllers::BasicController;
use opentelemetry::sdk::metrics::selectors;
use opentelemetry::sdk::resource::{
    EnvResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector,
};
use opentelemetry::sdk::{runtime, Resource};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{
    ExportConfig, Protocol, SpanExporter, SpanExporterBuilder, WithExportConfig,
};
use tonic::metadata::MetadataMap;

use opentelemetry::trace::TracerProvider;

pub struct UptraceBuilder {
    dsn: String,

    service_name: Option<String>,
    service_version: Option<String>,
    deployment_environment: Option<String>,

    tracing_disabled: bool,
    metrics_disabled: bool,

    trace_config: sdk::trace::Config,
    batch_config: sdk::trace::BatchConfig,
}

impl Default for UptraceBuilder {
    fn default() -> Self {
        Self {
            dsn: std::env::var("UPTRACE_DSN").ok().unwrap(),

            trace_config: sdk::trace::Config::default(),
            batch_config: sdk::trace::BatchConfig::default()
                .with_max_queue_size(30000)
                .with_max_export_batch_size(10000)
                .with_scheduled_delay(Duration::from_millis(5000)),

            service_name: None,
            service_version: None,
            deployment_environment: None,

            metrics_disabled: false,
            tracing_disabled: false,
        }
    }
}

impl UptraceBuilder {
    pub fn new() -> UptraceBuilder {
        Default::default()
    }

    pub fn with_dsn<T: Into<String>>(mut self, dsn: T) -> Self {
        self.dsn = dsn.into();
        self
    }

    /// Set the trace provider configuration.
    pub fn with_trace_config(mut self, trace_config: sdk::trace::Config) -> Self {
        self.trace_config = trace_config;
        self
    }

    /// Set the batch span processor configuration, and it will override the env vars.
    pub fn with_batch_config(mut self, batch_config: sdk::trace::BatchConfig) -> Self {
        self.batch_config = batch_config;
        self
    }

    pub fn with_service_name<T: Into<String>>(mut self, service_name: T) -> Self {
        self.service_name = Some(service_name.into());
        self
    }

    pub fn with_service_version<T: Into<String>>(mut self, service_version: T) -> Self {
        self.service_version = Some(service_version.into());
        self
    }

    pub fn with_deployment_environment<T: Into<String>>(
        mut self,
        deployment_environment: T,
    ) -> Self {
        self.deployment_environment = Some(deployment_environment.into());
        self
    }

    pub fn with_tracing_disabled(mut self) -> Self {
        self.tracing_disabled = true;
        self
    }

    pub fn with_metrics_disabled(mut self) -> Self {
        self.metrics_disabled = true;
        self
    }

    pub fn configure_opentelemetry<R: sdk::trace::TraceRuntime>(
        mut self,
        runtime: R,
    ) -> Result<(), Error> {
        if std::env::var("UPTRACE_DISABLED").is_ok() {
            return Ok(());
        }

        let dsn = Dsn::try_from(self.dsn.clone())?;
        if dsn.is_disabled() {
            return Ok(());
        }

        if !self.tracing_disabled {
            self.init_tracer(&dsn, runtime)?;
        }

        if !self.metrics_disabled {
            self.init_metrics(&dsn)?;
        }

        Ok(())
    }
}

impl UptraceBuilder {
    pub fn init_tracer<R: sdk::trace::TraceRuntime>(
        &mut self,
        dsn: &Dsn,
        runtime: R,
    ) -> Result<sdk::trace::Tracer, Error> {
        let mut metadata = MetadataMap::with_capacity(1);
        metadata.insert("uptrace-dsn", self.dsn.parse().unwrap());

        let exporter_builder: SpanExporterBuilder = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(dsn.otlp_grpc_addr())
            .with_timeout(Duration::from_secs(5))
            .with_metadata(metadata)
            .into();
        let span_exporter = exporter_builder
            .build_span_exporter()
            .map_err(|e| Error::TraceBuildError(Box::new(e)))?;
        let resource = self.build_resource();

        Ok(build_batch_with_exporter(
            span_exporter,
            self.trace_config.with_resource(resource),
            runtime,
            self.batch_config,
        ))
    }

    pub fn init_metrics(&mut self, dsn: &Dsn) -> Result<BasicController, Error> {
        let mut metadata = MetadataMap::with_capacity(1);
        metadata.insert("uptrace-dsn", self.dsn.parse().unwrap());

        let export_config = ExportConfig {
            endpoint: dsn.otlp_grpc_addr(),
            timeout: Duration::from_secs(10),
            protocol: Protocol::Grpc,
        };
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_export_config(export_config)
            .with_metadata(metadata);

        let ctrl = opentelemetry_otlp::new_pipeline()
            .metrics(
                selectors::simple::inexpensive(),
                delta_temporality_selector(),
                runtime::Tokio,
            )
            .with_exporter(exporter)
            .with_period(Duration::from_secs(15))
            .with_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| Error::MetricsBuildError(Box::new(e)))?;

        Ok(ctrl)
    }

    fn build_resource(&self) -> Resource {
        let mut kv = vec![];

        if let Ok(host) = hostname::get() {
            kv.push(KeyValue::new(
                "host.name",
                host.to_str().unwrap_or_default().to_string(),
            ));
        }

        if let Some(service_name) = self.service_name.clone() {
            kv.push(KeyValue::new("service.name", service_name));
        }

        if let Some(service_version) = self.service_version.clone() {
            kv.push(KeyValue::new("service.version", service_version));
        }

        if let Some(deployment_environment) = self.deployment_environment.clone() {
            kv.push(KeyValue::new(
                "deployment.environment",
                deployment_environment,
            ));
        }

        Resource::from_detectors(
            Duration::from_secs(0),
            vec![
                Box::new(SdkProvidedResourceDetector),
                Box::new(EnvResourceDetector::new()),
                Box::new(TelemetryResourceDetector),
            ],
        )
        .merge(&mut Resource::new(kv.into_iter()))
    }
}

fn build_batch_with_exporter<R: sdk::trace::TraceRuntime>(
    exporter: SpanExporter,
    trace_config: sdk::trace::Config,
    runtime: R,
    batch_config: sdk::trace::BatchConfig,
) -> sdk::trace::Tracer {
    let batch_processor = sdk::trace::BatchSpanProcessor::builder(exporter, runtime)
        .with_batch_config(batch_config)
        .build();

    let provider_builder = sdk::trace::TracerProvider::builder();
    let provider = provider_builder
        .with_span_processor(batch_processor)
        .with_config(trace_config)
        .build();

    let tracer = provider.versioned_tracer("uptrace-rust", Some(env!("CARGO_PKG_VERSION")), None);
    let _ = global::set_tracer_provider(provider);
    tracer
}
