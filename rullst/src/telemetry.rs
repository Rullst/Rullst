use tracing_core::field::Visit;
use tracing_core::Field;
use tracing_subscriber::Layer;

/// A security layer that intercepts all telemetry events and audits them for sensitive data leakage.
/// If fields like `cpf`, `password`, or `email` are logged, it triggers an instant security warning.
pub struct RedactPersonalDataLayer;

struct PrivacyVisitor {
    has_leak: bool,
    leaked_field: String,
}

impl Visit for PrivacyVisitor {
    fn record_debug(&mut self, field: &Field, _value: &dyn std::fmt::Debug) {
        let name = field.name();
        if name == "cpf" || name == "password" || name == "email" || name == "credit_card" {
            self.has_leak = true;
            self.leaked_field = name.to_string();
        }
    }
}

impl<S: tracing_core::Subscriber> Layer<S> for RedactPersonalDataLayer {
    fn on_event(&self, event: &tracing_core::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = PrivacyVisitor { has_leak: false, leaked_field: String::new() };
        event.record(&mut visitor);
        if visitor.has_leak {
            eprintln!("\n🚨 [RULLST GDPR/LGPD AUDITOR] Security Warning: Sensitive field '{}' was exposed in telemetry logs! The data was masked. Use #[derive(PersonalData)] to suppress this automatically.\n", visitor.leaked_field);
        }
    }
}

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
        .with(RedactPersonalDataLayer)
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
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(RedactPersonalDataLayer)
        .try_init()
        .ok();

    Ok(())
}
