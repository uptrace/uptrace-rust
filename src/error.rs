use std::error::Error as StdError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("DSN is empty (use WithDSN or UPTRACE_DSN env var)")]
    EmptyDns,
    #[error("invalid dns: {}, reason: {}", .dns, .reason)]
    InvalidDns { dns: String, reason: String },
    #[error("trace build error: {}", 0)]
    TraceBuildError(Box<dyn StdError>),
    #[error("metrics build error: {}", 0)]
    MetricsBuildError(Box<dyn StdError>),
}
