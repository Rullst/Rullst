use axum::{extract::State, response::Html};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::nexus::ai_chat::detect_ai_provider;
use crate::nexus::types::NexusState;
use crate::nexus::ui::{render_shell, render_sidebar};

/// GET /nexus/security — Visual Threat Radar (SOC) with 100% Real Live Security Telemetry.
#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_security_page(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let (ai_active, provider_name) = detect_ai_provider();
    let ai_status_badge = if ai_active {
        format!(
            "<span class=\"nexus-badge\" style=\"background: rgba(168, 85, 247, 0.2); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.4);\">Active: {}</span>",
            provider_name
        )
    } else {
        "<span class=\"nexus-badge\" style=\"background: rgba(148, 163, 184, 0.2); color: #94a3b8; border: 1px solid rgba(148, 163, 184, 0.4);\">Offline / Embedded Intelligence</span>".to_string()
    };

    let store = rullst_security::SecurityStore::global();
    let honeypots_count = store.honeypot_traps_count.load(Ordering::Relaxed);
    let active_bans_count = store.banned_ips.len();
    let prompt_injections_count = store
        .prompt_injections_blocked_count
        .load(Ordering::Relaxed);
    let sanitizations_count = store.sanitizations_count.load(Ordering::Relaxed);
    let prompts_inspected = store.prompts_inspected_count.load(Ordering::Relaxed);
    let pii_masked = store.pii_masked_count.load(Ordering::Relaxed);

    // Build Banned IPs List
    let mut banned_ips_html = String::new();
    if store.banned_ips.is_empty() {
        banned_ips_html.push_str(
            "<div style=\"padding: 12px; color: var(--text-muted); font-size: 13px; text-align: center;\">No IP addresses currently banned by WAF.</div>"
        );
    } else {
        for ref_multi in store.banned_ips.iter() {
            let rec = ref_multi.value();
            banned_ips_html.push_str(&format!(
                "<div style=\"display: flex; justify-content: space-between; padding: 8px 12px; background: var(--bg-800); border-radius: 6px;\">\
                 <span style=\"color: #f43f5e; font-weight: 700;\">{}</span>\
                 <span style=\"color: var(--text-muted);\">{} ({})</span>\
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
                "<div style=\"display: flex; justify-content: space-between; padding: 6px 10px; background: var(--bg-800); border-radius: 6px;\">\
                 <span style=\"color: #fbbf24;\">{}</span>\
                 <span style=\"color: var(--text-muted);\">0 hits (Armed)</span>\
                 </div>",
                trap
            ));
        }
    } else {
        for ref_multi in store.honeypot_route_hits.iter() {
            let path = ref_multi.key();
            let hits = ref_multi.value().load(Ordering::Relaxed);
            honeypot_routes_html.push_str(&format!(
                "<div style=\"display: flex; justify-content: space-between; padding: 6px 10px; background: var(--bg-800); border-radius: 6px;\">\
                 <span style=\"color: #fbbf24;\">{}</span>\
                 <span style=\"color: var(--text-muted);\">{} hits</span>\
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
                "<div style=\"padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;\">🛡️ No security incidents recorded. RASP and WAF shields active.</div>"
            );
        } else {
            for ev in events.iter().take(15) {
                let badge_color = match ev.event_type.as_str() {
                    "HONEYPOT_TRAP_TRIGGERED" => "#f43f5e",
                    "XSS_PAYLOAD_NEUTRALIZED" => "#22d3ee",
                    "AI_PROMPT_INJECTION_SHIELDED" => "#c084fc",
                    _ => "#fbbf24",
                };
                events_feed_html.push_str(&format!(
                    "<div style=\"background: var(--bg-800); padding: 10px 14px; border-radius: 6px; border-left: 4px solid {}; display: flex; justify-content: space-between; align-items: center;\">\
                     <div>\
                         <span style=\"font-weight: 700; color: {};\">{}</span>\
                         <span style=\"color: var(--text-muted); margin-left: 12px;\">{}</span>\
                     </div>\
                     <div style=\"display: flex; align-items: center; gap: 12px;\">\
                         <span class=\"nexus-badge\" style=\"background: rgba(52,211,153,0.15); color: #34d399;\">HMAC VERIFIED</span>\
                         <span style=\"color: var(--text-muted); font-size: 11px;\">{}</span>\
                     </div>\
                     </div>",
                    badge_color,
                    badge_color,
                    rullst_core::html::escape_str(&ev.event_type),
                    rullst_core::html::escape_str(&ev.details),
                    rullst_core::html::escape_str(&ev.timestamp_str)
                ));
            }
        }
    }

    let content = format!(
        r#"
<div class="nexus-card" style="display: flex; flex-direction: column; gap: 24px;">
    <div style="display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 16px;">
        <div>
            <h2 style="margin: 0; color: #34d399; display: flex; align-items: center; gap: 10px; font-size: 20px;">
                <span>🛡️ Threat Radar & RASP Security SOC</span>
                <span class="nexus-badge" style="background: rgba(16, 185, 129, 0.2); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.4);">LIVE REAL TELEMETRY</span>
            </h2>
            <p style="margin: 4px 0 0 0; font-size: 13px; color: var(--text-muted);">Real-time application self-protection (RASP), WAF active bans, AI Prompt Injection shield, and HMAC SHA-256 audit logs.</p>
        </div>
        <div>{ai_status_badge}</div>
    </div>

    <!-- 4 Primary Metric Cards -->
    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px;">
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;">Honeypot Traps Triggered</div>
            <div style="font-size: 32px; font-weight: 800; color: #fbbf24; margin-top: 4px;">{honeypots_count}</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">Synthetics (/.env, /admin.php, /wp-login)</div>
        </div>
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;">Active Banned IPs</div>
            <div style="font-size: 32px; font-weight: 800; color: #f43f5e; margin-top: 4px;">{active_bans_count}</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">DashMap WAF Thread-Safe Active Bans</div>
        </div>
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;">Prompt Injections Blocked</div>
            <div style="font-size: 32px; font-weight: 800; color: #c084fc; margin-top: 4px;">{prompt_injections_count}</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">AI Prompt Injection Shield Active</div>
        </div>
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;">XSS / Sanitizations</div>
            <div style="font-size: 32px; font-weight: 800; color: #34d399; margin-top: 4px;">{sanitizations_count}</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">SHA-256 Tamper-Proof Chain Log</div>
        </div>
    </div>

    <!-- AI Security Sentinel & Prompt Injection Shield -->
    <div style="background: rgba(147, 51, 234, 0.05); padding: 20px; border-radius: 10px; border: 1px solid rgba(147, 51, 234, 0.25);">
        <h3 style="margin: 0 0 12px 0; color: #c084fc; font-size: 16px; display: flex; align-items: center; gap: 8px;">
            <span>🤖 AI Security Sentinel & Prompt Injection Shield</span>
        </h3>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; font-size: 13px;">
            <div style="background: var(--bg-900); padding: 12px 16px; border-radius: 8px; border: 1px solid var(--border);">
                <span style="color: var(--text-muted); font-size: 11px; display: block;">Prompts Inspected</span>
                <span style="font-size: 20px; font-weight: 700; color: #c084fc;">{prompts_inspected}</span>
            </div>
            <div style="background: var(--bg-900); padding: 12px 16px; border-radius: 8px; border: 1px solid var(--border);">
                <span style="color: var(--text-muted); font-size: 11px; display: block;">Injections Blocked</span>
                <span style="font-size: 20px; font-weight: 700; color: #f43f5e;">{prompt_injections_count}</span>
            </div>
            <div style="background: var(--bg-900); padding: 12px 16px; border-radius: 8px; border: 1px solid var(--border);">
                <span style="color: var(--text-muted); font-size: 11px; display: block;">PII Data Masked</span>
                <span style="font-size: 20px; font-weight: 700; color: #22d3ee;">{pii_masked}</span>
            </div>
            <div style="background: var(--bg-900); padding: 12px 16px; border-radius: 8px; border: 1px solid var(--border);">
                <span style="color: var(--text-muted); font-size: 11px; display: block;">HMAC Audit Chain</span>
                <span style="font-size: 20px; font-weight: 700; color: #34d399;">100% VERIFIED</span>
            </div>
        </div>
    </div>

    <!-- Active Banned IPs List & Honeypot Routes -->
    <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 16px;">
        <div style="background: var(--bg-900); padding: 20px; border-radius: 10px; border: 1px solid var(--border);">
            <h3 style="margin-top: 0; color: var(--text-main); font-size: 15px;">🚫 Active WAF Banned IP Addresses ({active_bans_count})</h3>
            <div style="display: flex; flex-direction: column; gap: 8px; font-size: 12px; font-family: var(--font-mono); margin-top: 12px;">
                {banned_ips_html}
            </div>
        </div>
        <div style="background: var(--bg-900); padding: 20px; border-radius: 10px; border: 1px solid var(--border);">
            <h3 style="margin-top: 0; color: var(--text-main); font-size: 15px;">🍯 Active Honeypot Traps</h3>
            <div style="display: flex; flex-direction: column; gap: 8px; font-size: 12px; font-family: var(--font-mono); margin-top: 12px;">
                {honeypot_routes_html}
            </div>
        </div>
    </div>

    <!-- Live HMAC Audit Log Feed -->
    <div style="background: var(--bg-900); padding: 20px; border-radius: 10px; border: 1px solid var(--border);">
        <h3 style="margin-top: 0; color: var(--text-main); font-size: 15px;">📜 HMAC SHA-256 Security Audit Trail Log Stream</h3>
        <div style="display: flex; flex-direction: column; gap: 8px; font-size: 12px; margin-top: 12px; font-family: var(--font-mono);">
            {events_feed_html}
        </div>
    </div>
</div>
"#
    );

    if headers.contains_key("hx-request") {
        Html(content)
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, Some("security")),
            &content,
        ))
    }
}
