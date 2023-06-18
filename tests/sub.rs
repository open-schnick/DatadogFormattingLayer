use sub::DatadogFormattingLayer;
use tracing::{info, instrument};
use tracing_subscriber::{prelude::*, Registry};

#[test]
fn feature() {
    let subscriber = Registry::default()
        // .with(tracing_subscriber::fmt::layer().compact())
        .with(DatadogFormattingLayer);

    tracing::subscriber::set_global_default(subscriber).unwrap();

    some_test("Kevin");
}

#[instrument(fields(hello = "world"))]
fn some_test(value: &str) {
    info!(ola = "salve", "hello there {value}");
}
