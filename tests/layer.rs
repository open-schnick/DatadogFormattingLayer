use datadog_formatting_layer::DatadogFormattingLayer;
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{prelude::*, Registry};

#[test]
fn feature() {
    Registry::default()
        // .with(tracing_subscriber::fmt::layer().compact())
        .with(DatadogFormattingLayer)
        .init();

    warn!("Warning");
    some_test("Fasel");
}

#[instrument(fields(hello = "world"))]
fn some_test(value: &str) {
    info!(ola = "salve", value, "Bla {value}");
    some_test2()
}

#[instrument(fields(world = "world"))]
fn some_test2() {
    debug!(ola = "salve", "Hello");
}
