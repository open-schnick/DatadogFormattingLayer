use crate::ObservableSink;
use datadog_formatting_layer::DatadogFormattingLayer;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_datadog::ApiVersion;
use opentelemetry_sdk::trace::{Config, RandomIdGenerator, Sampler};
use serde_json::Value;
use smoothy::prelude::*;
use tracing::{debug, dispatcher::DefaultGuard, error, info, instrument, span, warn, Level};
use tracing_subscriber::{filter::Targets, prelude::*};

#[test]
fn events_outside_spans_have_no_datadog_ids() {
    let (sink, _guard) = setup_otel_subscriber();

    info!("Hello World!");

    let events = sink.events();

    assert_that(&events).size().is(1);
    assert_that(events[0].trace_id()).is_none();
    assert_that(events[0].span_id()).is_none();
}

#[test]
fn first_span_generates_trace_id() {
    let (sink, _guard) = setup_otel_subscriber();

    info!("No trace or span");

    span!(Level::INFO, "span").in_scope(|| debug!("This has a trace and a span"));

    let events = sink.events();

    assert_that(&events).size().is(2);
    assert_that(events[0].trace_id()).is_none();
    assert_that(events[0].span_id()).is_none();
    assert_that(events[1].trace_id()).is_valid();
    assert_that(events[1].span_id()).is_valid();
}

#[test]
fn events_in_nested_spans_have_correct_ids() {
    let (sink, _guard) = setup_otel_subscriber();

    span!(Level::INFO, "first span").in_scope(|| {
        debug!("This has a trace and a span id");

        span!(Level::INFO, "second span")
            .in_scope(|| error!("This has the same trace id but a different span id"));

        warn!("This has the same trace and span id as the first");
    });

    let events = sink.events();
    assert_that(&events).size().is(3);

    // First message generated trace and span id
    assert_that(events[0].trace_id()).is_valid();
    assert_that(events[0].span_id()).is_valid();
    // second message has the same trace id but a different span id
    assert_that(events[1].trace_id()).is(events[0].trace_id());
    assert_that(events[1].span_id()).is_valid();
    assert_that(events[1].span_id()).is_not(events[0].span_id());
    // third message has same trace and span id as the first
    assert_that(events[2].trace_id()).is(events[0].trace_id());
    assert_that(events[2].span_id()).is(events[0].span_id());
}

#[test]
fn events_created_by_instrument_macro_are_correctly_printed() {
    #[instrument(ret)]
    fn first(args: &str) {
        debug!("In first {args}");
        let _ = second();
    }
    #[instrument(ret)]
    fn second() -> Result<(), String> {
        Err("Error!".to_string())
    }

    let (sink, _guard) = setup_otel_subscriber();

    first("Span");

    let events = sink.events();
    assert_that(&events).size().is(3);

    assert_that(events.clone()).first().contains("\"level\":\"DEBUG\",\"fields.args\":\"Span\",\"message\":\"In first Span args=Span\",\"target\":\"layer::otel\"");
    assert_that(events.clone()).second().contains("\"level\":\"INFO\",\"fields.args\":\"Span\",\"fields.return\":\"Err(\\\"Error!\\\")\",\"message\":\" args=Span return=Err(\\\"Error!\\\")\",\"target\":\"layer::otel\"");
    assert_that(events.clone()).third().contains("\"level\":\"INFO\",\"fields.args\":\"Span\",\"fields.return\":\"()\",\"message\":\" args=Span return=()\",\"target\":\"layer::otel\"");
}

fn setup_otel_subscriber() -> (ObservableSink, DefaultGuard) {
    let sink = ObservableSink::default();

    #[allow(deprecated)]
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
        .install_simple()
        .unwrap();

    let tracer = provider.tracer("my-service");

    global::set_tracer_provider(provider);

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(DatadogFormattingLayer::with_sink(sink.clone()))
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(
            Targets::new()
                .with_target("layer::otel", Level::TRACE)
                .with_target("layer", Level::TRACE)
                .with_default(Level::ERROR),
        );

    let guard = tracing::subscriber::set_default(subscriber);

    (sink, guard)
}

pub trait LogMessageExt {
    fn span_id(&self) -> Option<u64>;
    fn trace_id(&self) -> Option<u64>;
}

impl LogMessageExt for String {
    fn span_id(&self) -> Option<u64> {
        let log: Value = serde_json::from_str(self).unwrap();
        log.get("dd.span_id")
            .map(|span_id| span_id.as_u64().unwrap())
    }

    fn trace_id(&self) -> Option<u64> {
        let log: Value = serde_json::from_str(self).unwrap();
        log.get("dd.trace_id")
            .map(|span_id| span_id.as_u64().unwrap())
    }
}

pub trait SmoothyExt {
    #[allow(clippy::wrong_self_convention)]
    fn is_valid(self);
}

impl SmoothyExt for BasicAsserter<Option<u64>> {
    fn is_valid(self) {
        self.is_some().and_value().is_not(0);
    }
}
