use opentelemetry::sdk::trace::{IdGenerator, RandomIdGenerator};

#[derive(Debug, Default)]
pub(crate) struct GenerateId(RandomIdGenerator);

impl IdGenerator for GenerateId {
    fn new_trace_id(&self) -> opentelemetry::trace::TraceId {
        self.0.new_trace_id()
    }

    fn new_span_id(&self) -> opentelemetry::trace::SpanId {
        self.0.new_span_id()
    }
}
