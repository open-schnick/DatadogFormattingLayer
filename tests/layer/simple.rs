use crate::ObservableSink;
use datadog_formatting_layer::DatadogFormattingLayer;
use smoothy::prelude::*;
use tracing::{debug, dispatcher::DefaultGuard, info, instrument, Level};
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
    #[instrument(ret)]
    fn first(args: &str) {
        debug!("In first {args}");
        let _ = second();
    }
    #[instrument(ret)]
    fn second() -> Result<(), String> {
        Err("Error!".to_string())
    }

    let (sink, _guard) = setup_simple_subscriber();

    first("Span");

    let events = sink.events();
    assert_that(&events).size().is(3);

    assert_that(events.clone()).first().contains("\"level\":\"DEBUG\",\"fields.args\":\"Span\",\"message\":\"In first Span args=Span\",\"target\":\"layer::simple\"}");
    assert_that(events.clone()).second().contains("\"level\":\"INFO\",\"fields.args\":\"Span\",\"fields.return\":\"Err(\\\"Error!\\\")\",\"message\":\" args=Span return=Err(\\\"Error!\\\")\",\"target\":\"layer::simple\"}");
    assert_that(events.clone()).third().contains("\"level\":\"INFO\",\"fields.args\":\"Span\",\"fields.return\":\"()\",\"message\":\" args=Span return=()\",\"target\":\"layer::simple\"}");
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
