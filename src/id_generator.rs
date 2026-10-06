use std::time::{SystemTime, UNIX_EPOCH};

use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_sdk::trace::IdGenerator;
use rand::Rng;

/// ID generator that prefixes trace IDs with the current time.
///
/// Time-ordered trace IDs let Uptrace store and look up traces more
/// efficiently:
/// - trace ID: 8 bytes of Unix time in nanoseconds followed by 8 random bytes;
/// - span ID: 4 bytes of Unix time in milliseconds followed by 4 random bytes.
#[derive(Clone, Debug, Default)]
pub struct UptraceIdGenerator {
    _private: (),
}

impl IdGenerator for UptraceIdGenerator {
    fn new_trace_id(&self) -> TraceId {
        let nanos = unix_nanos() as u64;
        let low: u64 = rand::rng().random();
        TraceId::from(((nanos as u128) << 64) | low as u128)
    }

    fn new_span_id(&self) -> SpanId {
        let millis = (unix_nanos() / 1_000_000) as u32;
        let low: u32 = rand::rng().random();
        SpanId::from(((millis as u64) << 32) | low as u64)
    }
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_has_time_prefix() {
        let before = unix_nanos() as u64;
        let id = UptraceIdGenerator::default().new_trace_id();
        let after = unix_nanos() as u64;

        let prefix = u64::from_be_bytes(id.to_bytes()[..8].try_into().unwrap());
        assert!((before..=after).contains(&prefix));
    }

    #[test]
    fn ids_are_unique() {
        let gen = UptraceIdGenerator::default();
        assert_ne!(gen.new_trace_id(), gen.new_trace_id());
        assert_ne!(gen.new_span_id(), gen.new_span_id());
        assert_ne!(gen.new_span_id(), SpanId::INVALID);
    }
}
