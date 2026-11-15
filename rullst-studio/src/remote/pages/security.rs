//! The security page: the radar of a `rullst.monitor.v2` document.

use super::monitor::project_heading;
use super::traces::{ago, version_1_only};
use super::{card, notice, table};
use crate::remote::document::MonitorDocument;

/// Counters shown by name, in this order; others follow under their key.
const COUNTERS: [(&str, &str); 18] = [
    ("rate_limit_blocks", "Rate-limit and lockout refusals"),
    ("login_jail_bans", "Login Jail lockouts"),
    ("honeypot_traps", "Honeypot hits"),
    ("rbac_denials", "RBAC and ownership denials"),
    ("cswsh_blocks", "Cross-site WebSocket refusals"),
    ("schema_violations", "JSON schema rejections"),
    ("prompt_injections_blocked", "Prompt injections blocked"),
    ("deception_hits", "Deception trap hits"),
    ("zero_trust_mismatches", "Zero-trust mismatches"),
    ("idor_warnings", "IDOR warnings"),
    ("sanitizations", "HTML payloads sanitized"),
    ("dlp_secrets_masked", "Payloads with secrets masked"),
    ("log_redactions", "Log lines redacted"),
    ("siem_dispatches", "Local SIEM alerts"),
    ("timing_guard_protected", "Timing-guarded responses"),
    ("mfa_verifications", "MFA verifications"),
    ("secure_headers_applied", "Responses with secure headers"),
    ("sri_signed_assets", "SRI hashes computed"),
];

/// The security radar: counters, bans and the newest events.
pub(crate) fn security(project: &str, document: &MonitorDocument) -> String {
    let mut out = project_heading(project, document, "Security");
    let Some(security) = &document.security else {
        out.push_str(&version_1_only());
        return out;
    };
    let counter = |key: &str| security.counters.get(key).copied().unwrap_or(0);
    out.push_str(r#"<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">"#);
    for (label, value, hint) in [
        (
            "Security events",
            security.events_total,
            "recorded since the process started",
        ),
        (
            "Active bans",
            security.active_bans,
            "peers banned by a honeypot",
        ),
        (
            "Refusals",
            counter("rate_limit_blocks"),
            "rate limits and lockouts",
        ),
        (
            "Login Jail",
            counter("login_jail_bans"),
            "peers locked out after failed sign-ins",
        ),
    ] {
        out.push_str(&card(label, &value.to_string(), hint));
    }
    out.push_str("</div>");
    let mut rows: Vec<Vec<String>> = COUNTERS
        .iter()
        .filter_map(|(key, label)| {
            let value = security.counters.get(*key)?;
            Some(vec![label.to_string(), value.to_string()])
        })
        .collect();
    rows.extend(
        security
            .counters
            .iter()
            .filter(|(key, _)| !COUNTERS.iter().any(|(known, _)| known == key))
            .map(|(key, value)| vec![key.clone(), value.to_string()]),
    );
    out.push_str(&table(&["Counter", "Since start"], &rows));
    if security.events.is_empty() {
        out.push_str(&notice(
            "ok",
            "No security events kept",
            "The application's security layers recorded no event that is still in its buffer. This does not prove that every layer is mounted.",
        ));
    } else {
        out.push_str(r#"<p class="text-sm text-slate-400">The newest events the application keeps, by type and time. Their details and client addresses never leave the server.</p>"#);
        let rows: Vec<Vec<String>> = security
            .events
            .iter()
            .map(|event| vec![ago(document, event.at), event.event_type.clone()])
            .collect();
        out.push_str(&table(&["When", "Event"], &rows));
    }
    out
}
