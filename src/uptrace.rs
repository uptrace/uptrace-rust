use opentelemetry::global;

use crate::Builder;

#[derive(Default)]
pub struct Uptrace {
    pub(crate) enable: bool,
}

impl Uptrace {
    pub fn builder() -> Builder {
        Builder::default()
    }
}

impl Drop for Uptrace {
    // todo shutdown metric
    fn drop(&mut self) {
        if self.enable {
            global::shutdown_tracer_provider();
        }
    }
}
