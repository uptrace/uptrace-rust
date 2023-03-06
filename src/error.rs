use std::error::Error as StdError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("DSN is empty (use WithDSN or UPTRACE_DSN env var)")]
    EmptyDsn,
    #[error("invalid dsn: {}, reason: {}", .dsn, .reason)]
    InvalidDsn { dsn: String, reason: String },
    #[error("trace build error: {}", 0)]
    TraceBuildError(Box<dyn StdError>),
    #[error("metrics build error: {}", 0)]
    MetricsBuildError(Box<dyn StdError>),
}
