//! Builds the OTLP/gRPC pipeline, which must run inside a Tokio runtime.
#![cfg(feature = "grpc")]

use uptrace::{Protocol, Uptrace};

#[tokio::test(flavor = "multi_thread")]
async fn builds_grpc_exporters() {
    for dsn in [
        "http://token@127.0.0.1:14317/1",
        "https://token@api.uptrace.dev?grpc=4317",
    ] {
        let uptrace = Uptrace::builder()
            .with_dsn(dsn)
            .with_protocol(Protocol::Grpc)
            .with_global(false)
            .build()
            .unwrap();
        assert!(uptrace.is_enabled(), "{dsn}");
        uptrace.shutdown().unwrap();
    }
}
