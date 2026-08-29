//! Security / Visual Threat Radar handler.

use super::super::layout::*;
use axum::response::{Html, IntoResponse};

pub async fn handle_studio_tools_security(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let (ai_configured, provider_name) = detect_ai_provider();
    let provider_observation = if ai_configured {
        format!(
            "Configuration detected for {}; reachability and mounted guardrails are not observed by Studio.",
            rullst_core::html::escape_str(&provider_name)
        )
    } else {
        "No supported provider configuration was detected. Studio does not connect an AI client automatically."
            .to_string()
    };

    let ai_setup_box = if !ai_configured {
        r#"<div class="bg-slate-900 border border-amber-900/60 rounded-xl p-6 mb-8">
            <h2 class="text-lg font-bold text-amber-400 mb-2 flex items-center gap-2">
                <span>AI provider configuration</span>
            </h2>
            <p class="text-slate-300 text-sm mb-4">The application-facing client supports OpenAI, Anthropic, Gemini, DeepSeek and Ollama. Configure and construct the client in application code; Studio does not treat a credential as proof of a live or protected integration.</p>
            <div class="bg-slate-950 p-4 rounded-lg border border-slate-800 text-xs font-mono space-y-2">
                <p class="text-slate-400"># Google Gemini:</p>
                <p class="text-cyan-300">GEMINI_API_KEY="AIzaSyYourGeminiApiKeyHere"</p>
                <p class="text-slate-400 mt-2"># OpenAI (ChatGPT / GPT-4o):</p>
                <p class="text-emerald-300">OPENAI_API_KEY="sk-YourOpenAiKeyHere"</p>
                <p class="text-slate-400 mt-2"># Anthropic Claude:</p>
                <p class="text-purple-300">ANTHROPIC_API_KEY="sk-ant-YourClaudeKeyHere"</p>
                <p class="text-slate-400 mt-2"># DeepSeek:</p>
                <p class="text-yellow-300">DEEPSEEK_API_KEY="sk-YourDeepSeekKeyHere"</p>
                <p class="text-slate-400 mt-2"># Ollama endpoint operated by you:</p>
                <p class="text-sky-300">OLLAMA_HOST="http://127.0.0.1:11434"</p>
                <p class="text-slate-400 mt-3"># 2. Add rullst-ai to your dependencies or use CLI scaffold:</p>
                <p class="text-yellow-300">cargo rullst pkg add rullst-ai</p>
            </div>
        </div>"#
    } else {
        ""
    };

    let sec_store = rullst_security::SecurityStore::global();
    let log_redactions = sec_store
        .log_redactions_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let zero_trust_mismatches = sec_store
        .zero_trust_mismatches_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let schema_violations = sec_store
        .schema_violations_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let sri_signed = sec_store
        .sri_signed_assets_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let mfa_verifications = sec_store
        .mfa_verifications_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let deception_hits = sec_store
        .deception_hits_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let cswsh_blocks = sec_store
        .cswsh_blocks_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let rate_limit_blocks = sec_store
        .rate_limit_blocks_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let siem_dispatches = sec_store
        .siem_dispatches_count
        .load(std::sync::atomic::Ordering::Relaxed);

    let events = sec_store
        .live_events
        .lock()
        .map(|e| e.iter().take(6).cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let mut incidents_html = String::new();
    if events.is_empty() {
        incidents_html.push_str(
            r#"<div class="p-6 text-center text-xs text-slate-500 font-medium bg-slate-950/60 border border-slate-800/80 rounded-xl">
                No local security events have been recorded by this process. This does not prove that every security layer is mounted or that no attack occurred.
            </div>"#,
        );
    } else {
        incidents_html.push_str(r#"<div class="space-y-3 font-mono text-xs">"#);
        for evt in events {
            let (badge_color, border_color) = match evt.event_type.as_str() {
                "HONEYPOT_TRAP_TRIGGERED" | "LOGIN_JAIL_TRIGGERED" => (
                    "text-rose-400 border-rose-500/30 bg-rose-500/10",
                    "border-rose-900/40",
                ),
                "XSS_SANITIZED" | "DLP_SECRET_LEAK_PREVENTED" => (
                    "text-cyan-400 border-cyan-500/30 bg-cyan-500/10",
                    "border-cyan-900/40",
                ),
                _ => (
                    "text-amber-400 border-amber-500/30 bg-amber-500/10",
                    "border-amber-900/40",
                ),
            };

            incidents_html.push_str(&format!(
                r#"<div class="p-3.5 bg-slate-950 border {border_color} rounded-xl flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                    <div class="flex items-center gap-2.5">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold border {badge_color}">{evt_type}</span>
                        <span class="text-slate-300 font-semibold">{details}</span>
                    </div>
                    <span class="text-slate-500 text-[11px] font-mono">{ts}</span>
                </div>"#,
                border_color = border_color,
                badge_color = badge_color,
                evt_type = rullst_core::html::escape_str(&evt.event_type),
                ts = rullst_core::html::escape_str(&evt.timestamp_str),
                details = rullst_core::html::escape_str(&evt.details)
            ));
        }
        incidents_html.push_str("</div>");
    }

    let content = format!(
        r#"<div class="p-6 md:p-8 font-mono space-y-8 max-w-7xl mx-auto overflow-y-auto">
            <header class="pb-6 border-b border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                    <h1 class="text-2xl md:text-3xl font-extrabold text-amber-400 flex items-center gap-3">
                        <span>🛡️ Visual Threat Radar & AI Security</span>
                    </h1>
                    <p class="text-slate-400 text-xs md:text-sm mt-1">Bounded counters and local in-process security events</p>
                </div>
                <div class="flex items-center gap-3">
                    <span class="px-3.5 py-1.5 bg-emerald-950/80 text-emerald-400 border border-emerald-800/80 rounded-full text-xs font-bold flex items-center gap-2 shadow-inner">
                        <span class="w-2 h-2 rounded-full bg-emerald-400 animate-ping"></span>
                        Local telemetry snapshot
                    </span>
                </div>
            </header>

            <!-- 9 Live Metric Cards Grid -->
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5 md:gap-6">
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🔒 Secret Log Redactor</div>
                    <div class="text-3xl font-bold text-emerald-400 mt-1">{log_redactions}</div>
                    <div class="text-xs text-slate-400 mt-2">Supported redaction operations recorded locally</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🧬 Zero-Trust Fingerprints</div>
                    <div class="text-3xl font-bold text-cyan-400 mt-1">{zero_trust_mismatches}</div>
                    <div class="text-xs text-slate-400 mt-2">Hijack &amp; subnet drift interventions</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🛡️ Schema Guard Intercepts</div>
                    <div class="text-3xl font-bold text-amber-400 mt-1">{schema_violations}</div>
                    <div class="text-xs text-slate-400 mt-2">JSON depth &amp; payload size limits</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🔑 Subresource Integrity (SRI)</div>
                    <div class="text-3xl font-bold text-purple-400 mt-1">{sri_signed}</div>
                    <div class="text-xs text-slate-400 mt-2">SHA-384 asset signature tags</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🔐 TOTP MFA Validations</div>
                    <div class="text-3xl font-bold text-emerald-400 mt-1">{mfa_verifications}</div>
                    <div class="text-xs text-slate-400 mt-2">RFC 6238 2FA authentications</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🍯 Dynamic Deception Traps</div>
                    <div class="text-3xl font-bold text-rose-400 mt-1">{deception_hits}</div>
                    <div class="text-xs text-slate-400 mt-2">Decoy route hits (/.env, /admin)</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🌐 CSWSH Protection</div>
                    <div class="text-3xl font-bold text-yellow-400 mt-1">{cswsh_blocks}</div>
                    <div class="text-xs text-slate-400 mt-2">Cross-Site WebSocket hijacks blocked</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">⚡ Sliding-Window Rate Limit</div>
                    <div class="text-3xl font-bold text-cyan-400 mt-1">{rate_limit_blocks}</div>
                    <div class="text-xs text-slate-400 mt-2">IP bucket throttles enforced</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">📡 Local SIEM-Candidate Alerts</div>
                    <div class="text-3xl font-bold text-emerald-400 mt-1">{siem_dispatches}</div>
                    <div class="text-xs text-slate-400 mt-2">Bounded in-process events; no external delivery</div>
                </div>
            </div>

            <!-- Live Security Incident Feed -->
            <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-lg font-bold text-slate-200 flex items-center gap-2">
                        <span>Local Security Event Stream</span>
                    </h2>
                    <span class="text-xs text-slate-500 font-mono">Bounded in-memory source</span>
                </div>
                {incidents_html}
            </div>

            {ai_setup_box}

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md">
                    <h2 class="text-lg font-bold text-cyan-400 mb-4 flex items-center gap-2">
                        <span>🤖 rullst-ai Guardrails</span>
                    </h2>
                    <p class="text-xs text-slate-400 mb-4">{provider_observation}</p>
                    <div class="space-y-3 text-sm">
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Prompt Injection Filter</span>
                            <span class="text-xs text-slate-500 font-bold">Not observed</span>
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">LLM Output PII Masking</span>
                            <span class="text-xs text-slate-500 font-bold">Not observed</span>
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Token Rate-Limit Quota</span>
                            <span class="text-xs text-slate-500 font-bold">Not observed</span>
                        </div>
                    </div>
                </div>

                <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md">
                    <h2 class="text-lg font-bold text-amber-400 mb-4 flex items-center gap-2">
                        <span>🔒 rullst-security Built-ins</span>
                    </h2>
                    <div class="space-y-3 text-sm">
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Double-Submit Cookie CSRF</span>
                            <span class="text-xs text-slate-500 font-bold">Verify composition</span>
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Parametrized SQLx ORM</span>
                            <span class="text-xs text-slate-500 font-bold">Bindings available</span>
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Leaky Bucket Rate Limiter</span>
                            <span class="text-xs text-slate-500 font-bold">See local counter</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>"#,
        log_redactions = log_redactions,
        zero_trust_mismatches = zero_trust_mismatches,
        schema_violations = schema_violations,
        sri_signed = sri_signed,
        mfa_verifications = mfa_verifications,
        deception_hits = deception_hits,
        cswsh_blocks = cswsh_blocks,
        rate_limit_blocks = rate_limit_blocks,
        siem_dispatches = siem_dispatches,
        incidents_html = incidents_html,
        ai_setup_box = ai_setup_box,
        provider_observation = provider_observation
    );

    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

// ─── AI Provider Detection ──────────────────────────────────────────────────

fn detect_ai_provider() -> (bool, String) {
    if let Ok(key) = std::env::var("GEMINI_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "Google Gemini API".to_string());
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "OpenAI (ChatGPT / GPT-4o)".to_string());
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "Anthropic Claude".to_string());
    }
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "DeepSeek".to_string());
    }
    if let Ok(host) = std::env::var("OLLAMA_HOST")
        && !host.trim().is_empty()
    {
        return (true, "Ollama".to_string());
    }
    (false, "No AI Provider Configured".to_string())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn security_page_does_not_infer_active_controls_from_configuration() {
        let response = handle_studio_tools_security(axum::http::HeaderMap::new())
            .await
            .into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("security page body");
        let html = String::from_utf8(body.to_vec()).expect("UTF-8 security page");

        assert!(html.contains("Local telemetry snapshot"));
        assert!(html.contains("Not observed"));
        assert!(!html.contains("Production Shield Active"));
        assert!(!html.contains("SQL-Injection Safe"));
        assert!(!html.contains("Connection successful"));
    }
}
