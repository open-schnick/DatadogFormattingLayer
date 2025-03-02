#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::unwrap_in_result
)]

use datadog_formatting_layer::EventSink;
use std::sync::{Arc, Mutex};

mod otel;
mod simple;

#[derive(Debug, Clone, Default)]
struct ObservableSink {
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
