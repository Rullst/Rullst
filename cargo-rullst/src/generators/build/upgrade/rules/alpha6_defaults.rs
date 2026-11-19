//! The 13.0.0-alpha.6 default changes that application code can name: the
//! health probes no longer report the framework version, the CSRF
//! middleware decides static files by path (`csrf_static_prefixes` and
//! `csrf_static_paths`) instead of by extension, the CSRF token a page
//! embeds is masked per response, and `Server` applies the CSRF check and the
//! secure headers in every environment. [`super::tokens`] calls the field, string
//! and method rules with what it reads, [`super::rust`] the struct rule with
//! each struct literal and [`super::files`] the script rule with each script
//! line.

use super::Rule;
use super::catalog::{
    CSRF_COOKIE_IN_PAGE, CSRF_STATIC_PREFIXES, HEALTH_FRAMEWORK_VERSION,
    HEALTH_PROBE_VERSION_FIELD, ROUTER_BASELINE_LAYERS,
};
use proc_macro2::TokenStream;

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
/// names one of Rullst's probe paths in a string, such as a test that reads
/// `body["version"]` after `get("/health")`. The path ends the string
/// (`"/health"`, `"{base}/ready"`), so an application's own `/healthz` does
/// not count.
pub(super) fn string_rule(value: &str, text: &str) -> Option<&'static Rule> {
    (value == "version" && (text.contains("/health\"") || text.contains("/ready\"")))
        .then_some(&HEALTH_FRAMEWORK_VERSION)
}

/// Whether Core's default `csrf_static_prefixes` and `csrf_static_paths`
/// cover what a service mounted at `path` serves: everything below a prefix
/// (`/static`, `/static/`) or one exact file.
fn default_static(path: &str) -> bool {
    let security = rullst_core::config::SecurityConfig::default();
    security.is_csrf_static_path(&format!("{}/", path.trim_end_matches('/')))
        || security.is_csrf_static_path(path)
}

/// The rule for a method call with `args`: `.nest_service("/path", ..)` or
/// `.route_service("/path", ..)` at a path the default static-file settings
/// do not cover, in a file that names `ServeDir` or `ServeFile`.
pub(super) fn method_rule(name: &str, args: &TokenStream, text: &str) -> Option<&'static Rule> {
    if !matches!(name, "nest_service" | "route_service")
        || !(text.contains("ServeDir") || text.contains("ServeFile"))
    {
        return None;
    }
    let path = super::tokens::string_value(args.clone().into_iter().next().as_ref())?;
    (path.starts_with('/') && !default_static(&path)).then_some(&CSRF_STATIC_PREFIXES)
}

/// The rule for a method call with `args`: `.layer(..)` or `.route_layer(..)`
/// whose argument names Core's `csrf_middleware` or `headers_middleware`,
/// which `Server` applies in every environment since 13.0.0-alpha.6.
pub(super) fn layer_rule(name: &str, args: &TokenStream) -> Option<&'static Rule> {
    if !matches!(name, "layer" | "route_layer") {
        return None;
    }
    let mut pending = vec![args.clone()];
    while let Some(tokens) = pending.pop() {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Group(group) => pending.push(group.stream()),
                proc_macro2::TokenTree::Ident(ident)
                    if ident == "csrf_middleware" || ident == "headers_middleware" =>
                {
                    return Some(&ROUTER_BASELINE_LAYERS);
                }
                _ => {}
            }
        }
    }
    None
}

/// The rule for a method call with `args`: `.strip_prefix("rullst_csrf=")`,
/// which reads the raw CSRF cookie value (the `get_csrf_token` helper
/// generated before 13.0.0-alpha.6), in a file that does not call
/// `mask_csrf_token`. It is a rule about production behaviour, so test code
/// is skipped ([`super::test_code`]).
pub(super) fn cookie_rule(name: &str, args: &TokenStream, text: &str) -> Option<&'static Rule> {
    if name != "strip_prefix" || text.contains("mask_csrf_token") {
        return None;
    }
    let prefix = super::tokens::string_value(args.clone().into_iter().next().as_ref())?;
    (prefix == "rullst_csrf=").then_some(&CSRF_COOKIE_IN_PAGE)
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
