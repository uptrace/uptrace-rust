use std::time::Duration;

use opentelemetry::{
    global, propagation, runtime,
    sdk::{
        propagation::{BaggagePropagator, TextMapCompositePropagator, TraceContextPropagator},
        resource,
        trace::{
            self as sdktrace, BatchSpanProcessor, BatchSpanProcessorBuilder, Config, TracerProvider,
        },
        Resource,
    },
    trace::{
        self,
        noop::{NoopTextMapPropagator, NoopTracerProvider},
    },
    KeyValue,
};
use opentelemetry_otlp::{ExportConfig, SpanExporter, SpanExporterBuilder, WithExportConfig};

use crate::{dns::Dns, error::Error, id::GenerateId, uptrace::Uptrace};

const UPTRACE_DSN: &str = "UPTRACE_DSN";
const UPTRACE_DISABLED: &str = "UPTRACE_DISABLED";

pub struct Builder {
    dns: Option<String>,

    resource_attributes: Option<Vec<opentelemetry::KeyValue>>,
    resource_detectors: Option<Vec<Box<dyn resource::ResourceDetector>>>,
    resource: Option<resource::Resource>,

    tracing_enabled: bool,
    trace_sampler: Option<sdktrace::Sampler>,
    pretty_print: bool,
    // bspOptions    :    Vec<sdktrace.BatchSpanProcessorOption>
    metrics_enabled: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            dns: std::env::var(UPTRACE_DSN).ok(),
            resource_attributes: None,
            resource_detectors: None,
            resource: None,
            tracing_enabled: true,
            trace_sampler: None,
            pretty_print: false,
            metrics_enabled: true,
        }
    }
}

impl Builder {
    pub fn with_dns(mut self, dns: impl Into<String>) -> Self {
        self.dns = Some(dns.into());
        self
    }

    pub fn with_resource_attributes(
        mut self,
        resource_attributes: impl Iterator<Item = opentelemetry::KeyValue>,
    ) -> Self {
        self.resource_attributes = Some(resource_attributes.collect());
        self
    }

    pub fn with_resource_detectors(
        mut self,
        resource_detectors: impl Iterator<Item = Box<dyn resource::ResourceDetector>>,
    ) -> Self {
        self.resource_detectors = Some(resource_detectors.collect());
        self
    }

    pub fn with_resource(mut self, resource: resource::Resource) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn with_tracing_enabled(mut self, enable: bool) -> Self {
        self.tracing_enabled = enable;
        self
    }
    pub fn with_trace_sampler(mut self, trace_sampler: sdktrace::Sampler) -> Self {
        self.trace_sampler = Some(trace_sampler);
        self
    }

    pub fn with_pretty_print(mut self, pretty_print: bool) -> Self {
        self.pretty_print = pretty_print;
        self
    }

    pub fn with_metrics_enabled(mut self, metrics_enabled: bool) -> Self {
        self.metrics_enabled = metrics_enabled;
        self
    }

    #[must_use]
    pub fn build(mut self) -> Result<Uptrace, Error> {
        if std::env::var(UPTRACE_DISABLED).is_ok() {
            return Ok(Default::default());
        }

        if !self.metrics_enabled && !self.tracing_enabled {
            return Ok(Default::default());
        }

        if self.dns.is_none() {
            return Err(Error::EmptyDns);
        }

        let dns = Dns::try_from(self.dns.take().unwrap_or_default())?;
        self.configure_propagator();

        // todo
        self.configure_metrics(&dns)?;

        self.configure_tracing(&dns)?;

        Ok(Uptrace { enable: true })
    }

    #[inline]
    fn configure_propagator(&mut self) {
        global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
            Box::new(BaggagePropagator::new()),
            Box::new(TraceContextPropagator::new()),
        ]))
    }

    fn configure_tracing(&mut self, dns: &Dns) -> Result<(), Error> {
        let span_builder: SpanExporterBuilder = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(dns.otlp_grpc_addr())
            .with_timeout(Duration::from_secs(5))
            // .with_channel(channel) // todo
            .into();

        let exporter = span_builder
            .build_span_exporter()
            .map_err(|e| Error::SpanBuildError(Box::new(e)))?;

        let bz = BatchSpanProcessor::builder(exporter, runtime::Tokio)
            .with_max_queue_size(1000)
            .with_max_export_batch_size(1000)
            .with_scheduled_delay(Duration::from_millis(5000))
            .build();

        let provider = TracerProvider::builder()
            .with_config(
                Config::default()
                    .with_id_generator(GenerateId::default())
                    .with_resource(self.resource()),
            )
            .with_span_processor(bz)
            .build();

        global::set_tracer_provider(provider);

        Ok(())
    }

    fn configure_metrics(&mut self, dns: &Dns) -> Result<(), Error> {
        // let mut builder = opentelemetry_otlp::GrpcioExporterBuilder::default()
        //     .with_endpoint(dns.otlp_host())
        //     .with_headers(
        //         vec![("uptrace-dsn".to_string(), dns.to_string())]
        //             .into_iter()
        //             .collect(),
        //     )
        //     .with_compression(Compression::Gzip);

        // // todo
        // if dns.scheme == "https" {
        //     // builder = builder.with_credentials(credentials::)
        // }
        // // let metric = MetricsExporter::new

        // global.set_meter_provider()
        Ok(())
    }

    pub fn resource(&mut self) -> Resource {
        let mut attr = self.resource_attributes.take().unwrap_or_default();
        attr.push(KeyValue::new(
            "host.name",
            hostname::get()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string(),
        ));

        attr.push(KeyValue::new("service.name", "rust test"));

        let resource = Resource::new(attr);
        resource
    }
}
