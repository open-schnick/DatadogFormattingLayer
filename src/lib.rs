//! # Datadog Formatting Layer
//!
//! A crate providing a tracing-subscriber layer for formatting events so Datadog can parse them.
//!
//! ## Features
//! - Provides a layer for tracing-subscriber
//! - Generates parsable "logs" for datadog and prints them to stdout
//! - Enables log correlation between spans and "logs" (see [datadog docs](https://docs.datadoghq.com/tracing/other_telemetry/connect_logs_and_traces/))
//!
//! ## Why not just `tracing_subscriber::fmt().json()` ?
//! The problem is, that datadog expects the "logs" to be in a specific (mostly undocumented) json format.
//!
//! This crates tries to mimic this format.
//!
//! ## Usage
//!
//! ### Simple
//!
//! ```
//! use datadog_formatting_layer::DatadogFormattingLayer;
//! use tracing::info;
//! use tracing_subscriber::prelude::*;
//!
//! tracing_subscriber::registry()
//!     .with(DatadogFormattingLayer)
//!     .init();
//!
//! info!(user = "Jack", "Hello World!");
//! ```
//!
//! Running this code will result in the following output on stdout:
//!
//! ```json
//! {"timestamp":"2023-06-21T10:36:50.364874878+00:00","level":"INFO","message":"Hello World user=Jack","target":"simple"}
//! ```
//!
//! ### With Opentelemetry
//! ```
//! use datadog_formatting_layer::DatadogFormattingLayer;
//! use opentelemetry::{
//!     global,
//!     sdk::{
//!         propagation::TraceContextPropagator,
//!         trace::{RandomIdGenerator, Sampler},
//!     },
//! };
//! use opentelemetry_datadog::ApiVersion;
//! use tracing::{debug, error, info, instrument, warn};
//! use tracing_subscriber::{prelude::*, util::SubscriberInitExt};
//!
//! // Just some otel boilerplate
//! global::set_text_map_propagator(TraceContextPropagator::new());
//!
//! let tracer = opentelemetry_datadog::new_pipeline()
//!     .with_service_name("my-service")
//!     .with_trace_config(
//!         opentelemetry::sdk::trace::config()
//!             .with_sampler(Sampler::AlwaysOn)
//!             .with_id_generator(RandomIdGenerator::default()),
//!     )
//!     .with_api_version(ApiVersion::Version05)
//!     .with_env("rls")
//!     .with_version("420")
//!     .install_simple()
//!     .unwrap();
//!
//! // Use both the tracer and the formatting layer
//! tracing_subscriber::registry()
//!     .with(DatadogFormattingLayer)
//!     .with(tracing_opentelemetry::layer().with_tracer(tracer))
//!     .init();
//!
//! // Here no span exists
//! info!(user = "Jack", "Hello World!");
//! some_test("fasel");
//!
//! // This will create a span and a trace id which is attached to the "logs"
//! #[instrument(fields(hello = "world"))]
//! fn some_test(value: &str) {
//!     // Here some span exists
//!     info!(ola = "salve", value, "Bla {value}");
//! }
//! ```
//!
//! When running this code with an datadog agent installed the logs will be sent to datadog
//! and parsed there.
//!
//! Otherwise the following output will be printed to stdout
//!
//! ```json
//! {"timestamp":"2023-06-21T10:36:50.363224217+00:00","level":"INFO","message":"Hello World! user=Jack","target":"otel"}
//! {"timestamp":"2023-06-21T10:36:50.363384118+00:00","level":"INFO","message":"Bla fasel user=Jack ola=salve value=Fasel hello=world","target":"otel","dd.trace_id":0,"dd.span_id":10201226522570980512}
//! ```
use chrono::Utc;
use datadog_ids::{DatadogId, DatadogIds};
use fields::{FieldPair, Fields};
use std::io::Write;
use tracing::{span::Attributes, Event, Id, Metadata, Subscriber};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, Scope},
    Layer,
};

mod datadog_ids;
mod fields;

/// The layer responsible for formatting tracing events in a way datadog can parse them
#[derive(Default)]
pub struct DatadogFormattingLayer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for DatadogFormattingLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
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
            let exts = span.extensions();
            let fields_from_span = exts.get::<Fields>().unwrap().to_owned();
            all_fields.extend(fields_from_span.fields)
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
        let formatted_event = self.format_event(message, event.metadata(), fields, datadog_ids);
        let serialized_event = serde_json::to_string(&formatted_event).unwrap();

        // the fmt layer does some magic to get a mutable ref to a generic Writer
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
        datadog_ids: Option<DatadogIds>,
    ) -> DatadogFormattedEvent {
        fields.iter().for_each(|(key, value)| {
            // message is just a regular field
            if key != "message" {
                message.push_str(&format!(" {}={}", key, value.trim_matches('\"')))
            }
        });

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
