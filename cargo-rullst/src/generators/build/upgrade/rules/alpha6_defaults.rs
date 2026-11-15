//! The 13.0.0-alpha.6 default changes that application code can name: the
//! health probes no longer report the framework version. [`super::tokens`]
//! calls the field and string rules with what it reads, [`super::rust`] the
//! struct rule with each struct literal and [`super::files`] the script rule
//! with each script line.

use super::Rule;
use super::catalog::{HEALTH_FRAMEWORK_VERSION, HEALTH_PROBE_VERSION_FIELD};

/// The probe payload types whose `version` field 13.0.0-alpha.6 removed.
const PROBE_TYPES: &[&str] = &["HealthProbeResponse", "ReadinessProbeResponse"];

fn names_probe_type(text: &str) -> bool {
    PROBE_TYPES.iter().any(|name| text.contains(name))
}

/// The rule for a field access `.name` (not a method call): `.version` in a
/// file that names a probe payload type.
pub(super) fn field_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (name == "version" && names_probe_type(text)).then_some(&HEALTH_PROBE_VERSION_FIELD)
}

/// The rule for a struct literal whose path ends in `last`: a
/// `HealthProbeResponse`, which is `#[non_exhaustive]` and has no `version`.
pub(super) fn struct_rule(last: &str) -> Option<&'static Rule> {
    (last == "HealthProbeResponse").then_some(&HEALTH_PROBE_VERSION_FIELD)
}

/// The rule for a string literal: the key `"version"` in a file that also
/// names a probe path in a string, such as a test that reads
/// `body["version"]` after `get("/health")`.
pub(super) fn string_rule(value: &str, text: &str) -> Option<&'static Rule> {
    (value == "version" && (text.contains("\"/health") || text.contains("\"/ready")))
        .then_some(&HEALTH_FRAMEWORK_VERSION)
}

/// A script line that reads `version` from a `/health` or `/ready` response
/// on the same line, such as `curl -fsS $URL/health | jq -r .version`.
pub(super) fn reads_probe_version(line: &str) -> bool {
    let probe = line.contains("/health") || line.contains("/ready");
    let field = line.match_indices(".version").any(|(index, found)| {
        line[index + found.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '_'))
    });
    probe && (field || line.contains("\"version\"") || line.contains("'version'"))
}
