use crate::{
    datadog_ids::{self, DatadogId, DatadogIds},
    event_sink::{EventSink, StdoutSink},
    fields::{self, FieldPair, Fields},
};
use chrono::Utc;
use tracing::{span::Attributes, Event, Id, Metadata, Subscriber};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, Scope},
    Layer,
};

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
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        #[allow(clippy::expect_used)]
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        let fields = fields::from_attributes(attrs);

        // insert fields from new span e.g #[instrument(fields(hello = "world"))]
        if extensions.get_mut::<Fields>().is_none() {
            extensions.insert(Fields { fields });
        }
    }

    // IDEA: maybe a on record implementation is required here

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut all_fields = vec![];

        // parse context
        for span in ctx
            .event_scope(event)
            .into_iter()
            .flat_map(Scope::from_root)
        {
            #[allow(clippy::expect_used)]
            let fields_from_span = span
                .extensions()
                .get::<Fields>()
                .expect("No Fields found in span extensions. This is a tracing bug.")
                .clone();

            all_fields.extend(fields_from_span.fields);
        }

        // parse event fields
        let mut fields = fields::from_event(event);

        // find message if present in fields
        let message = fields
            .iter()
            .find(|pair| pair.0 == "message")
            .map(|pair| pair.1.clone())
            .unwrap_or_default();

        // reason for extending fields with all_fields is the order of fields in the message
        fields.extend(all_fields);

        // look for datadog trace- and span-id
        let datadog_ids = datadog_ids::read_from_context(&ctx);

        // format and serialize the event metadata and fields
        let formatted_event = Self::format_event(message, event.metadata(), &fields, datadog_ids);
        let serialized_event = serde_json::to_string(&formatted_event)
            .unwrap_or_else(|_| "Failed to serialize an event to json".to_string());

        self.event_sink.write(serialized_event);
    }
}

impl<Sink: EventSink + 'static> DatadogFormattingLayer<Sink> {
    fn format_event(
        mut message: String,
        meta: &Metadata<'_>,
        fields: &[FieldPair],
        datadog_ids: Option<DatadogIds>,
    ) -> DatadogFormattedEvent {
        for (name, value) in fields {
            // message is just a regular field
            if name != "message" {
                message.push_str(&format!(" {}={}", name, value.trim_matches('\"')));
            }
        }

        // FIXME: refactor this
        let ids = {
            match datadog_ids {
                Some(ids) => (Some(ids.span_id), Some(ids.trace_id)),
                None => (None, None),
            }
        };

        // IDEA: maybe loggerName instead of target
        // IDEA: maybe use fields as attributes or something
        DatadogFormattedEvent {
            timestamp: Utc::now().to_rfc3339(),
            level: meta.level().to_string(),
            message,
            target: meta.target().to_string(),
            datadog_span_id: ids.0,
            datadog_trace_id: ids.1,
        }
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
