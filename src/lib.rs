pub mod builder;
pub use builder::Builder;

pub mod dns;

pub mod error;
pub use error::Error;

pub mod id;
pub mod uptrace;

pub fn configure_opentelemetry() {
    if env::uptrace_disable() {
        return;
    }
}

pub mod env {
    const UPTRACE_DISABLED: &str = "UPTRACE_DISABLED";

    #[inline]
    pub(crate) fn uptrace_disable() -> bool {
        std::env::var(UPTRACE_DISABLED).is_ok()
    }
}
