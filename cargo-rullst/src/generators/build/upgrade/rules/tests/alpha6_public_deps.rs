//! 13.0.0-alpha.6 keeps high-churn dependencies out of the public API: an
//! opaque telemetry layer, `FieldErrors` in `ValidationError`, and Studio's
//! OpenAPI entry points that take any `Serialize` document.

use super::*;

const LAYER: &str = "V13-TELEMETRY-LAYER-TYPE";
const FIELDS: &str = "V13-VALIDATION-FIELD-ERRORS";
const OPENAPI: &str = "V13-STUDIO-OPENAPI-INFERRED";

/// The lines at which `code` is found in `source`.
fn found(source: &str, code: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn the_rules_are_alpha6_must_change_rules() {
    for (code, replacement) in [
        (LAYER, "impl tracing_subscriber::Layer<S>"),
        (FIELDS, "errors.messages()"),
        (OPENAPI, "utoipa::openapi::OpenApi::default()"),
    ] {
        let rule = CATALOG.iter().find(|rule| rule.code == code).unwrap();
        assert_eq!(rule.introduced, "13.0.0-alpha.6", "{code}");
        assert_eq!(rule.kind, FindingKind::MustChange, "{code}");
        assert!(rule.message.contains("13.0.0-alpha.6"), "{code}");
        assert!(rule.guidance.contains(replacement), "{code}");
    }
}

#[test]
fn the_named_layer_type_and_its_own_builder_methods_must_change() {
    let source = r#"
use rullst::telemetry::distributed::DistributedTelemetry;
use tracing_opentelemetry::OpenTelemetryLayer;
fn layer(runtime: &DistributedTelemetry) -> OpenTelemetryLayer<Registry, SdkTracer> {
    runtime.layer()
}
fn compose(runtime: &DistributedTelemetry) {
    let located = runtime.layer().with_location(false);
    let threads = runtime.layer().with_threads(false).with_level(false);
    let filtered = runtime.layer().with_filter(LevelFilter::INFO);
    let boxed = runtime.layer().boxed();
    let unrelated = other.with_location(false);
}
"#;
    assert_eq!(found(source, LAYER), [3, 4, 8, 9]);

    // An application's own OpenTelemetry pipeline, in a file that does not
    // use Rullst's distributed telemetry.
    let own = r#"
use tracing_opentelemetry::OpenTelemetryLayer;
fn own(layer: OpenTelemetryLayer<Registry, Tracer>) -> OpenTelemetryLayer<Registry, Tracer> {
    tracing_opentelemetry::layer().with_location(false)
}
"#;
    assert!(found(own, LAYER).is_empty());
}

#[test]
fn the_errors_of_a_validation_failure_must_change() {
    let source = r#"
use rullst::ValidationError;
fn read(error: ValidationError) {
    match error {
        ValidationError::ValidationError { errors, .. } => errors.field_errors(),
        ValidationError::ExtractionError { message, .. } => {}
        _ => {}
    }
    if let rullst::validation::ValidationError::ValidationError { errors: found, is_htmx } = &error {}
    let rejected = ValidationError::ValidationError { errors: validator::ValidationErrors::new(), is_htmx: false };
    let failed = matches!(error, ValidationError::ValidationError { .. });
}
"#;
    assert_eq!(found(source, FIELDS), [5, 9, 10]);

    // A file that never names Rullst keeps its own enum of that name.
    let own = r#"
enum ValidationError { ValidationError { errors: Vec<String> } }
fn read(error: ValidationError) -> Vec<String> {
    let ValidationError::ValidationError { errors } = error;
    errors
}
"#;
    assert!(found(own, FIELDS).is_empty());
}

#[test]
fn openapi_arguments_typed_only_by_the_former_parameter_must_change() {
    let source = r#"
fn studio(text: &str, document: Document) {
    let a = Studio::new().with_openapi(Default::default());
    let b = Studio::new().with_openapi(serde_json::from_str(text)?);
    let c = rullst::studio::api_playground::router(document.into());
    let d = Studio::new().with_openapi(serde_json::from_value(value)?);
    let e = Studio::new().with_openapi(ApiDoc::openapi());
    let f = Studio::new().with_openapi(serde_json::from_str::<serde_json::Value>(text)?);
    let g = api_playground::router(utoipa::openapi::OpenApi::default());
    let h = Studio::new().with_openapi(document);
    let i = other::router(Default::default());
}
"#;
    assert_eq!(found(source, OPENAPI), [3, 4, 5, 6]);
}
