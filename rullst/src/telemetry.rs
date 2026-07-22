#[cfg(feature = "telemetry")]
/// Initializes the OpenTelemetry OTLP pipeline for distributed tracing.
/// This configuration connects to a local OTLP collector on port 4317.
/// Returns a Result which can be gracefully ignored if the collector is unavailable.
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    use opentelemetry_otlp::WithExportConfig;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()?;

    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name("rullst-app")
        .build();

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());
    use opentelemetry::trace::TracerProvider as _;
    let tracer = provider.tracer("rullst");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .try_init()?;

    Ok(())
}

#[cfg(not(feature = "telemetry"))]
/// Initializes the OpenTelemetry OTLP pipeline for distributed tracing.
/// 
/// This configuration connects to a local OTLP collector on port 4317.
/// Returns a Result which can be gracefully ignored if the collector is unavailable.
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Basic tracing without OpenTelemetry
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init().ok();
    
    Ok(())
}
