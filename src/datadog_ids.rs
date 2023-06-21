use opentelemetry::trace::{SpanId, TraceContextExt, TraceId};
use tracing::Subscriber;
use tracing_opentelemetry::OtelData;
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, SpanRef},
};

#[derive(Debug, serde::Serialize)]
pub struct DatadogId(u64);

pub struct DatadogIds {
    pub trace_id: DatadogId,
    pub span_id: DatadogId,
}

impl From<TraceId> for DatadogId {
    fn from(value: TraceId) -> Self {
        let bytes = &value.to_bytes()[std::mem::size_of::<u64>()..std::mem::size_of::<u128>()];
        Self(u64::from_be_bytes(bytes.try_into().unwrap()))
    }
}

impl From<SpanId> for DatadogId {
    fn from(value: SpanId) -> Self {
        Self(u64::from_be_bytes(value.to_bytes()))
    }
}

pub fn read_from_context<S>(ctx: &Context<'_, S>) -> Option<DatadogIds>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    ctx.lookup_current()
        .and_then(|span_ref| lookup_datadog_ids(&span_ref))
}

fn lookup_datadog_ids<S>(span_ref: &SpanRef<S>) -> Option<DatadogIds>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    span_ref.extensions().get::<OtelData>().map(|o| DatadogIds {
        trace_id: o.parent_cx.span().span_context().trace_id().into(),
        span_id: o.builder.span_id.unwrap_or(SpanId::INVALID).into(),
    })
}
