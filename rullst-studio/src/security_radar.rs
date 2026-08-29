//! Bounded, process-local security telemetry for Studio.

use axum::{
    Router,
    response::{Html, Json},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Write, sync::atomic::Ordering};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreatRadarStats {
    pub honeypot_traps_blocked: u64,
    pub active_ip_bans: usize,
    pub xss_sanitizations: u64,
    pub rbac_violations_prevented: u64,
    pub log_redactions: u64,
    pub zero_trust_mismatches: u64,
    pub schema_violations: u64,
    pub sri_signed_assets: u64,
    pub mfa_verifications: u64,
    pub deception_hits: u64,
    pub cswsh_blocks: u64,
    pub rate_limit_blocks: u64,
    pub siem_dispatches: u64,
    pub login_jail_bans: u64,
    pub dlp_secrets_masked: u64,
    pub secure_headers_applied: u64,
    pub idor_warnings: u64,
    pub timing_guard_protected: u64,
    pub prompt_injections_blocked: u64,
    pub audit_chain_integrity: Option<String>,
    pub threat_level: String,
    pub live_events: Vec<rullst_security::LiveSecurityEvent>,
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(render_radar_dashboard))
        .route("/stats", get(get_radar_stats))
}

/// JSON telemetry endpoints merged into the main Studio router without
/// replacing the unified `/studio/security` page.
pub fn stats_router() -> Router {
    Router::new()
        .route("/security/stats", get(get_radar_stats))
        .route("/studio/security/stats", get(get_radar_stats))
}

fn collect_stats() -> ThreatRadarStats {
    let store = rullst_security::SecurityStore::global();
    let active_ip_bans = store.active_banned_count();
    let live_events = match store.live_events.lock() {
        Ok(events) => events,
        Err(poisoned) => poisoned.into_inner(),
    };
    let events: Vec<rullst_security::LiveSecurityEvent> =
        live_events.iter().take(20).cloned().collect();
    drop(live_events);

    let honeypot_traps_blocked = store.honeypot_traps_count.load(Ordering::Relaxed);
    let xss_sanitizations = store.sanitizations_count.load(Ordering::Relaxed);
    let rbac_violations_prevented = store.rbac_denials_count.load(Ordering::Relaxed);
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
    let prompt_injections_blocked = store
        .prompt_injections_blocked_count
        .load(Ordering::Relaxed);
    let observed_counter_total = honeypot_traps_blocked
        .saturating_add(xss_sanitizations)
        .saturating_add(rbac_violations_prevented)
        .saturating_add(log_redactions)
        .saturating_add(zero_trust_mismatches)
        .saturating_add(schema_violations)
        .saturating_add(sri_signed_assets)
        .saturating_add(mfa_verifications)
        .saturating_add(deception_hits)
        .saturating_add(cswsh_blocks)
        .saturating_add(rate_limit_blocks)
        .saturating_add(siem_dispatches)
        .saturating_add(login_jail_bans)
        .saturating_add(dlp_secrets_masked)
        .saturating_add(secure_headers_applied)
        .saturating_add(idor_warnings)
        .saturating_add(timing_guard_protected)
        .saturating_add(prompt_injections_blocked);
    let local_activity_recorded =
        active_ip_bans > 0 || !events.is_empty() || observed_counter_total > 0;

    ThreatRadarStats {
        honeypot_traps_blocked,
        active_ip_bans,
        xss_sanitizations,
        rbac_violations_prevented,
        log_redactions,
        zero_trust_mismatches,
        schema_violations,
        sri_signed_assets,
        mfa_verifications,
        deception_hits,
        cswsh_blocks,
        rate_limit_blocks,
        siem_dispatches,
        login_jail_bans,
        dlp_secrets_masked,
        secure_headers_applied,
        idor_warnings,
        timing_guard_protected,
        prompt_injections_blocked,
        // No audit-chain verifier is supplied to Studio today.
        audit_chain_integrity: None,
        threat_level: if local_activity_recorded {
            "LOCAL_ACTIVITY_RECORDED".to_string()
        } else {
            "NO_LOCAL_ACTIVITY_RECORDED".to_string()
        },
        live_events: events,
    }
}

async fn get_radar_stats() -> Json<ThreatRadarStats> {
    Json(collect_stats())
}

async fn render_radar_dashboard() -> Html<String> {
    let stats = collect_stats();
    let mut rows = String::new();
    for (label, value) in [
        ("Honeypot trap calls", stats.honeypot_traps_blocked),
        ("HTML sanitizations", stats.xss_sanitizations),
        ("RBAC denials", stats.rbac_violations_prevented),
        ("Log redactions", stats.log_redactions),
        ("Schema violations", stats.schema_violations),
        ("Rate-limit blocks", stats.rate_limit_blocks),
        ("Prompt heuristic blocks", stats.prompt_injections_blocked),
    ] {
        let _ = write!(
            rows,
            "<tr class=\"border-b border-slate-800/70\"><th scope=\"row\" class=\"px-4 py-3 text-left font-medium text-slate-300\">{}</th><td class=\"px-4 py-3 text-right font-mono text-sky-300\">{value}</td></tr>",
            rullst_core::html::escape_str(label)
        );
    }

    let mut events = String::new();
    if stats.live_events.is_empty() {
        events.push_str("<p class=\"text-sm text-slate-400\">No local security events were recorded. This is not proof that every control is mounted or that no attack occurred.</p>");
    } else {
        events.push_str("<ul class=\"space-y-2\">");
        for event in &stats.live_events {
            let _ = write!(
                events,
                "<li class=\"rounded-lg border border-slate-800 bg-slate-950/70 p-3 text-sm text-slate-300\"><strong class=\"text-amber-300\">{}</strong> — {} — {}</li>",
                rullst_core::html::escape_str(&event.event_type),
                rullst_core::html::escape_str(&event.timestamp_str),
                rullst_core::html::escape_str(&event.details)
            );
        }
        events.push_str("</ul>");
    }

    let content = format!(
        r#"<section class="mx-auto w-full max-w-6xl p-6 lg:p-10">
  <div class="mb-8 flex flex-wrap items-start justify-between gap-4">
    <div>
      <p class="mb-2 text-xs font-bold uppercase tracking-[0.22em] text-emerald-400">Process-local evidence</p>
      <h1 class="text-3xl font-extrabold tracking-tight text-slate-100">Rullst local security telemetry</h1>
      <p class="mt-3 max-w-3xl text-sm leading-6 text-slate-400">Bounded counters from this process only. Unobserved middleware and external systems are not represented.</p>
    </div>
    <span class="inline-flex items-center gap-2 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-xs font-semibold text-emerald-300"><span class="h-2 w-2 rounded-full bg-emerald-400"></span>Local snapshot</span>
  </div>
  <div class="mb-6 rounded-2xl border border-slate-800 bg-slate-900/70 p-5 shadow-xl shadow-black/20 backdrop-blur">
    <p class="text-sm text-slate-300">Audit-chain integrity: <strong class="text-amber-300">unavailable</strong> (no verifier connected). Active in-memory IP bans: <strong class="text-sky-300">{active_bans}</strong>.</p>
  </div>
  <div class="grid gap-6 lg:grid-cols-2">
    <div class="overflow-hidden rounded-2xl border border-slate-800 bg-slate-900/70 shadow-xl shadow-black/20 backdrop-blur">
      <table class="w-full"><caption class="px-4 py-4 text-left text-sm font-bold uppercase tracking-wider text-slate-400">Locally recorded operations</caption><tbody>{rows}</tbody></table>
    </div>
    <div class="rounded-2xl border border-slate-800 bg-slate-900/70 p-5 shadow-xl shadow-black/20 backdrop-blur">
      <h2 class="mb-4 text-lg font-bold text-slate-100">Recent local events</h2>{events}
    </div>
  </div>
</section>"#,
        active_bans = stats.active_ip_bans,
    );
    Html(crate::data_browser::studio_layout(content, None, &[]))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn standalone_radar_reports_bounded_local_state_without_certification() {
        let response = router()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("radar body");
        let html = String::from_utf8(body.to_vec()).expect("UTF-8 radar page");

        assert!(html.contains("Bounded counters from this process only"));
        assert!(html.contains("Audit-chain integrity:"));
        assert!(html.contains(">unavailable</strong>"));
        assert!(html.contains("Rullst Studio Control Center"));
        assert!(html.contains("bg-slate-900/70"));
        assert!(!html.contains("A+ Rating"));
        assert!(!html.contains("System operating normally"));
        assert!(!html.contains("LIVE SOC MONITOR"));
    }
}
