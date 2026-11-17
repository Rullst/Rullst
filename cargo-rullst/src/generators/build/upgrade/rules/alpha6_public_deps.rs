//! The 13.0.0-alpha.6 changes that keep high-churn dependencies out of the
//! public API: `DistributedTelemetry::layer` returns an opaque layer instead
//! of `tracing_opentelemetry::OpenTelemetryLayer`,
//! `ValidationError::ValidationError` carries Rullst's `FieldErrors` instead
//! of `validator::ValidationErrors`, and Studio's OpenAPI entry points take
//! any `serde::Serialize` document instead of utoipa's `OpenApi`.
//! [`super::tokens`] calls these rules with the identifiers, paths, calls and
//! method calls it reads.
//!
//! Each is a must-change rule, so each looks for a shape that no longer
//! compiles: the layer type, or one of its own builder methods, in a file
//! that uses Rullst's distributed telemetry; the variant's `errors` field in
//! a file that names Rullst; and an OpenAPI argument whose type only the
//! former parameter gave.

use super::Rule;
use super::catalog::{STUDIO_OPENAPI_INFERRED, TELEMETRY_LAYER_TYPE, VALIDATION_FIELD_ERRORS};
use proc_macro2::{Delimiter, TokenTree};

/// The builder methods of `OpenTelemetryLayer` itself (tracing-opentelemetry
/// 0.34), which the opaque layer does not have.
const LAYER_METHODS: &[&str] = &[
    "with_tracer",
    "with_error_fields_to_exceptions",
    "with_error_events_to_status",
    "with_error_events_to_exceptions",
    "with_error_records_to_exceptions",
    "with_location",
    "with_tracked_inactivity",
    "with_threads",
    "with_level",
    "with_target",
    "with_context_activation",
    "with_counting_event_filter",
];

/// Deserializing functions whose result type the former `OpenApi`
/// parameter inferred.
const INFERRED_DESERIALIZERS: &[&str] = &["from_str", "from_slice", "from_value", "from_reader"];

fn uses_distributed_telemetry(text: &str) -> bool {
    text.contains("DistributedTelemetry") || text.contains("telemetry::distributed")
}

/// `OpenTelemetryLayer` in a file that uses Rullst's distributed telemetry.
pub(super) fn ident_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (name == "OpenTelemetryLayer" && uses_distributed_telemetry(text))
        .then_some(&TELEMETRY_LAYER_TYPE)
}

/// `ValidationError::ValidationError { errors, .. }`, as a pattern or a
/// value, given the token after the path.
pub(super) fn path_rule(
    segments: &[String],
    next: Option<&TokenTree>,
    text: &str,
) -> Option<&'static Rule> {
    let variant = segments.len() >= 2
        && segments[segments.len() - 2..]
            .iter()
            .all(|segment| segment == "ValidationError");
    let names_errors = matches!(next, Some(TokenTree::Group(fields))
        if fields.delimiter() == Delimiter::Brace
            && fields.stream().into_iter().any(|field| matches!(&field, TokenTree::Ident(name) if name == "errors")));
    (variant && names_errors && text.contains("rullst")).then_some(&VALIDATION_FIELD_ERRORS)
}

/// A builder method of `OpenTelemetryLayer` called on `.layer()`, or
/// `.with_openapi(..)` with an argument whose type was inferred.
pub(super) fn method_rule(
    name: &str,
    chain: &[String],
    args: &[TokenTree],
    text: &str,
) -> Option<&'static Rule> {
    if LAYER_METHODS.contains(&name)
        && chain.last().is_some_and(|link| link == "layer")
        && uses_distributed_telemetry(text)
    {
        return Some(&TELEMETRY_LAYER_TYPE);
    }
    (name == "with_openapi" && inferred(args)).then_some(&STUDIO_OPENAPI_INFERRED)
}

/// `api_playground::router(..)` with an argument whose type was inferred.
pub(super) fn call_rule(segments: &[&str], args: &[TokenTree]) -> Option<&'static Rule> {
    (segments.ends_with(&["api_playground", "router"]) && inferred(args))
        .then_some(&STUDIO_OPENAPI_INFERRED)
}

/// An argument that names no type of its own: `Default::default()`, a
/// `serde_json::from_*(..)` without a turbofish, or `value.into()`.
fn inferred(args: &[TokenTree]) -> bool {
    let ident = |index: usize, expected: &str| matches!(args.get(index), Some(TokenTree::Ident(name)) if name == expected);
    let call = |index: usize| matches!(args.get(index), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis);
    let default = ident(0, "Default") && ident(3, "default") && call(4);
    let deserialized = (0..args.len()).any(|index| {
        INFERRED_DESERIALIZERS
            .iter()
            .any(|function| ident(index, function))
            && call(index + 1)
    });
    let converted = args.len() >= 2
        && ident(args.len() - 2, "into")
        && call(args.len() - 1)
        && matches!(&args[..args.len() - 2], [.., TokenTree::Punct(dot)] if dot.as_char() == '.');
    default || deserialized || converted
}
