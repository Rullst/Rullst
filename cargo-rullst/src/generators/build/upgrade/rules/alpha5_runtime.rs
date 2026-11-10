//! The 13.0.0-alpha.5 runtime changes that application code can name:
//! `OtaManager`'s private state fields and dynamic Alpine.js directive values
//! in `html!`. [`super::tokens`] calls these with the field accesses and
//! `html!` attribute names it reads.

use super::Rule;
use super::catalog::{HTML_ALPINE_DIRECTIVE, IOT_OTA_STATE};

/// The fields that are now read with methods of the same names.
const OTA_FIELDS: &[&str] = &["current_partition", "status", "firmware_version"];

/// The rule for a field access `.name` (not a method call) in a file whose
/// text names `OtaManager`.
pub(super) fn field_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (OTA_FIELDS.contains(&name) && text.contains("OtaManager")).then_some(&IOT_OTA_STATE)
}

/// The rule for an `html!` attribute `name={value}`: an Alpine.js directive
/// (`x-*`, also `data-` prefixed), whose dynamic value no longer compiles.
pub(super) fn html_attribute_rule(name: &str) -> Option<&'static Rule> {
    let name = name.to_ascii_lowercase();
    let unprefixed = name.strip_prefix("data-").unwrap_or(&name);
    unprefixed
        .starts_with("x-")
        .then_some(&HTML_ALPINE_DIRECTIVE)
}
