# Datadog Formatting Layer

A crate providing a tracing-subscriber layer for formatting events so Datadog can parse them.

[![Release](https://github.com/open-schnick/DatadogFormattingLayer/actions/workflows/release.yml/badge.svg)](https://github.com/open-schnick/DatadogFormattingLayer/actions/workflows/release.yml)
[![Test](https://github.com/open-schnick/DatadogFormattingLayer/actions/workflows/test.yml/badge.svg)](https://github.com/open-schnick/DatadogFormattingLayer/actions/workflows/test.yml)
![License](https://img.shields.io/crates/l/datadog-formatting-layer)
[![Crates.io](https://img.shields.io/crates/v/datadog-formatting-layer)](https://crates.io/crates/datadog-formatting-layer)

## Features

- Provides a layer for tracing-subscriber
- Generates parsable "logs" for datadog and prints them to stdout
- Enables log correlation between spans and "logs" (see [datadog docs](https://docs.datadoghq.com/tracing/other_telemetry/connect_logs_and_traces/))

## Why not just `tracing_subscriber::fmt().json()` ?

The problem is, that datadog expects the "logs" to be in a specific (mostly undocumented) json format.

This crates tries to mimic this format.

## Usage

### Simple

```rust
use datadog_formatting_layer::DatadogFormattingLayer;
use tracing::info;
use tracing_subscriber::prelude::*;

tracing_subscriber::registry()
    .with(DatadogFormattingLayer::default())
    .init();

info!(user = "Jack", "Hello World!");
```

Running this code will result in the following output on stdout:

```json
{
  "timestamp": "2023-06-21T10:36:50.364874878+00:00",
  "level": "INFO",
  "fields.user": "Jack",
  "message": "Hello World user=Jack",
  "target": "simple"
}
```

### With Opentelemetry

```rust
use datadog_formatting_layer::DatadogFormattingLayer;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_datadog::ApiVersion;
use opentelemetry_sdk::trace::{Config, RandomIdGenerator, Sampler};
use tracing::{dispatcher::DefaultGuard, info, Level, instrument};
use tracing_subscriber::{filter::Targets, prelude::*};

fn main() {
    // IMPORTANT: as of otel version 0.28 this has to be called outside the context of an async runtime
    let provider = opentelemetry_datadog::new_pipeline()
        .with_service_name("my-service")
        .with_trace_config(
            Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default()),
        )
        .with_api_version(ApiVersion::Version05)
        .with_env("rls")
        .with_version("420")
        .install_batch()
        .unwrap();

    let tracer = provider.tracer("my-service");

    global::set_tracer_provider(provider);

    // Use both the tracer and the formatting layer
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(DatadogFormattingLayer::default())
        .with(tracing_opentelemetry::layer().with_tracer(tracer));

    let _guard = tracing::subscriber::set_default(subscriber);

    // Here no span exists
    info!(user = "Jack", "Hello World!");
    some_test("fasel");
}

// This will create a span and a trace id which is attached to the "logs"
#[instrument(fields(hello = "world"))]
fn some_test(value: &str) {
    // Here some span exists
    info!(ola = "salve", value, "Bla {value}");
}
```

When running this code with a datadog agent installed the logs will be sent to datadog
and parsed there.

Otherwise, the following output will be printed to stdout (fields are excluded for readability)

```json
{
  "timestamp": "2023-06-21T10:36:50.363224217+00:00",
  "level": "INFO",
  "message": "Hello World! user=Jack",
  "target": "otel"
}
{
  "timestamp": "2023-06-21T10:36:50.363384118+00:00",
  "level": "INFO",
  "message": "Bla fasel user=Jack ola=salve value=Fasel hello=world",
  "target": "otel",
  "dd.trace_id": 0,
  "dd.span_id": 10201226522570980512
}
```

## Supported Opentelemetry versions:

| OpenTelemetry | DatadogFormattingLayer |
| ------------- | ---------------------- |
| 0.28.\*       | 4.\*                   |
| 0.23.\*       | 3.\*                   |
| 0.22.\*       | 2.1.\*, 2.2.\*         |
| 0.20.\*       | 1.1.\*, 2.0.\*         |
| 0.19.\*       | 1.0.\*                 |
