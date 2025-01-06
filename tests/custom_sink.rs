#![allow(missing_docs)]
use datadog_formatting_layer::{DatadogFormattingLayer, StdoutSink};
use tracing::warn;
use tracing_subscriber::prelude::*;

#[test]
fn works() {
    tracing_subscriber::registry()
        .with(DatadogFormattingLayer::with_sink(StdoutSink::default()))
        .init();

    warn!("Warning");
}
