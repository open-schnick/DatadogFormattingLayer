use crate::{
    datadog_ids::{self, DatadogId},
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

        // FIXME: refactor this
        let ids = {
            match datadog_ids {
                Some(ids) => (Some(ids.span_id), Some(ids.trace_id)),
                None => (None, None),
            }
        };

        // IDEA: maybe loggerName instead of target
        // IDEA: maybe use fields as attributes or something
        let formatted_event = DatadogFormattedEvent {
            timestamp: Utc::now().to_rfc3339(),
            level: event.metadata().level().to_string(),
            message,
            target: event.metadata().target().to_string(),
            datadog_span_id: ids.0,
            datadog_trace_id: ids.1,
        };

        let serialized_event = serde_json::to_string(&formatted_event)
            .unwrap_or_else(|_| "Failed to serialize an event to json".to_string());

        self.event_sink.write(serialized_event);
    }
}

#[derive(Debug, serde::Serialize)]
struct DatadogFormattedEvent {
    timestamp: String,
    level: String,
    message: String,
    target: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dd.trace_id")]
    datadog_trace_id: Option<DatadogId>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dd.span_id")]
    datadog_span_id: Option<DatadogId>,
}
