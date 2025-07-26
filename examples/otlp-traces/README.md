# Using OTLP Traces Exporter with Uptrace

This example demonstrates how to configure
[OpenTelemetry Rust](https://uptrace.dev/get/opentelemetry-rust) to export spans to Uptrace.

## Prerequisites

Before running this example, you need to [create an Uptrace project](https://uptrace.dev/get) to
obtain your project DSN.

## Running the Example

Execute the example by setting your project DSN via the `UPTRACE_DSN` environment variable:

```shell
UPTRACE_DSN="https://<project_secret>@api.uptrace.dev?grpc=4317" cargo run
```

Replace `<project_secret>` with your actual project secret from your Uptrace project.
