//! The 13.0.0-alpha.5 public API clean-ups: removed alias paths, internals
//! made crate-private, private Security collections, Connect's own JWKS key
//! set and error variants whose payload changed. [`super::tokens`] and [`super::rust`]
//! call these with the paths, identifiers and fields they read.

use super::Rule;
use super::catalog::{
    AI_HTTP_STATUS, ALIAS_PATHS, CONNECT_JWKS, ERROR_PAYLOADS, HIDDEN_INTERNALS, SECURITY_GLOBALS,
    STORE_COLLECTIONS,
};

/// Removed second paths of an item, as consecutive path segments.
const ALIASES: &[&[&str]] = &[
    &["rullst", "rullst_orm"],
    &["rullst", "security_runtime"],
    &["rullst_mail", "mail"],
    &["rullst", "mail", "mail"],
    &["rullst_mail", "drivers", "mock"],
    &["rullst", "mail", "drivers", "mock"],
];

/// Internals named through their module, whose own names are generic.
const HIDDEN_PATHS: &[&[&str]] = &[
    &["artisan", "runner"],
    &["server", "dylib_loader"],
    &["tenant", "CURRENT_TENANT"],
    &["schema", "QUERY_LOGGING"],
    &["schema", "QUERY_TIMEOUT_SECS"],
    &["schema", "MAX_QUERY_LIMIT"],
    &["passkey", "cbor"],
    &["client", "DEFAULT_HTTP_CLIENT"],
    &["rullst_connect", "macros"],
];

/// Distinctive names of the internals made crate-private or removed.
const HIDDEN_IDENTS: &[&str] = &[
    "dylib_loader",
    "encrypt_aes_gcm",
    "decrypt_aes_gcm",
    "redis_cache_namespace",
    "impl_standard_redirect_url",
    "impl_standard_refresh_token",
    "current_timestamp_str",
    "get_real_rss_memory_mb",
    "generate_smart_nexus_ai_response",
    "field_kind_sql",
    "claim_next_at",
    "model_hash_key",
];

/// Security functions that returned `dashmap` collections.
const SECURITY_GLOBAL_IDENTS: &[&str] = &[
    "default_deception_traps",
    "global_deception_routes",
    "global_rate_limit_store",
];

/// Connect's JWKS items whose key-set type changed, and the removed raw cache.
const JWKS_IDENTS: &[&str] = &[
    "JWKS_CACHE",
    "JwksCache",
    "fetch_and_cache_jwks",
    "fetch_and_cache_jwks_for_kid",
];

/// Error variants whose payload is now a Rullst type.
const PAYLOAD_VARIANTS: &[[&str; 2]] = &[
    ["AiError", "RequestError"],
    ["AiError", "SerializationError"],
    ["ClientContractError", "InvalidJson"],
    ["ClientContractError", "EncodeJson"],
    ["TwinSyncError", "Serialization"],
];

/// The private `SecurityStore` collection fields.
const STORE_FIELDS: &[&str] = &["banned_ips", "honeypot_route_hits", "live_events"];

fn contains(segments: &[String], needle: &[&str]) -> bool {
    segments
        .windows(needle.len())
        .any(|window| window.iter().zip(needle).all(|(left, right)| left == right))
}

/// The rule for a path such as `rullst::rullst_orm::Orm`, if any.
pub(super) fn path_rule(segments: &[String]) -> Option<&'static Rule> {
    if ALIASES.iter().any(|alias| contains(segments, alias)) {
        return Some(&ALIAS_PATHS);
    }
    if HIDDEN_PATHS.iter().any(|path| contains(segments, path)) {
        return Some(&HIDDEN_INTERNALS);
    }
    if contains(segments, &["AiError", "ApiError"]) {
        return Some(&AI_HTTP_STATUS);
    }
    PAYLOAD_VARIANTS
        .iter()
        .any(|variant| contains(segments, variant))
        .then_some(&ERROR_PAYLOADS)
}

/// The rule for a distinctive identifier anywhere in the code, if any.
pub(super) fn ident_rule(name: &str) -> Option<&'static Rule> {
    if HIDDEN_IDENTS.contains(&name) {
        Some(&HIDDEN_INTERNALS)
    } else if SECURITY_GLOBAL_IDENTS.contains(&name) {
        Some(&SECURITY_GLOBALS)
    } else {
        JWKS_IDENTS.contains(&name).then_some(&CONNECT_JWKS)
    }
}

/// The rule for a field access `.name` (not a method call) in a file whose
/// text names `SecurityStore` or its `SecurityTelemetry` alias.
pub(super) fn field_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (STORE_FIELDS.contains(&name)
        && (text.contains("SecurityStore") || text.contains("SecurityTelemetry")))
    .then_some(&STORE_COLLECTIONS)
}

/// Every full path of a `use` tree with the line of its last segment, so
/// `use rullst::{rullst_orm, web};` yields `rullst::rullst_orm`.
pub(super) fn use_paths(tree: &syn::UseTree) -> Vec<(Vec<String>, usize)> {
    let mut found = Vec::new();
    collect(tree, &mut Vec::new(), &mut found);
    found
}

fn collect(tree: &syn::UseTree, prefix: &mut Vec<String>, found: &mut Vec<(Vec<String>, usize)>) {
    let mut leaf = |ident: &syn::Ident, prefix: &[String]| {
        let mut path = prefix.to_vec();
        path.push(ident.to_string());
        found.push((path, ident.span().start().line));
    };
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect(&path.tree, prefix, found);
            prefix.pop();
        }
        syn::UseTree::Name(name) => leaf(&name.ident, prefix),
        syn::UseTree::Rename(rename) => leaf(&rename.ident, prefix),
        syn::UseTree::Glob(glob) if !prefix.is_empty() => {
            found.push((prefix.clone(), glob.star_token.spans[0].start().line));
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect(item, prefix, found);
            }
        }
    }
}
