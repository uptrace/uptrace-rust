use std::time::Duration;

pub mod dns;
pub use dns::Dns;

pub mod error;
pub use error::Error;

use opentelemetry::{
    global, runtime,
    sdk::{
        export::metrics::aggregation::delta_temporality_selector,
        metrics::{controllers, processors, selectors},
        trace::{self, BatchSpanProcessor, TracerProvider},
        Resource,
    },
    Context, KeyValue,
};
use opentelemetry_otlp::{SpanExporterBuilder, WithExportConfig};
use tonic::metadata::MetadataMap;

pub struct UptraceBuilder {
    dns: Option<String>,

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
            dns: std::env::var("UPTRACE_DSN").ok(),

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

    pub fn with_dns<T: Into<String>>(self, dns: T) -> Self {
        Self {
            dns: Some(dns.into()),
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

    pub fn install_simple(mut self) -> Result<(), Error> {
        if std::env::var("UPTRACE_DISABLED").is_ok() {
            return Ok(());
        }

        let dns = Dns::try_from(self.dns.take().unwrap_or_default())?;
        if dns.is_disabled() {
            return Ok(());
        }

        if !self.disable_trace {
            self.build_trace(&dns)?;
        }

        if !self.disable_metrics {
            self.build_metrics(&dns)?;
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

    fn build_trace(&mut self, dns: &Dns) -> Result<(), Error> {
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

        let span_builder: SpanExporterBuilder = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(dns.otlp_grpc_addr())
            .with_timeout(Duration::from_secs(5))
            .with_metadata({
                let mut map = MetadataMap::new();
                map.insert(
                    "uptrace-dsn",
                    dns.original
                        .as_str()
                        .parse()
                        .map_err(|e| Error::TraceBuildError(Box::new(e)))?,
                );
                map
            })
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

    fn build_metrics(&mut self, _dns: &Dns) -> Result<(), Error> {
        let builder = controllers::basic(processors::factory(
            selectors::simple::inexpensive(),
            delta_temporality_selector(),
        ))
        .with_resource(self.build_resource())
        .with_collect_period(Duration::from_secs(15))
        .with_collect_timeout(Duration::from_secs(5))
        .with_push_timeout(Duration::from_secs(5));

        let controller = builder.build();
        controller
            .start(&Context::current(), runtime::Tokio)
            .map_err(|e| Error::MetricsBuildError(Box::new(e)))?;

        global::set_meter_provider(controller);
        Ok(())
    }
}
