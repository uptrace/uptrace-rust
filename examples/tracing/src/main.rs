use tracing::{error, info, info_span};
use tracing_subscriber::{prelude::*, EnvFilter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads the DSN from the UPTRACE_DSN env var (format: https://uptrace.dev/get#dsn).
    let uptrace = uptrace::Uptrace::builder()
        .with_service_name("myservice")
        .with_service_version("1.0.0")
        .build()?;

    // Don't export the exporter's own logs to avoid feedback loops.
    let otel_filter = EnvFilter::new("info")
        .add_directive("hyper=off".parse()?)
        .add_directive("h2=off".parse()?)
        .add_directive("tonic=off".parse()?)
        .add_directive("reqwest=off".parse()?);

    tracing_subscriber::registry()
        .with(uptrace.tracing_layer())
        .with(uptrace.log_layer().with_filter(otel_filter))
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::new("info")))
        .init();

    info_span!("handle-request", http.route = "/users/:id").in_scope(|| {
        info!(user_id = 123, "loading user");
        error!(user_id = 123, "user not found");

        // Logs emitted inside a span are linked to its trace.
        println!("View trace: {}", uptrace.current_trace_url());
    });

    uptrace.shutdown()?;
    Ok(())
}
