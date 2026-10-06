use std::fmt;
use std::time::Duration;

use opentelemetry_sdk::logs::{self, LogProcessor, LoggerProviderBuilder};
use opentelemetry_sdk::metrics::{Instrument, MeterProviderBuilder, Stream, Temporality};
use opentelemetry_sdk::trace::{
    self, IdGenerator, ShouldSample, SpanLimits, SpanProcessor, TracerProviderBuilder,
};

/// A deferred change to an SDK provider builder.
///
/// Storing closures lets users pass any concrete SDK type (processors, ID
/// generators, views) without the SDK having to implement its traits for `Box`.
type Customizer<B> = Box<dyn FnOnce(B) -> B + Send>;

fn apply<B>(builder: B, customizers: Vec<Customizer<B>>) -> B {
    customizers.into_iter().fold(builder, |b, f| f(b))
}

/// Configuration for exporting traces to Uptrace.
///
/// ```
/// use uptrace::TracesConfig;
/// use opentelemetry_sdk::trace::Sampler;
///
/// let config = TracesConfig::default()
///     .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.5))));
/// ```
#[derive(Default)]
pub struct TracesConfig {
    pub(crate) sampler: Option<Box<dyn ShouldSample>>,
    pub(crate) batch_config: Option<trace::BatchConfig>,
    customizers: Vec<Customizer<TracerProviderBuilder>>,
    has_id_generator: bool,
}

impl TracesConfig {
    /// Sets the sampler. The default is the SDK default (parent-based, always on),
    /// which can also be configured with `OTEL_TRACES_SAMPLER`.
    pub fn with_sampler<T: ShouldSample + 'static>(mut self, sampler: T) -> Self {
        self.sampler = Some(Box::new(sampler));
        self
    }

    /// Overrides the batch span processor configuration used for the Uptrace exporter.
    ///
    /// By default the queue size scales with the number of CPUs (1000 to 16000 spans)
    /// and spans are exported every 10 seconds; `OTEL_BSP_*` env vars take precedence.
    pub fn with_batch_config(mut self, config: trace::BatchConfig) -> Self {
        self.batch_config = Some(config);
        self
    }

    /// Replaces the default [`UptraceIdGenerator`](crate::UptraceIdGenerator).
    pub fn with_id_generator<T: IdGenerator + 'static>(mut self, id_generator: T) -> Self {
        self.has_id_generator = true;
        self.customize(move |b| b.with_id_generator(id_generator))
    }

    /// Registers an additional span processor, e.g. to export spans elsewhere too.
    pub fn with_span_processor<T: SpanProcessor + 'static>(self, processor: T) -> Self {
        self.customize(move |b| b.with_span_processor(processor))
    }

    /// Sets limits on the number of span attributes, events and links.
    pub fn with_span_limits(self, limits: SpanLimits) -> Self {
        self.customize(move |b| b.with_span_limits(limits))
    }

    fn customize(
        mut self,
        f: impl FnOnce(TracerProviderBuilder) -> TracerProviderBuilder + Send + 'static,
    ) -> Self {
        self.customizers.push(Box::new(f));
        self
    }

    pub(crate) fn has_id_generator(&self) -> bool {
        self.has_id_generator
    }

    pub(crate) fn apply(&mut self, builder: TracerProviderBuilder) -> TracerProviderBuilder {
        apply(builder, std::mem::take(&mut self.customizers))
    }
}

impl fmt::Debug for TracesConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TracesConfig")
            .field("sampler", &self.sampler)
            .field("batch_config", &self.batch_config)
            .finish_non_exhaustive()
    }
}

/// Configuration for exporting metrics to Uptrace.
pub struct MetricsConfig {
    pub(crate) interval: Duration,
    pub(crate) temporality: Temporality,
    customizers: Vec<Customizer<MeterProviderBuilder>>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(15),
            temporality: Temporality::Delta,
            customizers: Vec::new(),
        }
    }
}

impl MetricsConfig {
    /// Sets how often metrics are exported. The default is 15 seconds.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the aggregation temporality. The default is [`Temporality::Delta`],
    /// which Uptrace processes most efficiently.
    pub fn with_temporality(mut self, temporality: Temporality) -> Self {
        self.temporality = temporality;
        self
    }

    /// Registers a view to customize the metric streams, e.g. to rename an
    /// instrument or change histogram buckets.
    pub fn with_view<F>(mut self, view: F) -> Self
    where
        F: Fn(&Instrument) -> Option<Stream> + Send + Sync + 'static,
    {
        self.customizers.push(Box::new(move |b| b.with_view(view)));
        self
    }

    pub(crate) fn apply(&mut self, builder: MeterProviderBuilder) -> MeterProviderBuilder {
        apply(builder, std::mem::take(&mut self.customizers))
    }
}

impl fmt::Debug for MetricsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetricsConfig")
            .field("interval", &self.interval)
            .field("temporality", &self.temporality)
            .finish_non_exhaustive()
    }
}

/// Configuration for exporting logs to Uptrace.
#[derive(Default)]
pub struct LogsConfig {
    pub(crate) batch_config: Option<logs::BatchConfig>,
    customizers: Vec<Customizer<LoggerProviderBuilder>>,
}

impl LogsConfig {
    /// Overrides the batch log processor configuration used for the Uptrace exporter.
    ///
    /// By default the queue size scales with the number of CPUs (1000 to 16000 records)
    /// and logs are exported every 10 seconds; `OTEL_BLRP_*` env vars take precedence.
    pub fn with_batch_config(mut self, config: logs::BatchConfig) -> Self {
        self.batch_config = Some(config);
        self
    }

    /// Registers an additional log processor.
    pub fn with_processor<T: LogProcessor + 'static>(mut self, processor: T) -> Self {
        self.customizers
            .push(Box::new(move |b| b.with_log_processor(processor)));
        self
    }

    pub(crate) fn apply(&mut self, builder: LoggerProviderBuilder) -> LoggerProviderBuilder {
        apply(builder, std::mem::take(&mut self.customizers))
    }
}

impl fmt::Debug for LogsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogsConfig")
            .field("batch_config", &self.batch_config)
            .finish_non_exhaustive()
    }
}

/// Queue size for batch processors, scaled by the available parallelism.
pub(crate) fn queue_size() -> usize {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    (cpus / 2 * 1000).clamp(1000, 16000)
}

pub(crate) const EXPORT_INTERVAL: Duration = Duration::from_secs(10);

/// Reports whether the env var is set to a non-empty value.
pub(crate) fn env_is_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_size_is_clamped() {
        let n = queue_size();
        assert!((1000..=16000).contains(&n));
    }
}
