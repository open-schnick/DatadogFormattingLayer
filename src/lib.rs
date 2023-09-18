//! # Datadog Formatting Layer
//!
//! A crate providing a tracing-subscriber layer for formatting events so Datadog can parse them.
//!
//! ## Features
//! - Provides a layer for tracing-subscriber
//! - Generates parsable "logs" for datadog and prints them to stdout
//! - Enables log correlation between spans and "logs" (see [datadog docs](https://docs.datadoghq.com/tracing/other_telemetry/connect_logs_and_traces/))
//!
//! ## Why not just `tracing_subscriber::fmt().json()` ?
//! The problem is, that datadog expects the "logs" to be in a specific (mostly undocumented) json format.
//!
//! This crates tries to mimic this format.
//!
//! ## Usage
//!
//! ### Simple
//!
//! ```
//! use datadog_formatting_layer::DatadogFormattingLayer;
//! use tracing::info;
//! use tracing_subscriber::prelude::*;
//!
//! tracing_subscriber::registry()
//!     .with(DatadogFormattingLayer)
//!     .init();
//!
//! info!(user = "Jack", "Hello World!");
//! ```
//!
//! Running this code will result in the following output on stdout:
//!
//! ```json
//! {"timestamp":"2023-06-21T10:36:50.364874878+00:00","level":"INFO","message":"Hello World user=Jack","target":"simple"}
//! ```
//!
//! ### With Opentelemetry
//! ```
//! use datadog_formatting_layer::DatadogFormattingLayer;
//! use opentelemetry::{
//!     global,
//!     sdk::{
//!         propagation::TraceContextPropagator,
//!         trace::{RandomIdGenerator, Sampler},
//!     },
//! };
//! use opentelemetry_datadog::ApiVersion;
//! use tracing::{debug, error, info, instrument, warn};
//! use tracing_subscriber::{prelude::*, util::SubscriberInitExt};
//!
//! // Just some otel boilerplate
//! global::set_text_map_propagator(TraceContextPropagator::new());
//!
//! let tracer = opentelemetry_datadog::new_pipeline()
//!     .with_service_name("my-service")
//!     .with_trace_config(
//!         opentelemetry::sdk::trace::config()
//!             .with_sampler(Sampler::AlwaysOn)
//!             .with_id_generator(RandomIdGenerator::default()),
//!     )
//!     .with_api_version(ApiVersion::Version05)
//!     .with_env("rls")
//!     .with_version("420")
//!     .install_simple()
//!     .unwrap();
//!
//! // Use both the tracer and the formatting layer
//! tracing_subscriber::registry()
//!     .with(DatadogFormattingLayer)
//!     .with(tracing_opentelemetry::layer().with_tracer(tracer))
//!     .init();
//!
//! // Here no span exists
//! info!(user = "Jack", "Hello World!");
//! some_test("fasel");
//!
//! // This will create a span and a trace id which is attached to the "logs"
//! #[instrument(fields(hello = "world"))]
//! fn some_test(value: &str) {
//!     // Here some span exists
//!     info!(ola = "salve", value, "Bla {value}");
//! }
//! ```
//!
//! When running this code with an datadog agent installed the logs will be sent to datadog
//! and parsed there.
//!
//! Otherwise the following output will be printed to stdout
//!
//! ```json
//! {"timestamp":"2023-06-21T10:36:50.363224217+00:00","level":"INFO","message":"Hello World! user=Jack","target":"otel"}
//! {"timestamp":"2023-06-21T10:36:50.363384118+00:00","level":"INFO","message":"Bla fasel user=Jack ola=salve value=Fasel hello=world","target":"otel","dd.trace_id":0,"dd.span_id":10201226522570980512}
//! ```

#![deny(
    rust_2018_idioms,
    unused_must_use,
    clippy::nursery,
    clippy::pedantic,
    clippy::perf,
    clippy::cargo,
    clippy::correctness,
    clippy::dbg_macro,
    clippy::else_if_without_else,
    clippy::empty_drop,
    clippy::empty_structs_with_brackets,
    clippy::expect_used,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::integer_division,
    clippy::multiple_inherent_impl,
    clippy::panic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::same_name_method,
    clippy::string_to_string,
    clippy::todo,
    clippy::try_err,
    clippy::unimplemented,
    clippy::unnecessary_self_imports,
    clippy::unreachable,
    clippy::unwrap_used,
    clippy::wildcard_enum_match_arm,
    missing_docs
)]
#![allow(clippy::module_name_repetitions)]

mod datadog_ids;
mod fields;
mod layer;

// reexport
pub use layer::DatadogFormattingLayer;
