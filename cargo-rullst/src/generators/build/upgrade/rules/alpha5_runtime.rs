//! The 13.0.0-alpha.5 runtime changes that application code can name:
//! `OtaManager`'s private state fields, dynamic Alpine.js directive values
//! in `html!`, the IoT codec errors that became `#[non_exhaustive]` and the IoT
//! sensor card that returns `IotHtml`. [`super::tokens`] calls the field,
//! attribute and identifier rules with what it reads, [`super::rust`] the
//! match rule with each `match`.

use super::Rule;
use super::catalog::{
    HTML_ALPINE_DIRECTIVE, IOT_CODEC_ERROR_MATCH, IOT_OTA_STATE, IOT_SENSOR_CARD_HTML,
};

/// The `rullst-iot` error enums 13.0.0-alpha.5 made `#[non_exhaustive]`.
const IOT_CODEC_ERRORS: &[&str] = &["MqttCodecError", "CoapCodecError"];

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

/// The rule for a `match` without a catch-all arm, given whether its arm
/// patterns name an identifier: one of the IoT codec error enums.
pub(super) fn exhaustive_match_rule(names: impl Fn(&str) -> bool) -> Option<&'static Rule> {
    IOT_CODEC_ERRORS
        .iter()
        .any(|name| names(name))
        .then_some(&IOT_CODEC_ERROR_MATCH)
}

/// The rule for an identifier: `render_sensor_card`, whose return type
/// changed from `String` to `IotHtml`.
pub(super) fn ident_rule(name: &str) -> Option<&'static Rule> {
    (name == "render_sensor_card").then_some(&IOT_SENSOR_CARD_HTML)
}
