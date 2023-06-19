use chrono::Utc;
use std::{collections::HashMap, io::Write};
use tracing::{field::Visit, span::Attributes, Event, Id, Metadata, Subscriber};
use tracing_subscriber::{
    layer::Context,
    registry::{self, LookupSpan},
    Layer,
};

#[derive(Default)]
pub struct DatadogFormattingLayer;

type FieldPair = (String, String);

#[derive(Debug, Clone)]
struct Fields {
    fields: Vec<FieldPair>,
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for DatadogFormattingLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        let mut visitor = Visitor::default();
        attrs.values().record(&mut visitor);

        let fields = visitor
            .fields
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<FieldPair>>();

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
            .flat_map(registry::Scope::from_root)
        {
            let exts = span.extensions();
            let fields_from_span = exts.get::<Fields>().unwrap().to_owned();
            all_fields.extend(fields_from_span.fields)
        }

        // parse event fields
        let mut visitor = Visitor::default();
        event.record(&mut visitor);

        let message = visitor.fields.get("message").cloned().unwrap_or_default();

        let mut fields = visitor
            .fields
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<FieldPair>>();

        // reason for extending fields with all_fields is the order of fields in the message
        fields.extend(all_fields);

        // format and serialize the event metadata and fields
        let formatted_event = self.format_event(message, event.metadata(), fields);
        let serialized_event = serde_json::to_string(&formatted_event).unwrap();

        // the fmt layer does some fucking magic to get a mutable ref to a generic Writer
        #[allow(clippy::explicit_write)]
        writeln!(std::io::stdout(), "{serialized_event}").unwrap();
    }
}

impl DatadogFormattingLayer {
    fn format_event(
        &self,
        mut message: String,
        meta: &Metadata,
        fields: Vec<FieldPair>,
    ) -> DatadogFormattedEvent {
        fields.iter().for_each(|(key, value)| {
            if key != "message" {
                message.push_str(&format!(
                    " {}={}",
                    key,
                    value.trim_start_matches('\"').trim_end_matches('\"')
                ))
            }
        });

        // IDEA: maybe loggerName instead of target
        // IDEA: maybe use fields as attributes or something
        DatadogFormattedEvent {
            timestamp: Utc::now().to_rfc3339(),
            level: meta.level().to_string(),
            message,
            target: meta.target().to_string(),
        }
    }
}

#[derive(Default)]
struct Visitor {
    fields: HashMap<String, String>,
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{:?}", value));
    }
}

#[derive(Debug, serde::Serialize)]
struct DatadogFormattedEvent {
    timestamp: String,
    level: String,
    message: String,
    target: String,
}
