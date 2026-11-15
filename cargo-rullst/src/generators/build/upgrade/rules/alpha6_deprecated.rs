//! The APIs 13.0.0-alpha.6 removed because they were deprecated: in 12.0.0
//! (Mail, IoT, Nexus, AI, Security, Core's queue, the Connect providers'
//! `new` and the `#[route]` marker) or in 13.0.0-alpha.5
//! (`rullst_orm::schema_diff`). [`super::tokens`] calls the path, identifier
//! and method rules with what it reads, and [`super::rust`] the path rule
//! with each `use` path and the attribute rule with each function attribute.
//!
//! Distinctive paths such as `ResendDriver::new` are must-change rules. A
//! method with a common name (`.build()`, `.is_safe()`, `.commit_update(..)`)
//! counts only in a file that names its type, and its rule is a review rule,
//! because another type in that file can have a method of that name.

use super::Rule;
use super::catalog::{
    AI_DEPRECATED_REMOVED, CONNECT_INFALLIBLE_NEW_REMOVED, IOT_OTA_LEGACY_REMOVED,
    MAIL_DEPRECATED_REMOVED, NEXUS_INFALLIBLE_BUILD_REMOVED, ORM_SCHEMA_DIFF_REMOVED,
    QUEUE_PURGE_COMPLETED_REMOVED, ROUTE_MARKER_REMOVED, SECURITY_DEPRECATED_REMOVED,
};

/// Removed associated functions and variants, as `Type::name` pairs.
const REMOVED_PAIRS: &[(&str, &str, &Rule)] = &[
    ("ResendDriver", "new", &MAIL_DEPRECATED_REMOVED),
    ("AwsSesDriver", "new", &MAIL_DEPRECATED_REMOVED),
    (
        "TrackingEngine",
        "generate_open_token",
        &MAIL_DEPRECATED_REMOVED,
    ),
    (
        "TrackingEngine",
        "generate_click_token",
        &MAIL_DEPRECATED_REMOVED,
    ),
    (
        "TrackingEngine",
        "inject_open_pixel",
        &MAIL_DEPRECATED_REMOVED,
    ),
    ("TrackingEngine", "rewrite_links", &MAIL_DEPRECATED_REMOVED),
    ("OtaManager", "new", &IOT_OTA_LEGACY_REMOVED),
    ("OtaError", "LegacyApiUnsupported", &IOT_OTA_LEGACY_REMOVED),
    ("Nexus", "build", &NEXUS_INFALLIBLE_BUILD_REMOVED),
    ("Nexus", "into_router", &NEXUS_INFALLIBLE_BUILD_REMOVED),
    ("AuditChain", "new", &SECURITY_DEPRECATED_REMOVED),
    (
        "Queue",
        "purge_completed_jobs",
        &QUEUE_PURGE_COMPLETED_REMOVED,
    ),
    ("CredentialMode", "Invalid", &CONNECT_INFALLIBLE_NEW_REMOVED),
    ("rullst_orm", "schema_diff", &ORM_SCHEMA_DIFF_REMOVED),
    ("orm", "schema_diff", &ORM_SCHEMA_DIFF_REMOVED),
    ("rullst", "route", &ROUTE_MARKER_REMOVED),
    ("rullst_core", "route", &ROUTE_MARKER_REMOVED),
    ("rullst_macros", "route", &ROUTE_MARKER_REMOVED),
];

/// The built-in Connect providers whose infallible `new` was removed. An
/// application can give its own type such a name, so they count only in a
/// file that names Connect.
const CONNECT_PROVIDERS: &[&str] = &[
    "AppleProvider",
    "Auth0Provider",
    "CognitoProvider",
    "DiscordProvider",
    "FacebookProvider",
    "GithubProvider",
    "GoogleProvider",
    "LinkedinProvider",
    "MicrosoftProvider",
    "XProvider",
];

fn names_connect(text: &str) -> bool {
    text.contains("rullst_connect") || text.contains("connect::")
}

fn names_mail(text: &str) -> bool {
    text.contains("rullst_mail") || text.contains("mail::")
}

fn names_ai(text: &str) -> bool {
    text.contains("rullst_ai") || text.contains("ai::") || text.contains("AiClient")
}

/// The rule for a path such as `ResendDriver::new` or
/// `rullst_orm::schema_diff`, given the file's text, if any.
pub(super) fn path_rule(segments: &[String], text: &str) -> Option<&'static Rule> {
    segments.windows(2).find_map(|pair| {
        let (left, right) = (pair[0].as_str(), pair[1].as_str());
        if right == "new" && CONNECT_PROVIDERS.contains(&left) && names_connect(text) {
            return Some(&CONNECT_INFALLIBLE_NEW_REMOVED);
        }
        REMOVED_PAIRS
            .iter()
            .find(|(ty, name, _)| *ty == left && *name == right)
            .map(|(_, _, rule)| *rule)
    })
}

/// The rule for an identifier, given the file's text, if any.
pub(super) fn ident_rule(name: &str, text: &str) -> Option<&'static Rule> {
    match name {
        "with_open_tracking" | "with_click_tracking" if names_mail(text) => {
            Some(&MAIL_DEPRECATED_REMOVED)
        }
        "LegacyApiUnsupported" if text.contains("OtaError") || text.contains("rullst_iot") => {
            Some(&IOT_OTA_LEGACY_REMOVED)
        }
        // Also `client.structured_prompt::<T>(..)`, which is not a plain call.
        "structured_prompt" if names_ai(text) => Some(&AI_DEPRECATED_REMOVED),
        "extract_tables_from_ast" | "ParsedTable" | "ParsedField"
            if text.contains("schema_diff") =>
        {
            Some(&ORM_SCHEMA_DIFF_REMOVED)
        }
        _ => None,
    }
}

/// The rule for a method call `.name(args)`, given whether it has no
/// arguments and the file's text, if any.
pub(super) fn method_rule(name: &str, no_arguments: bool, text: &str) -> Option<&'static Rule> {
    match name {
        "verify_signature" | "commit_update" if text.contains("OtaManager") => {
            Some(&IOT_OTA_LEGACY_REMOVED)
        }
        "build" | "into_router" if no_arguments && text.contains("Nexus") => {
            Some(&NEXUS_INFALLIBLE_BUILD_REMOVED)
        }
        "is_safe" if no_arguments && text.contains("Guardrail") => Some(&AI_DEPRECATED_REMOVED),
        "with_distributed" if no_arguments && text.contains("RateLimiter") => {
            Some(&SECURITY_DEPRECATED_REMOVED)
        }
        "purge_completed_jobs" if text.contains("Queue") => Some(&QUEUE_PURGE_COMPLETED_REMOVED),
        _ => None,
    }
}

/// The rule for a function attribute: `#[rullst::route]` (also through
/// `rullst_core` or `rullst_macros`) or, in a file that names Rullst, a bare
/// `#[route]` without arguments. Other frameworks' `#[route(...)]` attributes
/// take arguments, and Rullst's marker refused them.
pub(super) fn attribute_rule(attribute: &syn::Attribute, text: &str) -> Option<&'static Rule> {
    let segments: Vec<String> = attribute
        .path()
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    match segments.as_slice() {
        [name] if name == "route" => (matches!(attribute.meta, syn::Meta::Path(_))
            && text.contains("rullst"))
        .then_some(&ROUTE_MARKER_REMOVED),
        [.., scope, name]
            if name == "route"
                && matches!(scope.as_str(), "rullst" | "rullst_core" | "rullst_macros") =>
        {
            Some(&ROUTE_MARKER_REMOVED)
        }
        _ => None,
    }
}
