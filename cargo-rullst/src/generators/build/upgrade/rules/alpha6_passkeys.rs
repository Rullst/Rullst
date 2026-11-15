//! 13.0.0-alpha.6 moved Rullst's passkeys (`rullst_auth::auth::passkey`,
//! `rullst::auth::passkey`) and the aws-lc-rs dependency that checks their
//! keys and signatures behind `rullst-auth`'s `passkeys` feature
//! (`V13-AUTH-PASSKEYS-FEATURE`). [`super::files`] records whether a manifest
//! enables it; [`super::rust`] and [`super::tokens`] call [`path_rule`] with
//! each path they read.

use super::catalog::PASSKEYS_FEATURE;
use super::{Rule, WorkspaceFacts};

/// The features of `package` that build the passkeys since 13.0.0-alpha.6.
pub(super) fn passkey_features(package: &str) -> &'static [&'static str] {
    match package {
        "rullst" => &["auth-passkeys", "auth-sqlite", "auth-passkey-postgres"],
        "rullst-auth" => &["passkeys", "sqlite", "passkey-postgres"],
        _ => &[],
    }
}

/// A path into the passkey module: `rullst::auth::passkey`,
/// `rullst_auth::passkey` or either through `auth::auth`, or `auth::passkey`
/// in a file that names `rullst::auth` or `rullst_auth` (after
/// `use rullst::auth;`). An application's own `crate::auth::passkey` does not
/// count.
fn names_passkeys(segments: &[String], text: &str) -> bool {
    let names: Vec<&str> = segments.iter().map(String::as_str).collect();
    match names.as_slice() {
        ["rullst" | "rullst_auth", "auth", "passkey", ..]
        | ["rullst", "auth", "auth", "passkey", ..]
        | ["rullst_auth", "passkey", ..] => true,
        ["auth", "passkey", ..] => text.contains("rullst::auth") || text.contains("rullst_auth"),
        _ => false,
    }
}

/// The rule for a path that names the passkeys when no manifest enables them.
pub(super) fn path_rule(
    segments: &[String],
    text: &str,
    facts: &WorkspaceFacts,
) -> Option<&'static Rule> {
    (!facts.passkeys && names_passkeys(segments, text)).then_some(&PASSKEYS_FEATURE)
}
