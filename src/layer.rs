use crate::{
    datadog_ids,
    event_sink::{EventSink, StdoutSink},
    fields::{self, FieldPair, FieldStore},
    formatting::DatadogLog,
};
use chrono::Utc;
use tracing::{span::Attributes, Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// The layer responsible for formatting tracing events in a way datadog can parse them
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct DatadogFormattingLayer<Sink: EventSink + 'static> {
    event_sink: Sink,
}

impl<S: EventSink + 'static> DatadogFormattingLayer<S> {
    /// Create a new `DatadogFormattingLayer` with the provided event sink
    pub const fn with_sink(sink: S) -> Self {
        Self { event_sink: sink }
    }
}

impl Default for DatadogFormattingLayer<StdoutSink> {
    fn default() -> Self {
        Self::with_sink(StdoutSink::default())
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>, Sink: EventSink + 'static> Layer<S>
    for DatadogFormattingLayer<Sink>
{
    fn on_new_span(&self, span_attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        #[allow(clippy::expect_used)]
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        let fields = fields::from_attributes(span_attrs);

        // insert fields from new span e.g #[instrument(fields(hello = "world"))]
        if extensions.get_mut::<FieldStore>().is_none() {
            extensions.insert(FieldStore { fields });
        }
    }

    // IDEA: maybe a on record implementation is required here

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let event_fields = fields::from_event(event);

        // find message if present in event fields
        let message = event_fields
            .iter()
            .find(|pair| pair.name == "message")
            .map(|pair| pair.value.clone())
            .unwrap_or_default();

        let all_fields: Vec<FieldPair> = Vec::default()
            .into_iter()
            .chain(fields::from_spans(&ctx, event))
            .chain(event_fields)
            .collect();

        // look for datadog trace- and span-id
        let datadog_ids = datadog_ids::read_from_context(&ctx);

        let log = DatadogLog {
            timestamp: Utc::now(),
            level: event.metadata().level().to_owned(),
            message,
            fields: all_fields,
            target: event.metadata().target().to_string(),
            datadog_ids,
        };

        let serialized_event = log.format();

        self.event_sink.write(serialized_event);
    }
}

#[cfg(test)]
mod simple_layer {
    use self::setup::first_span;
    use super::*;
    use simple_layer::setup::setup_simple_subscriber;
    use smoothy::prelude::*;
    use tracing::info;

    #[test]
    fn simple_log() {
        let (sink, _guard) = setup_simple_subscriber();

        info!("Hello World!");

        let events = sink.events();
        assert_that(&events).size().is(1);

        assert_that(events).first().contains("\",\"level\":\"INFO\",\"message\":\"Hello World!\",\"target\":\"datadog_formatting_layer::layer::simple_layer\"}");
    }

    #[test]
    fn log_with_fields() {
        let (sink, _guard) = setup_simple_subscriber();

        info!(user = "John Doe", "Hello World!");

        let events = sink.events();
        assert_that(&events).size().is(1);

        assert_that(events).first().contains("\",\"level\":\"INFO\",\"fields.user\":\"John Doe\",\"message\":\"Hello World! user=John Doe\",\"target\":\"datadog_formatting_layer::layer::simple_layer\"}");
    }

    #[allow(clippy::redundant_clone)]
    #[test]
    fn complex_logs() {
        let (sink, _guard) = setup_simple_subscriber();

        first_span("Argument");

        let events = sink.events();
        assert_that(&events).size().is(3);

        assert_that(events.clone()).first().contains("\",\"level\":\"DEBUG\",\"fields.first_value\":\"Argument\",\"message\":\"First Span! first_value=Argument\",\"target\":\"datadog_formatting_layer::layer::setup\"}");
        assert_that(events.clone()).second().contains("\",\"level\":\"DEBUG\",\"fields.attr\":\"value\",\"fields.first_value\":\"Argument\",\"message\":\"Second Span! attr=value first_value=Argument\",\"target\":\"datadog_formatting_layer::layer::setup\"}");
        assert_that(events.clone()).third().contains("\",\"level\":\"INFO\",\"fields.attr\":\"value\",\"fields.first_value\":\"Argument\",\"fields.return\":\"Return Value\",\"message\":\" attr=value first_value=Argument return=Return Value\",\"target\":\"datadog_formatting_layer::layer::setup\"}");
    }
}

#[cfg(test)]
mod layer_with_otel {
    use self::setup::{first_span, setup_otel_subscriber};
    use super::*;
    use crate::layer::setup::{LogMessageExt, SmoothyExt};
    use smoothy::prelude::*;
    use tracing::info;

    #[tokio::test]
    async fn without_spans_has_no_datadog_ids() {
        let (sink, _guard) = setup_otel_subscriber().await;

        info!("Hello World!");

        let events = sink.events();

        assert_that(events.clone()).size().is(1);
        assert_that(events[0].trace_id()).is_none();
        assert_that(events[0].span_id()).is_none();
    }

    #[tokio::test]
    async fn with_spans_has_correct_datadog_ids() {
        let (sink, _guard) = setup_otel_subscriber().await;

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
}

#[cfg(test)]
mod setup {
    use super::*;
    use opentelemetry::global;
    use opentelemetry_datadog::ApiVersion;
    use opentelemetry_sdk::{
        propagation::TraceContextPropagator,
        runtime::Tokio,
        trace::{config, RandomIdGenerator, Sampler},
    };
    use serde_json::Value;
    use smoothy::prelude::*;
    use std::sync::{Arc, Mutex};
    use tracing::{debug, instrument, subscriber::DefaultGuard};
    use tracing_subscriber::prelude::*;

    pub fn setup_simple_subscriber() -> (ObservableSink, DefaultGuard) {
        let sink = ObservableSink::default();

        let subscriber =
            tracing_subscriber::registry().with(DatadogFormattingLayer::with_sink(sink.clone()));

        let guard = tracing::subscriber::set_default(subscriber);

        (sink, guard)
    }

    pub async fn setup_otel_subscriber() -> (ObservableSink, DefaultGuard) {
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
            .with(DatadogFormattingLayer::with_sink(sink.clone()))
            .with(tracing_opentelemetry::layer().with_tracer(tracer));

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

    #[derive(Debug, Clone, Default)]
    pub struct ObservableSink {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl EventSink for ObservableSink {
        #[allow(clippy::print_stdout)]
        fn write(&self, event: String) {
            println!("{event}");
            self.events.lock().unwrap().push(event);
        }
    }

    impl ObservableSink {
        pub fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    #[instrument]
    pub fn first_span(first_value: &str) {
        debug!("First Span!");
        second_span();
    }

    #[instrument(fields(attr = "value"), ret)]
    fn second_span() -> String {
        debug!("Second Span!");
        String::from("Return Value")
    }
}
