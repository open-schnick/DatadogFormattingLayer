use datadog_formatting_layer::DatadogFormattingLayer;
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::prelude::*;

#[test]
fn works() {
    tracing_subscriber::registry()
        .with(DatadogFormattingLayer)
        .init();

    warn!("Warning");
    some_test("Fasel");
}

#[instrument(fields(hello = "world"))]
fn some_test(value: &str) {
    info!(ola = "salve", value, "Bla {value}");
    some_test1()
}

#[instrument(fields(world = "world"))]
fn some_test1() {
    debug!(ola = "salve", "Hello");
    some_test2()
}

#[instrument()]
fn some_test2() {
    error!("Oh no :(");
}
