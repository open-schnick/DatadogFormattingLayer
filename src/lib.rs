#![doc = include_str!("../README.md")]
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
// Disable certain lints in tests
#![cfg_attr(
    test,
    allow(
        clippy::pedantic,
        clippy::as_conversions,
        clippy::indexing_slicing,
        clippy::unwrap_used
    )
)]

mod datadog_ids;
mod event_sink;
mod fields;
mod formatting;
mod layer;

// reexport
pub use event_sink::{EventSink, StdoutSink};
pub use layer::DatadogFormattingLayer;
