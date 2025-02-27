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
    ///
    /// # Example
    /// ```
    /// use datadog_formatting_layer::{DatadogFormattingLayer, EventSink, StdoutSink};
    ///
    /// let layer: DatadogFormattingLayer<StdoutSink> =
    ///     DatadogFormattingLayer::with_sink(StdoutSink::default());
    /// ```
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

    // IDEA: maybe an on record implementation is required here

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
