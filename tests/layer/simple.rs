use crate::{first_span, ObservableSink};
use datadog_formatting_layer::DatadogFormattingLayer;
use smoothy::prelude::*;
use tracing::{dispatcher::DefaultGuard, info, Level};
use tracing_subscriber::{prelude::*, FmtSubscriber};

#[test]
fn simple_logs_get_formatted_and_printed() {
    let (sink, _guard) = setup_simple_subscriber();

    info!("Hello World!");

    let events = sink.events();
    assert_that(&events).size().is(1);

    assert_that(events).first().contains(
        "\",\"level\":\"INFO\",\"message\":\"Hello World!\",\"target\":\"layer::simple\"}",
    );
}

#[test]
fn fields_are_formatted_and_printed() {
    let (sink, _guard) = setup_simple_subscriber();

    info!(user = "John Doe", "Hello World!");

    let events = sink.events();
    assert_that(&events).size().is(1);

    assert_that(events).first().contains("\",\"level\":\"INFO\",\"fields.user\":\"John Doe\",\"message\":\"Hello World! user=John Doe\",\"target\":\"layer::simple\"}");
}

#[allow(clippy::redundant_clone)]
#[test]
fn complex_logs() {
    let (sink, _guard) = setup_simple_subscriber();

    first_span("Argument");

    let events = sink.events();
    assert_that(&events).size().is(3);

    assert_that(events.clone()).first().contains("\",\"level\":\"DEBUG\",\"fields.first_value\":\"Argument\",\"message\":\"First Span! first_value=Argument\",\"target\":\"layer\"}");
    assert_that(events.clone()).second().contains("\",\"level\":\"DEBUG\",\"fields.attr\":\"value\",\"fields.first_value\":\"Argument\",\"message\":\"Second Span! attr=value first_value=Argument\",\"target\":\"layer\"}");
    assert_that(events.clone()).third().contains("\",\"level\":\"INFO\",\"fields.attr\":\"value\",\"fields.first_value\":\"Argument\",\"fields.return\":\"Return Value\",\"message\":\" attr=value first_value=Argument return=Return Value\",\"target\":\"layer\"}");
}

fn setup_simple_subscriber() -> (ObservableSink, DefaultGuard) {
    let sink = ObservableSink::default();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish()
        .with(DatadogFormattingLayer::with_sink(sink.clone()));

    let guard = tracing::subscriber::set_default(subscriber);

    (sink, guard)
}
