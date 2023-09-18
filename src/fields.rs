use std::collections::HashMap;
use tracing::{field::Visit, span::Attributes};

pub type FieldPair = (String, String);

#[derive(Debug, Clone)]
pub struct Fields {
    pub fields: Vec<FieldPair>,
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

pub fn from_attributes(attrs: &Attributes<'_>) -> Vec<FieldPair> {
    let mut visitor = Visitor::default();
    attrs.values().record(&mut visitor);

    visitor
        .fields
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<FieldPair>>()
}

pub fn from_event(event: &tracing::Event<'_>) -> Vec<FieldPair> {
    let mut visitor = Visitor::default();
    event.record(&mut visitor);

    visitor
        .fields
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<FieldPair>>()
}
