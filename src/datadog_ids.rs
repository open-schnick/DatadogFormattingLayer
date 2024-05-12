use opentelemetry::trace::{SpanId, TraceContextExt, TraceId};
use tracing::Subscriber;
use tracing_opentelemetry::OtelData;
use tracing_subscriber::{layer::Context, registry::LookupSpan};

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(Debug, Clone, Copy, serde::Deserialize, PartialEq, Eq))]
pub struct DatadogTraceId(pub(crate) u64);

#[allow(clippy::fallible_impl_from)]
impl From<TraceId> for DatadogTraceId {
    // TraceId are u128 -> 16 Bytes
    // but datadog needs u64 -> 8 Bytes
    // Therefore we just take the 8 most significant bytes
    // This is not ideal and may lead to duplicate trace correlations
    // but we cannot do anything against that anyways.
    fn from(value: TraceId) -> Self {
        let bytes = value.to_bytes();
        // this cannot fail
        #[allow(clippy::unwrap_used)]
        let most_significant_8_bytes = bytes.get(8..16).unwrap();

        // this also cannot fail because we checked the range one line above
        #[allow(clippy::unwrap_used)]
        let bytes_as_sized_slice: [u8; 8] = most_significant_8_bytes.try_into().unwrap();

        Self(u64::from_be_bytes(bytes_as_sized_slice))
    }
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(Debug, Clone, Copy, serde::Deserialize, PartialEq, Eq))]
pub struct DatadogSpanId(pub u64);

impl From<SpanId> for DatadogSpanId {
    fn from(value: SpanId) -> Self {
        Self(u64::from_be_bytes(value.to_bytes()))
    }
}

pub fn read_from_context<S>(ctx: &Context<'_, S>) -> (Option<DatadogTraceId>, Option<DatadogSpanId>)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let ids: Option<(DatadogTraceId, DatadogSpanId)> = ctx.lookup_current().and_then(|span_ref| {
        span_ref.extensions().get::<OtelData>().map(|o| {
            (
                o.parent_cx.span().span_context().trace_id().into(),
                o.builder.span_id.unwrap_or(SpanId::INVALID).into(),
            )
        })
    });

    ids.map_or((None, None), |ids| (Some(ids.0), Some(ids.1)))
}
