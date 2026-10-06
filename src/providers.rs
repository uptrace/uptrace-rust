use opentelemetry_sdk::logs::{self, BatchLogProcessor, SdkLoggerProvider};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{self, BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_sdk::Resource;

use crate::config::{env_is_set, queue_size, EXPORT_INTERVAL};
use crate::exporter::{self, Protocol};
use crate::{Dsn, Error, LogsConfig, MetricsConfig, TracesConfig, UptraceIdGenerator};

pub(crate) fn tracer_provider(
    dsn: &Dsn,
    protocol: Protocol,
    resource: Resource,
    mut config: TracesConfig,
) -> Result<SdkTracerProvider, Error> {
    let exporter = exporter::span_exporter(dsn, protocol)?;
    let batch_config = config
        .batch_config
        .take()
        .unwrap_or_else(default_span_batch_config);
    let processor = BatchSpanProcessor::builder(exporter)
        .with_batch_config(batch_config)
        .build();

    let mut builder = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(processor);
    if !config.has_id_generator() {
        builder = builder.with_id_generator(UptraceIdGenerator::default());
    }
    if let Some(sampler) = config.sampler.take() {
        builder = builder.with_sampler(sampler);
    }
    Ok(config.apply(builder).build())
}

pub(crate) fn meter_provider(
    dsn: &Dsn,
    protocol: Protocol,
    resource: Resource,
    mut config: MetricsConfig,
) -> Result<SdkMeterProvider, Error> {
    let exporter = exporter::metric_exporter(dsn, protocol, config.temporality)?;
    let reader = PeriodicReader::builder(exporter)
        .with_interval(config.interval)
        .build();

    let builder = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader);
    Ok(config.apply(builder).build())
}

pub(crate) fn logger_provider(
    dsn: &Dsn,
    protocol: Protocol,
    resource: Resource,
    mut config: LogsConfig,
) -> Result<SdkLoggerProvider, Error> {
    let exporter = exporter::log_exporter(dsn, protocol)?;
    let batch_config = config
        .batch_config
        .take()
        .unwrap_or_else(default_log_batch_config);
    let processor = BatchLogProcessor::builder(exporter)
        .with_batch_config(batch_config)
        .build();

    let builder = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_log_processor(processor);
    Ok(config.apply(builder).build())
}

// Uptrace defaults are applied only where the user did not set the standard
// OTEL_BSP_* / OTEL_BLRP_* env vars, which the SDK builders already read.

fn default_span_batch_config() -> trace::BatchConfig {
    let mut builder = trace::BatchConfigBuilder::default();
    if !env_is_set(trace::OTEL_BSP_MAX_QUEUE_SIZE) {
        builder = builder.with_max_queue_size(queue_size());
    }
    if !env_is_set(trace::OTEL_BSP_MAX_EXPORT_BATCH_SIZE) {
        builder = builder.with_max_export_batch_size(queue_size());
    }
    if !env_is_set(trace::OTEL_BSP_SCHEDULE_DELAY) {
        builder = builder.with_scheduled_delay(EXPORT_INTERVAL);
    }
    builder.build()
}

fn default_log_batch_config() -> logs::BatchConfig {
    let mut builder = logs::BatchConfigBuilder::default();
    if !env_is_set(logs::OTEL_BLRP_MAX_QUEUE_SIZE) {
        builder = builder.with_max_queue_size(queue_size());
    }
    if !env_is_set(logs::OTEL_BLRP_MAX_EXPORT_BATCH_SIZE) {
        builder = builder.with_max_export_batch_size(queue_size());
    }
    if !env_is_set(logs::OTEL_BLRP_SCHEDULE_DELAY) {
        builder = builder.with_scheduled_delay(EXPORT_INTERVAL);
    }
    builder.build()
}
