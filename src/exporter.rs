use std::time::Duration;

use opentelemetry_otlp::{
    Compression, ExporterBuildError, LogExporter, MetricExporter, SpanExporter, WithExportConfig,
};
use opentelemetry_sdk::metrics::Temporality;

use crate::Dsn;

const EXPORT_TIMEOUT: Duration = Duration::from_secs(10);

/// OTLP transport used to send data to Uptrace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Protocol {
    /// OTLP/HTTP with protobuf encoding (the `http` feature).
    #[cfg(feature = "http")]
    Http,
    /// OTLP/gRPC (the `grpc` feature). Requires a Tokio runtime.
    #[cfg(feature = "grpc")]
    Grpc,
}

impl Default for Protocol {
    fn default() -> Self {
        #[cfg(feature = "http")]
        return Protocol::Http;
        #[cfg(not(feature = "http"))]
        return Protocol::Grpc;
    }
}

pub(crate) fn span_exporter(
    dsn: &Dsn,
    protocol: Protocol,
) -> Result<SpanExporter, ExporterBuildError> {
    let builder = SpanExporter::builder();
    match protocol {
        #[cfg(feature = "http")]
        Protocol::Http => http::configure(builder.with_http(), dsn, "/v1/traces").build(),
        #[cfg(feature = "grpc")]
        Protocol::Grpc => grpc::configure(builder.with_tonic(), dsn)?.build(),
    }
}

pub(crate) fn metric_exporter(
    dsn: &Dsn,
    protocol: Protocol,
    temporality: Temporality,
) -> Result<MetricExporter, ExporterBuildError> {
    let builder = MetricExporter::builder();
    match protocol {
        #[cfg(feature = "http")]
        Protocol::Http => http::configure(builder.with_http(), dsn, "/v1/metrics")
            .with_temporality(temporality)
            .build(),
        #[cfg(feature = "grpc")]
        Protocol::Grpc => grpc::configure(builder.with_tonic(), dsn)?
            .with_temporality(temporality)
            .build(),
    }
}

pub(crate) fn log_exporter(
    dsn: &Dsn,
    protocol: Protocol,
) -> Result<LogExporter, ExporterBuildError> {
    let builder = LogExporter::builder();
    match protocol {
        #[cfg(feature = "http")]
        Protocol::Http => http::configure(builder.with_http(), dsn, "/v1/logs").build(),
        #[cfg(feature = "grpc")]
        Protocol::Grpc => grpc::configure(builder.with_tonic(), dsn)?.build(),
    }
}

#[cfg(feature = "http")]
mod http {
    use std::collections::HashMap;

    use opentelemetry_otlp::WithHttpConfig;

    use super::*;

    pub(super) fn configure<B>(builder: B, dsn: &Dsn, path: &str) -> B
    where
        B: WithExportConfig + WithHttpConfig,
    {
        builder
            .with_endpoint(format!("{}{}", dsn.otlp_http_endpoint(), path))
            .with_headers(HashMap::from([("uptrace-dsn".into(), dsn.to_string())]))
            .with_compression(Compression::Gzip)
            .with_timeout(EXPORT_TIMEOUT)
    }
}

#[cfg(feature = "grpc")]
mod grpc {
    use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
    use opentelemetry_otlp::tonic_types::transport::ClientTlsConfig;
    use opentelemetry_otlp::WithTonicConfig;

    use super::*;

    pub(super) fn configure<B>(builder: B, dsn: &Dsn) -> Result<B, ExporterBuildError>
    where
        B: WithExportConfig + WithTonicConfig,
    {
        let mut metadata = MetadataMap::with_capacity(1);
        let value = dsn.to_string().parse().map_err(|_| {
            ExporterBuildError::InvalidConfiguration("DSN is not a valid gRPC header value".into())
        })?;
        metadata.insert("uptrace-dsn", value);

        let mut builder = builder
            .with_endpoint(dsn.otlp_grpc_endpoint())
            .with_metadata(metadata)
            .with_compression(Compression::Gzip)
            .with_timeout(EXPORT_TIMEOUT);
        if dsn.scheme() == "https" {
            builder = builder.with_tls_config(ClientTlsConfig::new().with_native_roots());
        }
        Ok(builder)
    }
}
