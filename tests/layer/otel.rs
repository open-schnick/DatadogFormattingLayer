use crate::{first_span, ObservableSink};
use datadog_formatting_layer::DatadogFormattingLayer;
use opentelemetry::global;
use opentelemetry_datadog::ApiVersion;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    runtime::Tokio,
    trace::{config, RandomIdGenerator, Sampler},
};
use serde_json::Value;
use smoothy::prelude::*;
use tracing::{dispatcher::DefaultGuard, info, Level};
use tracing_subscriber::{filter::Targets, prelude::*};

#[tokio::test]
async fn without_spans_has_no_datadog_ids() {
    let (sink, _guard) = setup_otel_subscriber();

    info!("Hello World!");

    let events = sink.events();

    assert_that(events.clone()).size().is(1);
    assert_that(events[0].trace_id()).is_none();
    assert_that(events[0].span_id()).is_none();
}

#[tokio::test]
async fn with_spans_has_correct_datadog_ids() {
    let (sink, _guard) = setup_otel_subscriber();

    first_span("Argument");

    let events = sink.events();
    assert_that(&events).size().is(3);

    // First message has no trace id but a span id
    assert_that(events[0].trace_id()).is_not_valid();
    assert_that(events[0].span_id()).is_valid();
    // second message has trace id and different span id
    assert_that(events[1].trace_id()).is_valid();
    assert_that(events[1].span_id()).is_valid();
    assert_that(events[1].span_id()).is_not(events[0].span_id());
    // third message has same trace id as the second and different span id
    assert_that(events[2].trace_id()).is_valid();
    assert_that(events[2].trace_id()).is(events[1].trace_id());
    assert_that(events[2].span_id()).is_valid();
    assert_that(events[2].span_id()).is(events[1].span_id());
}

fn setup_otel_subscriber() -> (ObservableSink, DefaultGuard) {
    let sink = ObservableSink::default();

    // otel boilerplate
    global::set_text_map_propagator(TraceContextPropagator::new());

    let tracer = opentelemetry_datadog::new_pipeline()
        .with_service_name("my-service")
        .with_trace_config(
            config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default()),
        )
        .with_api_version(ApiVersion::Version05)
        .with_env("rls")
        .with_version("420")
        .install_batch(Tokio)
        .unwrap();

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
    #[allow(clippy::wrong_self_convention)]
    fn is_not_valid(self);
}

impl SmoothyExt for BasicAsserter<Option<u64>> {
    fn is_valid(self) {
        self.is_some().and_value().is_not(0);
    }

    fn is_not_valid(self) {
        self.is_some().and_value().is(0);
    }
}
