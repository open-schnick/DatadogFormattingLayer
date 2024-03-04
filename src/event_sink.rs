use std::io::{stdout, Write};

/// Something that can produce any sink for events
pub trait EventSink {
    /// Write an event to the sink
    fn write(&self, event: String);
}

/// Default sink. Writes the messages to stdout
#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct StdoutSink;

impl EventSink for StdoutSink {
    fn write(&self, event: String) {
        #[allow(clippy::unwrap_used)]
        stdout().write_all(event.as_bytes()).unwrap();
    }
}
