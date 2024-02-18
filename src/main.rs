mod api;

use anyhow::{Context, Result};
use configured::Configured;
use error_ext::StdErrorExt;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, runtime, trace, Resource};
use serde::Deserialize;
use serde_json::json;
use std::{fmt::Display, panic};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::{error, info, Subscriber};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt, EnvFilter, Layer,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first, because needed for tracing initialization.
    let config = Config::load()
        .context("load configuration")
        .inspect_err(log_error)?;

    // Initialize tracing.
    init_tracing(config.tracing.clone()).inspect_err(log_error)?;

    // Replace the default panic hook with one that uses structured logging at ERROR level.
    panic::set_hook(Box::new(|panic| error!(%panic, "process panicked")));

    // Run and log any error.
    run(config).await.inspect_err(|error| {
        error!(
            error = format!("{error:#}"),
            backtrace = %error.backtrace(),
            "process exited with ERROR"
        )
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    api: api::Config,
    tracing: TracingConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TracingConfig {
    service_name: String,
    otlp_exporter_endpoint: String,
}

/// Initialize tracing: apply an `EnvFilter` using the `RUST_LOG` environment variable to define the
/// log levels, add a formatter layer logging trace events as JSON and an OpenTelemetry layer
/// exporting trace data.
fn init_tracing(config: TracingConfig) -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    global::set_error_handler(|error| error!(error = error.as_chain(), "otel error"))
        .context("set error handler")?;

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().json().flatten_event(true))
        .with(otlp_layer(config)?)
        .try_init()
        .context("initialize tracing subscriber")
}

/// Create an OTLP layer exporting tracing data.
fn otlp_layer<S>(config: TracingConfig) -> Result<impl Layer<S>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(config.otlp_exporter_endpoint);

    let trace_config = trace::config().with_resource(Resource::new(vec![KeyValue::new(
        "service.name",
        config.service_name,
    )]));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(runtime::Tokio)
        .context("install tracer")?;

    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

fn log_error(error: &impl Display) {
    let now = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
    let error = serde_json::to_string(&json!({
        "timestamp": now,
        "level": "ERROR",
        "message": "process exited with ERROR",
        "error": format!("{error:#}")
    }));
    // Not using `eprintln!`, because `tracing_subscriber::fmt` uses stdout by default.
    println!("{}", error.unwrap());
}

async fn run(config: Config) -> Result<()> {
    info!(?config, "starting");

    api::serve(config.api).await
}
