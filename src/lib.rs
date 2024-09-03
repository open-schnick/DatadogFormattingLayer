#![doc = include_str!("../README.md")]
// Disable certain lints in tests
#![cfg_attr(
    test,
    allow(
        clippy::pedantic,
        clippy::as_conversions,
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::unwrap_in_result
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
