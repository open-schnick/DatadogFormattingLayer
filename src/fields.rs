use std::{cmp::Ordering, collections::HashMap};
use tracing::{field::Visit, span::Attributes, Event, Subscriber};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, Scope},
};

#[derive(Debug, Clone)]
pub struct FieldStore {
    pub fields: Vec<FieldPair>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPair {
    pub name: String,
    pub value: String,
}

impl PartialOrd for FieldPair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FieldPair {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

pub fn from_attributes(attrs: &Attributes<'_>) -> Vec<FieldPair> {
    let mut visitor = Visitor::default();
    attrs.values().record(&mut visitor);

    visitor
        .fields
        .into_iter()
        .map(|(key, value)| FieldPair { name: key, value })
        .collect()
}

pub fn from_event(event: &Event<'_>) -> Vec<FieldPair> {
    let mut visitor = Visitor::default();
    event.record(&mut visitor);

    visitor
        .fields
        .into_iter()
        .map(|(key, value)| FieldPair { name: key, value })
        .collect()
}

pub fn from_spans<S: Subscriber + for<'a> LookupSpan<'a>>(
    ctx: &Context<'_, S>,
    event: &Event<'_>,
) -> Vec<FieldPair> {
    ctx.event_scope(event)
        .into_iter()
        .flat_map(Scope::from_root)
        .flat_map(|span| {
            #[allow(clippy::expect_used)]
            let fields_from_span = span
                .extensions()
                .get::<FieldStore>()
                .expect("No Fields found in span extensions")
                .clone();

            fields_from_span.fields
        })
        .collect()
}

#[derive(Default)]
struct Visitor {
    fields: HashMap<String, String>,
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}
