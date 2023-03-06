use std::{thread, time::Duration};

use opentelemetry::{global, Context};
use uptrace::UptraceBuilder;

#[tokio::main]
async fn main() {
    UptraceBuilder::new()
        .with_service_name("myservice")
        .with_service_version("1.0.0")
        .with_deployment_environment("testing")
        .configure_opentelemetry()
        .unwrap();

    let meter = global::meter("app_or_crate_name");
    let histogram = meter.f64_histogram("ex.com.three").init();

    let cx = Context::new();
    for _i in 1..100000 {
        histogram.record(&cx, 1.3, &[]);
        thread::sleep(Duration::from_millis(100));
    }
}
