use opentelemetry_otlp::ExporterBuildError;
use opentelemetry_sdk::error::OTelSdkError;

/// Errors returned by this crate.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// No DSN was configured.
    #[error("DSN is empty (use with_dsn or UPTRACE_DSN env var)")]
    EmptyDsn,

    /// The DSN could not be parsed.
    #[error("invalid DSN {dsn:?}: {reason}")]
    InvalidDsn {
        /// The DSN as provided.
        dsn: String,
        /// Why the DSN is invalid.
        reason: String,
    },

    /// An OTLP exporter could not be created.
    #[error("can't create OTLP exporter: {0}")]
    Exporter(#[from] ExporterBuildError),

    /// One or more providers failed to flush or shut down.
    #[error("OpenTelemetry SDK error: {}", join(.0))]
    Sdk(Vec<OTelSdkError>),
}

impl From<OTelSdkError> for Error {
    fn from(err: OTelSdkError) -> Self {
        Error::Sdk(vec![err])
    }
}

fn join(errs: &[OTelSdkError]) -> String {
    errs.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
