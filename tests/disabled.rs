//! Runs in its own process because it sets env vars.

#[test]
fn uptrace_disabled_env_var() {
    std::env::set_var("UPTRACE_DISABLED", "1");
    // The DSN is not even parsed when Uptrace is disabled.
    let uptrace = uptrace::Uptrace::builder().with_dsn("").build().unwrap();
    assert!(!uptrace.is_enabled());
    assert_eq!(uptrace.dsn().site_url(), "https://app.uptrace.dev");
    uptrace.shutdown().unwrap();
}
