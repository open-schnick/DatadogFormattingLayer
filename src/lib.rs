use std::{io::Write, time::SystemTime};
use tracing::{Level, Subscriber};
use tracing_subscriber::{registry::LookupSpan, Layer};

pub struct DatadogFormattingLayer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for DatadogFormattingLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        // The context contains the spans which contain extentions which contain fields and stuff
        // (BUT we need to record that aswell)
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let formatted_event = self.format_event(event);
        let serialized_event = serde_json::to_string(&formatted_event).unwrap();

        // creating a new handle might not be the best thing
        // the fmt layer does some fucking magic
        writeln!(std::io::stdout(), "{serialized_event}").unwrap();
    }
}

impl DatadogFormattingLayer {
    fn format_event(&self, event: &tracing::Event<'_>) -> DatadogFormattedEvent {
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
        event.record
        todo!()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DatadogFormattedEvent {
    timestamp: SystemTime,
    level: String,
}
