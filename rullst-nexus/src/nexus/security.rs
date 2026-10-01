use axum::{extract::State, response::Html};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::nexus::ai_chat::detect_ai_provider;
use crate::nexus::types::NexusState;
use crate::nexus::ui::{render_shell, render_sidebar, wants_fragment};

const AUDIT_CHAIN_UNAVAILABLE: &str = "Unavailable";

fn event_integrity_badge(verified_hmac: bool) -> (&'static str, &'static str) {
    if verified_hmac {
        ("HMAC VERIFIED", "nexus-badge-verified")
    } else {
        ("UNSIGNED LOCAL EVENT", "nexus-badge-unsigned")
    }
}

/// Maps a metric or event to a CSS tone class; inline colours would need a
/// relaxed `style-src`.
fn event_tone(event_type: &str) -> &'static str {
    match event_type {
        "HONEYPOT_TRAP_TRIGGERED" => "nexus-tone-rose",
        "XSS_PAYLOAD_NEUTRALIZED" => "nexus-tone-cyan",
        "AI_PROMPT_INJECTION_SHIELDED" => "nexus-tone-violet",
        _ => "nexus-tone-amber",
    }
}

/// GET /nexus/security — Visual Threat Radar (SOC) with bounded in-process telemetry.
#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_security_page(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let (ai_active, provider_name) = detect_ai_provider();
    let ai_status_badge = if ai_active {
        format!(
            "<span class=\"nexus-badge nexus-badge-ai-active\">Active: {}</span>",
            provider_name
        )
    } else {
        "<span class=\"nexus-badge nexus-badge-ai-offline\">Offline / Embedded Intelligence</span>"
            .to_string()
    };

    let store = rullst_security::SecurityStore::global();
    let honeypots_count = store.honeypot_traps_count.load(Ordering::Relaxed);
    let active_bans_count = store.active_banned_count();
    let prompt_injections_count = store
        .prompt_injections_blocked_count
        .load(Ordering::Relaxed);
    let sanitizations_count = store.sanitizations_count.load(Ordering::Relaxed);
    let prompts_inspected = store.prompts_inspected_count.load(Ordering::Relaxed);
    let pii_masked = store.pii_masked_count.load(Ordering::Relaxed);
    let log_redactions = store.log_redactions_count.load(Ordering::Relaxed);
    let zero_trust_mismatches = store.zero_trust_mismatches_count.load(Ordering::Relaxed);
    let schema_violations = store.schema_violations_count.load(Ordering::Relaxed);
    let sri_signed_assets = store.sri_signed_assets_count.load(Ordering::Relaxed);
    let mfa_verifications = store.mfa_verifications_count.load(Ordering::Relaxed);
    let deception_hits = store.deception_hits_count.load(Ordering::Relaxed);
    let cswsh_blocks = store.cswsh_blocks_count.load(Ordering::Relaxed);
    let rate_limit_blocks = store.rate_limit_blocks_count.load(Ordering::Relaxed);
    let siem_dispatches = store.siem_dispatches_count.load(Ordering::Relaxed);
    let login_jail_bans = store.login_jail_bans_count.load(Ordering::Relaxed);
    let dlp_secrets_masked = store.dlp_secrets_masked_count.load(Ordering::Relaxed);
    let secure_headers_applied = store.secure_headers_applied_count.load(Ordering::Relaxed);
    let idor_warnings = store.idor_warnings_count.load(Ordering::Relaxed);
    let timing_guard_protected = store.timing_guard_protected_count.load(Ordering::Relaxed);

    // Build Banned IPs List
    let mut banned_ips_html = String::new();
    if store.banned_ips.is_empty() {
        banned_ips_html.push_str(
            "<div class=\"nexus-feed-empty\">No IP addresses currently banned by WAF.</div>",
        );
    } else {
        for ref_multi in store.banned_ips.iter() {
            let rec = ref_multi.value();
            banned_ips_html.push_str(&format!(
                "<div class=\"nexus-feed-row\">\
                 <span class=\"nexus-tone-rose nexus-event-type\">{}</span>\
                 <span class=\"nexus-muted\">{} ({})</span>\
                 </div>",
                rullst_core::html::escape_str(&rec.ip),
                rullst_core::html::escape_str(&rec.reason),
                rullst_core::html::escape_str(&rec.timestamp_str)
            ));
        }
    }

    // Build Honeypot Routes List
    let mut honeypot_routes_html = String::new();
    if store.honeypot_route_hits.is_empty() {
        let default_traps = vec![
            "/.env",
            "/.env.local",
            "/.env.production",
            "/.git/config",
            "/.aws/credentials",
            "/.vscode/sftp.json",
            "/.ds_store",
            "/admin.php",
            "/wp-login.php",
            "/wp-admin/",
            "/phpmyadmin/",
            "/config.json",
            "/setup.php",
            "/xmlrpc.php",
            "/actuator/health",
            "/console",
            "/api/v1/debug",
            "/swagger-ui.html",
            "/database.sqlite",
            "/backup.sql",
            "/server-status",
            "/docker-compose.yml",
        ];
        for trap in default_traps {
            honeypot_routes_html.push_str(&format!(
                "<div class=\"nexus-feed-row\">\
                 <span class=\"nexus-tone-amber\">{}</span>\
                 <span class=\"nexus-muted\">Available default; mount middleware to arm</span>\
                 </div>",
                trap
            ));
        }
    } else {
        for ref_multi in store.honeypot_route_hits.iter() {
            let path = ref_multi.key();
            let hits = ref_multi.value().load(Ordering::Relaxed);
            honeypot_routes_html.push_str(&format!(
                "<div class=\"nexus-feed-row\">\
                 <span class=\"nexus-tone-amber\">{}</span>\
                 <span class=\"nexus-muted\">{} hits</span>\
                 </div>",
                rullst_core::html::escape_str(path),
                hits
            ));
        }
    }

    // Build Live Events Feed
    let mut events_feed_html = String::new();
    if let Ok(events) = store.live_events.lock() {
        if events.is_empty() {
            events_feed_html.push_str(
                "<div class=\"nexus-feed-empty\">No in-process security events recorded. This does not prove that every security middleware is mounted.</div>",
            );
        } else {
            for ev in events.iter().take(15) {
                let tone = event_tone(ev.event_type.as_str());
                let (integrity_badge, integrity_class) = event_integrity_badge(ev.verified_hmac);
                events_feed_html.push_str(&format!(
                    "<div class=\"nexus-event {}\">\
                     <div>\
                         <span class=\"nexus-event-type\">{}</span>\
                         <span class=\"nexus-event-detail\">{}</span>\
                     </div>\
                     <div class=\"nexus-event-meta\">\
                         <span class=\"nexus-badge {}\">{}</span>\
                         <span class=\"nexus-event-time\">{}</span>\
                     </div>\
                     </div>",
                    tone,
                    rullst_core::html::escape_str(&ev.event_type),
                    rullst_core::html::escape_str(&ev.details),
                    integrity_class,
                    integrity_badge,
                    rullst_core::html::escape_str(&ev.timestamp_str)
                ));
            }
        }
    }

    let content = format!(
        r#"
<div class="nexus-card nexus-stack">
    <div class="nexus-panel-header">
        <div>
            <h2 class="nexus-panel-title nexus-tone-emerald">
                <span>🛡️ Threat Radar & RASP Security SOC</span>
                <span class="nexus-badge nexus-badge-live">LIVE IN-PROCESS TELEMETRY</span>
            </h2>
            <p class="nexus-panel-lead">In-process RASP, WAF, AI prompt-filter and sanitization counters. Audit integrity is reported only when a verifier source is connected.</p>
        </div>
        <div>{ai_status_badge}</div>
    </div>

    <!-- 4 Primary Metric Cards -->
    <div class="nexus-metric-grid">
        <div class="nexus-metric">
            <div class="nexus-metric-label">Honeypot Traps Triggered</div>
            <div class="nexus-metric-value nexus-tone-amber">{honeypots_count}</div>
            <div class="nexus-metric-hint">Synthetics (/.env, /admin.php, /wp-login)</div>
        </div>
        <div class="nexus-metric">
            <div class="nexus-metric-label">Active Banned IPs</div>
            <div class="nexus-metric-value nexus-tone-rose">{active_bans_count}</div>
            <div class="nexus-metric-hint">DashMap WAF Thread-Safe Active Bans</div>
        </div>
        <div class="nexus-metric">
            <div class="nexus-metric-label">Prompt Injections Blocked</div>
            <div class="nexus-metric-value nexus-tone-violet">{prompt_injections_count}</div>
            <div class="nexus-metric-hint">AI Prompt Injection Shield Active</div>
        </div>
        <div class="nexus-metric">
            <div class="nexus-metric-label">XSS / Sanitizations</div>
            <div class="nexus-metric-value nexus-tone-emerald">{sanitizations_count}</div>
            <div class="nexus-metric-hint">In-process sanitization counter</div>
        </div>
    </div>

    <!-- AI Security Sentinel & Prompt Injection Shield -->
    <div class="nexus-section nexus-section-ai">
        <h3 class="nexus-section-title nexus-tone-violet">
            <span>🤖 AI Security Sentinel & Prompt Injection Shield</span>
        </h3>
        <div class="nexus-mini-grid">
            <div class="nexus-mini">
                <span class="nexus-mini-label">Prompts Inspected</span>
                <span class="nexus-mini-value nexus-tone-violet">{prompts_inspected}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Injections Blocked</span>
                <span class="nexus-mini-value nexus-tone-rose">{prompt_injections_count}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">PII Data Masked</span>
                <span class="nexus-mini-value nexus-tone-cyan">{pii_masked}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Audit Chain Source</span>
                <span class="nexus-mini-value nexus-tone-slate">{AUDIT_CHAIN_UNAVAILABLE}</span>
            </div>
        </div>
    </div>

    <!-- Deep Security & Zero-Trust Defense Primitives -->
    <div class="nexus-section nexus-section-defense">
        <h3 class="nexus-section-title nexus-tone-emerald">
            <span>🛡️ Deep Security & Zero-Trust Defenses</span>
        </h3>
        <div class="nexus-mini-grid">
            <div class="nexus-mini">
                <span class="nexus-mini-label">Log Secrets Redacted</span>
                <span class="nexus-mini-value nexus-tone-amber">{log_redactions}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Login Jail Bans</span>
                <span class="nexus-mini-value nexus-tone-rose">{login_jail_bans}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">DLP Leaks Blocked</span>
                <span class="nexus-mini-value nexus-tone-emerald">{dlp_secrets_masked}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">OWASP Headers Applied</span>
                <span class="nexus-mini-value nexus-tone-sky">{secure_headers_applied}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Zero-Trust Mismatches</span>
                <span class="nexus-mini-value nexus-tone-rose">{zero_trust_mismatches}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Schema / Bomb Blocked</span>
                <span class="nexus-mini-value nexus-tone-indigo">{schema_violations}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">SRI Signed Assets</span>
                <span class="nexus-mini-value nexus-tone-emerald">{sri_signed_assets}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">MFA TOTP Verified</span>
                <span class="nexus-mini-value nexus-tone-sky">{mfa_verifications}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Deception Traps Hit</span>
                <span class="nexus-mini-value nexus-tone-rose">{deception_hits}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">CSWSH Hijacks Blocked</span>
                <span class="nexus-mini-value nexus-tone-violet">{cswsh_blocks}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Rate Limit Drops</span>
                <span class="nexus-mini-value nexus-tone-amber">{rate_limit_blocks}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Local SIEM-Candidate Alerts</span>
                <span class="nexus-mini-value nexus-tone-emerald">{siem_dispatches}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">IDOR Route Warnings</span>
                <span class="nexus-mini-value nexus-tone-amber">{idor_warnings}</span>
            </div>
            <div class="nexus-mini">
                <span class="nexus-mini-label">Anti-Timing Protected</span>
                <span class="nexus-mini-value nexus-tone-cyan">{timing_guard_protected}</span>
            </div>
        </div>
    </div>

    <!-- Active Banned IPs List & Honeypot Routes -->
    <div class="nexus-split">
        <div class="nexus-box">
            <h3 class="nexus-box-title">🚫 Active WAF Banned IP Addresses ({active_bans_count})</h3>
            <div class="nexus-feed">
                {banned_ips_html}
            </div>
        </div>
        <div class="nexus-box">
            <h3 class="nexus-box-title">🍯 Honeypot Routes & Observed Hits</h3>
            <div class="nexus-feed">
                {honeypot_routes_html}
            </div>
        </div>
    </div>

    <!-- Local security event feed; integrity badges reflect each event's actual source. -->
    <div class="nexus-box">
        <h3 class="nexus-box-title">📜 Local Security Event Stream</h3>
        <div class="nexus-feed">
            {events_feed_html}
        </div>
    </div>
</div>
"#
    );

    if wants_fragment(&headers) {
        Html(content)
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, Some("security")),
            &content,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_integrity_badge_never_promotes_unsigned_events() {
        assert_eq!(event_integrity_badge(false).0, "UNSIGNED LOCAL EVENT");
        assert_eq!(event_integrity_badge(true).0, "HMAC VERIFIED");
        assert_eq!(AUDIT_CHAIN_UNAVAILABLE, "Unavailable");
    }
}

#[cfg(test)]
#[path = "security_page_tests.rs"]
mod page_tests;
