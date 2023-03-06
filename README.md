# Uptrace for Rust

![build workflow](https://github.com/uptrace/uptrace-rust/actions/workflows/build.yml/badge.svg)
[![Documentation](https://img.shields.io/badge/uptrace-documentation-informational)](https://uptrace.dev/get/opentelemetry-rust.html)
[![Chat](https://img.shields.io/badge/-telegram-red?color=white&logo=telegram&logoColor=black)](https://t.me/uptrace)

<a href="https://uptrace.dev/get/opentelemetry-rust.html">
  <img src="https://uptrace.dev/get/devicon/rust-plain.svg" height="200px" />
</a>

## Introduction

uptrace-rust is an OpenTelemery Rust distribution configured to export
[traces](https://uptrace.dev/opentelemetry/distributed-tracing.html) and
[metrics](https://uptrace.dev/opentelemetry/metrics.html) to Uptrace.

## Quickstart

Install uptrace-rust:

```bash
cargo add uptrace
```

Run the [basic example](example/basic) below using the DSN from the Uptrace project settings page.

```shell
UPTRACE_DSN=http://project2_secret_token@localhost:14317/2 cargo run --example basic
```
