use datadog_formatting_layer::DatadogFormattingLayer;
use opentelemetry::{
    global,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{RandomIdGenerator, Sampler},
    },
};
use opentelemetry_datadog::ApiVersion;
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::{prelude::*, util::SubscriberInitExt};

#[test]
fn works_with_otel_stack() {
    // otel boilerplate
    global::set_text_map_propagator(TraceContextPropagator::new());

    let tracer = opentelemetry_datadog::new_pipeline()
        .with_service_name("my-service")
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default()),
        )
        .with_api_version(ApiVersion::Version05)
        .with_env("rls")
        .with_version("420")
        .install_simple()
        .unwrap();

    tracing_subscriber::registry()
        .with(DatadogFormattingLayer)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    warn!("Warning");
    some_test("Fasel");
}

#[instrument(fields(hello = "world"))]
fn some_test(value: &str) {
    info!(ola = "salve", value, "Bla {value}");
    some_test1()
}

#[instrument(fields(world = "world"))]
fn some_test1() {
    debug!(ola = "salve", "Hello");
    some_test2()
}

#[instrument()]
fn some_test2() {
    error!("Oh no :(");
}
