use crate::{
    datadog_ids::{self, DatadogSpanId, DatadogTraceId},
    event_sink::{EventSink, StdoutSink},
    fields::{self, FieldPair, FieldStore},
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
        let mut message = event_fields
            .iter()
            .find(|pair| pair.name == "message")
            .map(|pair| pair.value.clone())
            .unwrap_or_default();

        let all_fields: Vec<FieldPair> = Vec::default()
            .into_iter()
            .chain(fields::from_spans(&ctx, event))
            .chain(event_fields)
            .collect();

        // serialize the event metadata and fields
        for field in all_fields {
            // message is just a regular field
            if field.name != "message" {
                message.push_str(&format!(
                    " {}={}",
                    field.name,
                    field.value.trim_matches('\"')
                ));
            }
        }

        // look for datadog trace- and span-id
        let datadog_ids = datadog_ids::read_from_context(&ctx);

        // IDEA: maybe loggerName instead of target
        // IDEA: maybe use fields as attributes or something
        let formatted_event = DatadogFormattedEvent {
            timestamp: Utc::now().to_rfc3339(),
            level: event.metadata().level().to_string(),
            message,
            target: event.metadata().target().to_string(),
            datadog_trace_id: datadog_ids.0,
            datadog_span_id: datadog_ids.1,
        };

        let mut serialized_event = serde_json::to_string(&formatted_event)
            .unwrap_or_else(|_| "Failed to serialize an event to json".to_string());

        serialized_event.push('\n');

        self.event_sink.write(serialized_event);
    }
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(Debug, serde::Deserialize))]
struct DatadogFormattedEvent {
    timestamp: String,
    level: String,
    message: String,
    target: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dd.trace_id")]
    datadog_trace_id: Option<DatadogTraceId>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dd.span_id")]
    datadog_span_id: Option<DatadogSpanId>,
}

#[cfg(test)]
mod simple {
    use super::setup::{first_span, setup_simple_subscriber};
    use smoothy::assert_that;
    use tracing::{debug, error, info, trace, warn};

    #[test]
    fn single_event() {
        let (sink, _guard) = setup_simple_subscriber();

        info!("Hello World!");

        let events = sink.events();
        assert_that(&events).size().is(1);

        assert_that(events[0].timestamp.is_empty()).is(false);
        assert_that(events[0].level.clone()).equals("INFO");
        assert_that(events[0].message.clone()).equals("Hello World!");
        assert_that(events[0].target.clone()).equals("datadog_formatting_layer::layer::simple");
        assert_that(events[0].datadog_span_id).is_none();
        assert_that(events[0].datadog_trace_id).is_none();
    }

    #[test]
    fn events_with_different_level() {
        let (sink, _guard) = setup_simple_subscriber();

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!("Warn");
        error!("Error");

        let events = sink.events();
        assert_that(&events).size().is(5);

        assert_that(events[0].level.clone()).equals("TRACE");
        assert_that(events[1].level.clone()).equals("DEBUG");
        assert_that(events[2].level.clone()).equals("INFO");
        assert_that(events[3].level.clone()).equals("WARN");
        assert_that(events[4].level.clone()).equals("ERROR");
    }

    #[test]
    fn event_with_single_field() {
        let (sink, _guard) = setup_simple_subscriber();

        info!(user = "Kevin", "Hello World!");

        let events = sink.events();
        assert_that(&events).size().is(1);

        assert_that(events[0].message.clone()).equals("Hello World! user=Kevin");
    }

    #[test]
    fn event_with_multiple_fields() {
        let (sink, _guard) = setup_simple_subscriber();

        info!(yaks = 42, yaks_shaved = true, "Shaving");

        let events = sink.events();
        assert_that(&events).size().is(1);

        // Order of fields is not guaranteed
        assert_that(events[0].message.clone())
            .contains("Shaving")
            .and()
            .contains("yaks=42")
            .and()
            .contains("yaks_shaved=true");
    }

    #[test]
    fn spans_with_fields() {
        let (sink, _guard) = setup_simple_subscriber();

        first_span("Argument");

        let events = sink.events();
        assert_that(&events).size().is(3);

        // Order of fields is not guaranteed
        assert_that(events[0].message.clone()).equals("First Span! first_value=Argument");
        assert_that(events[1].message.clone())
            .contains("Second Span!")
            .and()
            .contains("first_value=Argument")
            .and()
            .contains("attr=value");
        assert_that(events[2].message.clone())
            .contains("return=Return Value")
            .and()
            .contains("first_value=Argument")
            .and()
            .contains("attr=value");
    }
}

#[cfg(test)]
mod otel {
    use super::setup::{first_span, setup_otel_subscriber, SmoothyExt};
    use smoothy::assert_that;
    use tracing::{debug, error, info, trace, warn};

    #[test]
    fn single_event() {
        let (sink, _guard) = setup_otel_subscriber();

        info!("Hello World!");

        let events = sink.events();
        assert_that(&events).size().is(1);

        assert_that(events[0].timestamp.is_empty()).is(false);
        assert_that(events[0].level.clone()).equals("INFO");
        assert_that(events[0].message.clone()).equals("Hello World!");
        assert_that(events[0].target.clone()).equals("datadog_formatting_layer::layer::otel");
        assert_that(events[0].datadog_span_id).is_none();
        assert_that(events[0].datadog_trace_id).is_none();
    }

    #[test]
    fn events_with_different_level() {
        let (sink, _guard) = setup_otel_subscriber();

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!("Warn");
        error!("Error");

        let events = sink.events();
        assert_that(&events).size().is(5);

        assert_that(events[0].level.clone()).equals("TRACE");
        assert_that(events[1].level.clone()).equals("DEBUG");
        assert_that(events[2].level.clone()).equals("INFO");
        assert_that(events[3].level.clone()).equals("WARN");
        assert_that(events[4].level.clone()).equals("ERROR");
    }

    #[test]
    fn event_with_single_field() {
        let (sink, _guard) = setup_otel_subscriber();

        info!(user = "Kevin", "Hello World!");

        let events = sink.events();
        assert_that(&events).size().is(1);

        assert_that(events[0].message.clone()).equals("Hello World! user=Kevin");
    }

    #[test]
    fn event_with_multiple_fields() {
        let (sink, _guard) = setup_otel_subscriber();

        info!(yaks = 42, yaks_shaved = true, "Shaving");

        let events = sink.events();
        assert_that(&events).size().is(1);

        // Order of fields is not guaranteed
        assert_that(events[0].message.clone())
            .contains("Shaving")
            .and()
            .contains("yaks=42")
            .and()
            .contains("yaks_shaved=true");
    }

    #[test]
    fn spans_with_fields() {
        let (sink, _guard) = setup_otel_subscriber();

        first_span("Argument");

        let events = sink.events();
        assert_that(&events).size().is(3);

        // Order of fields is not guaranteed
        assert_that(events[0].message.clone()).equals("First Span! first_value=Argument");
        assert_that(events[1].message.clone())
            .contains("Second Span!")
            .and()
            .contains("first_value=Argument")
            .and()
            .contains("attr=value");
        assert_that(events[2].message.clone())
            .contains("return=Return Value")
            .and()
            .contains("first_value=Argument")
            .and()
            .contains("attr=value");
    }

    #[test]
    fn events_within_spans_contain_trace_and_span_id() {
        let (sink, _guard) = setup_otel_subscriber();

        first_span("Argument");

        let events = sink.events();
        assert_that(&events).size().is(3);

        let first_span_id = events[0].datadog_span_id.unwrap();
        let first_trace_id = events[0].datadog_trace_id.unwrap();
        let second_span_id = events[1].datadog_span_id.unwrap();
        let second_trace_id = events[1].datadog_trace_id.unwrap();
        let third_span_id = events[2].datadog_span_id.unwrap();
        let third_trace_id = events[2].datadog_trace_id.unwrap();

        assert_that(first_span_id).is_any_valid_id();
        // first event does not have a trace id
        assert_that(first_trace_id).is_not_a_valid_id();

        assert_that(second_span_id).is_any_valid_id();
        assert_that(second_span_id.0).is_not(first_span_id.0);
        assert_that(second_trace_id).is_any_valid_id();

        assert_that(third_span_id).is_any_valid_id();
        assert_that(third_span_id.0).is_not(first_span_id.0);
        assert_that(third_span_id.0).is(second_span_id.0);
        assert_that(third_trace_id.0).is(second_trace_id.0);
    }
}

#[cfg(test)]
mod setup {
    use super::*;
    use opentelemetry::global;
    use opentelemetry_datadog::ApiVersion;
    use opentelemetry_sdk::{
        propagation::TraceContextPropagator,
        trace::{config, RandomIdGenerator, Sampler},
    };
    use smoothy::BasicAsserter;
    use std::sync::{Arc, Mutex};
    use tracing::{debug, instrument, subscriber::DefaultGuard, warn};
    use tracing_subscriber::prelude::*;

    pub fn setup_simple_subscriber() -> (ObservableSink, DefaultGuard) {
        let sink = ObservableSink::default();

        let subscriber = tracing_subscriber::registry()
            .with(DatadogFormattingLayer::with_sink(sink.clone()))
            .with(tracing_subscriber::fmt::layer());

        let guard = tracing::subscriber::set_default(subscriber);

        (sink, guard)
    }

    pub fn setup_otel_subscriber() -> (ObservableSink, DefaultGuard) {
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
            .install_simple()
            .unwrap();

        let subscriber = tracing_subscriber::registry()
            .with(DatadogFormattingLayer::with_sink(sink.clone()))
            .with(tracing_opentelemetry::layer().with_tracer(tracer));

        let guard = tracing::subscriber::set_default(subscriber);

        (sink, guard)
    }

    #[derive(Debug, Clone, Default)]
    pub struct ObservableSink {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl EventSink for ObservableSink {
        fn write(&self, event: String) {
            self.events.lock().unwrap().push(event);
        }
    }

    impl ObservableSink {
        pub fn events(&self) -> Vec<DatadogFormattedEvent> {
            let serialized_events = self.events.lock().unwrap().clone();
            serialized_events
                .into_iter()
                .map(|serialized_event| serde_json::from_str(&serialized_event).unwrap())
                .collect()
        }
    }

    #[instrument]
    pub fn first_span(first_value: &str) {
        debug!("First Span!");
        second_span();
    }

    #[instrument(fields(attr = "value"), ret)]
    pub fn second_span() -> String {
        debug!("Second Span!");
        String::from("Return Value")
    }

    pub trait SmoothyExt {
        #[allow(clippy::wrong_self_convention)]
        fn is_any_valid_id(self);
        #[allow(clippy::wrong_self_convention)]
        fn is_not_a_valid_id(self);
    }

    impl SmoothyExt for BasicAsserter<DatadogTraceId> {
        fn is_any_valid_id(self) {
            self.is_not(DatadogTraceId(0));
        }

        fn is_not_a_valid_id(self) {
            self.is(DatadogTraceId(0));
        }
    }

    impl SmoothyExt for BasicAsserter<DatadogSpanId> {
        fn is_any_valid_id(self) {
            self.is_not(DatadogSpanId(0));
        }

        fn is_not_a_valid_id(self) {
            self.is(DatadogSpanId(0));
        }
    }
}
