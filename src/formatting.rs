use crate::{
    datadog_ids::{DatadogSpanId, DatadogTraceId},
    fields::FieldPair,
};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::fmt::Write;
use tracing::Level;

/// All the data required to create a Datadog-compatible log
#[cfg_attr(test, derive(Debug, Clone))]
pub struct DatadogLog {
    pub timestamp: DateTime<Utc>,
    pub level: Level,
    pub message: String,
    pub fields: Vec<FieldPair>,
    pub target: String,
    pub datadog_ids: Option<(DatadogTraceId, DatadogSpanId)>,
}

impl DatadogLog {
    pub fn format(mut self) -> String {
        let mut log = Map::new();

        log.insert("timestamp".to_string(), self.timestamp.to_rfc3339().into());
        log.insert("level".to_string(), self.level.to_string().into());

        self.fields.sort();

        let mut message = self.message;

        for field in &self.fields {
            // message is just a regular field
            if field.name != "message" {
                let value = field.value.trim_matches('\"');

                write!(message, " {}={}", field.name, value).expect("Failed to write to message");
                log.insert(format!("fields.{}", &field.name), value.into());
            }
        }

        log.insert("message".to_string(), message.into());
        // IDEA: maybe loggerName instead of target
        log.insert("target".to_string(), self.target.into());

        if let Some((trace_id, span_id)) = self.datadog_ids {
            log.insert("dd.trace_id".to_string(), trace_id.0.into());
            log.insert("dd.span_id".to_string(), span_id.0.into());
        }

        let json = Value::Object(log);

        serde_json::to_string(&json)
            .unwrap_or_else(|err| format!("Failed to serialize a log to json: {err}"))
    }
}

#[cfg(test)]
mod format {
    use super::*;
    use crate::timestamp;
    use serde_json::json;
    use smoothy::assert_that;

    #[test]
    fn different_levels() {
        let trace = DatadogLog {
            timestamp: timestamp!("2022-01-01T00:00:00Z"),
            level: Level::TRACE,
            message: "Hello World!".to_string(),
            fields: vec![],
            target: "target".to_string(),
            datadog_ids: None,
        };

        assert_that(trace.clone().format()).contains("\"level\":\"TRACE\"");

        let debug = DatadogLog {
            level: Level::DEBUG,
            ..trace
        };
        assert_that(debug.clone().format()).contains("\"level\":\"DEBUG\"");

        let info = DatadogLog {
            level: Level::INFO,
            ..debug
        };
        assert_that(info.clone().format()).contains("\"level\":\"INFO\"");

        let warn = DatadogLog {
            level: Level::WARN,
            ..info
        };
        assert_that(warn.clone().format()).contains("\"level\":\"WARN\"");

        let error = DatadogLog {
            level: Level::ERROR,
            ..warn
        };
        assert_that(error.format()).contains("\"level\":\"ERROR\"");
    }

    #[test]
    fn without_fields() {
        let sut = DatadogLog {
            timestamp: timestamp!("2022-01-01T00:00:00Z"),
            level: Level::INFO,
            message: "Hello World!".to_string(),
            fields: vec![],
            target: "target".to_string(),
            datadog_ids: None,
        };

        assert_that(sut.format()).is(json!({"timestamp": "2022-01-01T00:00:00+00:00", "level": "INFO", "message": "Hello World!", "target": "target"}).to_string());
    }

    #[test]
    fn with_datadog_ids() {
        let sut = DatadogLog {
            timestamp: timestamp!("2022-01-01T00:00:00Z"),
            level: Level::INFO,
            message: "Hello World!".to_string(),
            fields: vec![],
            target: "target".to_string(),
            datadog_ids: Some((DatadogTraceId(1), DatadogSpanId(2))),
        };

        assert_that(sut.format()).is(json!({"timestamp": "2022-01-01T00:00:00+00:00", "level": "INFO", "message": "Hello World!", "target": "target", "dd.trace_id": 1, "dd.span_id": 2}).to_string());
    }

    #[test]
    fn with_field() {
        let fields = vec![FieldPair {
            name: "foo".to_string(),
            value: "bar".to_string(),
        }];

        let sut = DatadogLog {
            timestamp: timestamp!("2022-01-01T00:00:00Z"),
            level: Level::INFO,
            message: "Hello World!".to_string(),
            fields,
            target: "target".to_string(),
            datadog_ids: None,
        };

        assert_that(sut.format()).is(json!({"timestamp": "2022-01-01T00:00:00+00:00", "level": "INFO", "fields.foo": "bar", "message": "Hello World! foo=bar", "target": "target"}).to_string());
    }

    #[test]
    fn multiple_fields_are_sorted_by_name_and_inlined_in_the_message() {
        let fields = vec![
            FieldPair {
                name: "a".to_string(),
                value: "c".to_string(),
            },
            FieldPair {
                name: "b".to_string(),
                value: "b".to_string(),
            },
            FieldPair {
                name: "c".to_string(),
                value: "a".to_string(),
            },
        ];

        let sut = DatadogLog {
            timestamp: timestamp!("2022-01-01T00:00:00Z"),
            level: Level::INFO,
            message: "Hello World!".to_string(),
            fields,
            target: "target".to_string(),
            datadog_ids: None,
        };

        assert_that(sut.format()).is(json!({"timestamp": "2022-01-01T00:00:00+00:00", "level": "INFO", "fields.a": "c", "fields.b": "b", "fields.c": "a", "message": "Hello World! a=c b=b c=a", "target": "target"}).to_string());
    }

    #[macro_export]
    macro_rules! timestamp {
        ($date:expr) => {
            DateTime::parse_from_rfc3339($date)
                .unwrap()
                .with_timezone(&Utc)
        };
    }
}
