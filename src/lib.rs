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
//!         .install_simple()?;
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

use opentelemetry::runtime;
use opentelemetry::sdk::export::metrics::aggregation::delta_temporality_selector;
use opentelemetry::sdk::metrics::selectors;
use opentelemetry::{
    global,
    sdk::{
        trace::{self, BatchSpanProcessor, TracerProvider},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::{ExportConfig, Protocol, SpanExporterBuilder, WithExportConfig};
use tonic::metadata::MetadataMap;

pub struct UptraceBuilder {
    dsn: Option<String>,

    trace_config: Option<trace::Config>,
    batch_config: Option<trace::BatchConfig>,

    service_name: Option<String>,
    service_version: Option<String>,
    deployment_environment: Option<String>,

    disable_trace: bool,
    disable_metrics: bool,
}

impl Default for UptraceBuilder {
    fn default() -> Self {
        Self {
            dsn: std::env::var("UPTRACE_DSN").ok(),

            trace_config: None,
            batch_config: None,

            service_name: None,
            service_version: None,
            deployment_environment: None,

            disable_metrics: false,
            disable_trace: false,
        }
    }
}

impl UptraceBuilder {
    pub fn new() -> UptraceBuilder {
        Default::default()
    }

    pub fn with_dsn<T: Into<String>>(self, dsn: T) -> Self {
        Self {
            dsn: Some(dsn.into()),
            ..self
        }
    }

    pub fn with_trace_config(self, config: trace::Config) -> Self {
        Self {
            trace_config: Some(config),
            ..self
        }
    }

    pub fn with_batch_config(self, config: trace::BatchConfig) -> Self {
        Self {
            batch_config: Some(config),
            ..self
        }
    }

    pub fn with_service_name<T: Into<String>>(self, service_name: T) -> Self {
        Self {
            service_name: Some(service_name.into()),
            ..self
        }
    }

    pub fn with_service_version<T: Into<String>>(self, service_version: T) -> Self {
        Self {
            service_version: Some(service_version.into()),
            ..self
        }
    }

    pub fn with_deployment_environment<T: Into<String>>(self, deployment_environment: T) -> Self {
        Self {
            deployment_environment: Some(deployment_environment.into()),
            ..self
        }
    }

    pub fn with_disable_trace(self) -> Self {
        Self {
            disable_trace: true,
            ..self
        }
    }

    pub fn with_disable_metrics(self) -> Self {
        Self {
            disable_metrics: true,
            ..self
        }
    }

    pub fn configure_opentelemetry(mut self) -> Result<(), Error> {
        if std::env::var("UPTRACE_DISABLED").is_ok() {
            return Ok(());
        }

        let dsn = Dsn::try_from(self.dsn.take().unwrap_or_default())?;
        if dsn.is_disabled() {
            return Ok(());
        }

        if !self.disable_trace {
            self.init_tracing(&dsn)?;
        }

        if !self.disable_metrics {
            self.init_metrics(&dsn)?;
        }

        Ok(())
    }
}

impl UptraceBuilder {
    fn build_resource(&mut self) -> Resource {
        let mut kv = vec![];

        if let Ok(host) = hostname::get() {
            kv.push(KeyValue::new(
                "host.name",
                host.to_str().unwrap_or_default().to_string(),
            ));
        }

        if let Some(service_name) = self.service_name.take() {
            kv.push(KeyValue::new("service.name", service_name));
        }

        if let Some(service_version) = self.service_version.take() {
            kv.push(KeyValue::new("service.version", service_version));
        }

        if let Some(deployment_environment) = self.deployment_environment.take() {
            kv.push(KeyValue::new(
                "deployment.environment",
                deployment_environment,
            ));
        }

        Resource::new(kv.into_iter())
    }

    fn init_tracing(&mut self, dsn: &Dsn) -> Result<(), Error> {
        let resource = self.build_resource();
        self.trace_config = if let Some(cfg) = self.trace_config.take() {
            let new_resource = Resource::empty();
            new_resource.merge(&resource);
            new_resource.merge(cfg.resource.as_ref());
            Some(cfg.with_resource(new_resource))
        } else {
            let cfg = trace::config();
            Some(cfg.with_resource(Resource::empty().merge(&resource)))
        };

        let mut metadata = MetadataMap::with_capacity(1);
        metadata.insert("uptrace-dsn", dsn.original.as_str().parse().unwrap());

        let span_builder: SpanExporterBuilder = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(dsn.otlp_grpc_addr())
            .with_timeout(Duration::from_secs(5))
            .with_metadata(metadata)
            .into();

        let exporter = span_builder
            .build_span_exporter()
            .map_err(|e| Error::TraceBuildError(Box::new(e)))?;

        let bz = BatchSpanProcessor::builder(exporter, runtime::Tokio)
            .with_batch_config(self.batch_config.take().unwrap_or_else(|| {
                trace::BatchConfig::default()
                    .with_max_queue_size(1000)
                    .with_max_export_batch_size(1000)
                    .with_scheduled_delay(Duration::from_millis(5000))
            }))
            .build();

        let provider = TracerProvider::builder()
            .with_config(self.trace_config.take().unwrap_or_default())
            .with_span_processor(bz)
            .build();

        global::set_tracer_provider(provider);
        Ok(())
    }

    fn init_metrics(&mut self, dsn: &Dsn) -> Result<(), Error> {
        let export_config = ExportConfig {
            endpoint: dsn.otlp_grpc_addr(),
            timeout: Duration::from_secs(10),
            protocol: Protocol::Grpc,
        };

        let mut metadata = MetadataMap::with_capacity(1);
        metadata.insert("uptrace-dsn", dsn.original.as_str().parse().unwrap());

        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_export_config(export_config)
            .with_metadata(metadata);

        let _ctrl = opentelemetry_otlp::new_pipeline()
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

        Ok(())
    }
}
