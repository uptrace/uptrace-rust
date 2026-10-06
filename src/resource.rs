use opentelemetry::KeyValue;
use opentelemetry_resource_detectors::{
    HostResourceDetector, OsResourceDetector, ProcessResourceDetector,
};
use opentelemetry_sdk::resource::ResourceDetector;
use opentelemetry_sdk::Resource;

/// Builds the resource shared by all providers.
///
/// Resource attributes are merged in this order (later wins):
/// SDK defaults, `OTEL_RESOURCE_ATTRIBUTES` / `OTEL_SERVICE_NAME`, host, OS and
/// process detectors, user detectors, and finally user attributes.
pub(crate) fn build_resource(
    detectors: Vec<Box<dyn ResourceDetector + Send + Sync>>,
    attributes: Vec<KeyValue>,
) -> Resource {
    let builder = Resource::builder()
        .with_detector(Box::new(HostResourceDetector::default()))
        .with_detector(Box::new(OsResourceDetector))
        .with_detector(Box::new(ProcessResourceDetector));
    detectors
        .into_iter()
        .fold(builder, |b, d| b.with_detector(d))
        .with_attributes(attributes)
        .build()
}
