use chrono::Utc;
use std::{collections::HashMap, io::Write};
use tracing::{field::Visit, Subscriber};
use tracing_subscriber::{registry::LookupSpan, Layer};

#[derive(Default)]
pub struct DatadogFormattingLayer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for DatadogFormattingLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        // TODO: The context contains the spans which contain extentions which contain fields and stuff
        // (BUT we need to record that aswell)
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let formatted_event = self.format_event(event);
        let serialized_event = serde_json::to_string(&formatted_event).unwrap();

        // the fmt layer does some fucking magic to get a mutable ref to a generic Writer
        #[allow(clippy::explicit_write)]
        writeln!(std::io::stdout(), "{serialized_event}").unwrap();
    }
}

impl DatadogFormattingLayer {
    fn format_event(&self, event: &tracing::Event<'_>) -> DatadogFormattedEvent {
        let meta = event.metadata();
        let mut visitor = Visitor::default();
        event.record(&mut visitor);

        let mut message = visitor.fields.get("message").unwrap().to_string();

        visitor.fields.iter().for_each(|(key, value)| {
            if key != "message" {
                message.push_str(&format!(
                    " {}={}",
                    key,
                    value.trim_start_matches('\"').trim_end_matches('\"')
                ))
            }
        });

        // for span in ctx
        //     .event_scope(event)
        //     .into_iter()
        //     .flat_map(registry::Scope::from_root)
        // {
        //     let exts = span.extensions();
        //     let maybe_exts = exts.get::<FormattedFields<String>>();
        //     println!("FIELDS: {:?}", maybe_exts);
        //     if let Some(fields) = maybe_exts {
        //         if !fields.is_empty() {
        //             println!("FIELDS: {:?}", fields);
        //         }
        //     }
        // }
        //

        // IDEA: maybe loggerName instead of target
        //
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
pub struct DatadogFormattedEvent {
    timestamp: String,
    level: String,
    message: String,
    target: String,
}
