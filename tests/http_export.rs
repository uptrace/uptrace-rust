//! Exports traces, metrics, and logs to a mock OTLP/HTTP server.
#![cfg(feature = "http")]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use opentelemetry::logs::{LogRecord, Logger, LoggerProvider};
use opentelemetry::metrics::MeterProvider;
use opentelemetry::trace::{TraceContextExt, Tracer, TracerProvider};
use uptrace::Uptrace;

struct Request {
    path: String,
    headers: HashMap<String, String>,
    body_len: usize,
}

fn serve(listener: TcpListener, tx: mpsc::Sender<Request>) {
    for stream in listener.incoming() {
        let tx = tx.clone();
        thread::spawn(move || handle(stream.unwrap(), tx));
    }
}

/// Handles keep-alive connections with one or more requests each.
fn handle(stream: TcpStream, tx: mpsc::Sender<Request>) {
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let path = line
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_string();

        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }

        let body_len: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; body_len];
        reader.read_exact(&mut body).unwrap();

        writer
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/x-protobuf\r\ncontent-length: 0\r\n\r\n",
            )
            .unwrap();
        let _ = tx.send(Request {
            path,
            headers,
            body_len,
        });
    }
}

#[test]
fn exports_all_signals() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || serve(listener, tx));

    let dsn = format!("http://project_token@127.0.0.1:{port}/1");
    let uptrace = Uptrace::builder()
        .with_dsn(&dsn)
        .with_service_name("test-service")
        .with_global(false)
        .build()
        .unwrap();
    assert!(uptrace.is_enabled());

    let tracer = uptrace.tracer_provider().unwrap().tracer("test");
    let url = tracer.in_span("test-span", |cx| {
        uptrace.trace_url(cx.span().span_context())
    });
    assert!(url.starts_with(&format!("http://127.0.0.1:{port}/traces/")));

    let meter = uptrace.meter_provider().unwrap().meter("test");
    meter.u64_counter("test_counter").build().add(1, &[]);

    let logger = uptrace.logger_provider().unwrap().logger("test");
    let mut record = logger.create_log_record();
    record.set_body("hello".into());
    logger.emit(record);

    uptrace.shutdown().unwrap();

    let mut paths = Vec::new();
    while let Ok(req) = rx.recv_timeout(Duration::from_secs(5)) {
        assert_eq!(req.headers.get("uptrace-dsn"), Some(&dsn), "{}", req.path);
        assert_eq!(
            req.headers.get("content-encoding").map(String::as_str),
            Some("gzip")
        );
        assert!(req.body_len > 0);
        paths.push(req.path);
        if ["/v1/traces", "/v1/metrics", "/v1/logs"]
            .iter()
            .all(|p| paths.iter().any(|x| x == p))
        {
            return;
        }
    }
    panic!("not all signals were exported, got {paths:?}");
}
