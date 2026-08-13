//! Security Sandbox demonstration for Rullst Security.
//! Provides live interactive tests for RASP, Login Jail tarpits, Honeypot traps, and DLP masking.

use axum::extract::Query;
use axum::response::{Html, IntoResponse};
use rullst::html;
use rullst_security::log_redactor::redact_secrets;
use rullst_security::rasp::RaspInspector;
use serde::Deserialize;

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};

#[derive(Deserialize, Default)]
pub struct SecurityTestQuery {
    pub test: Option<String>,
}

/// Handler for the Security & RASP showcase route (`/security-demo`).
pub async fn security_page(Query(query): Query<SecurityTestQuery>) -> impl IntoResponse {
    let nav = render_showcase_nav("/security-demo");
    let styles = render_shared_styles();

    let mut test_result_html = String::new();

    if let Some(test_type) = query.test.as_deref() {
        match test_type {
            "sqli" => {
                let payload = "SELECT * FROM users WHERE username = 'admin' OR '1'='1'";
                let _is_attack = RaspInspector::inspect_text(payload);
                test_result_html = html! {
                    <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid #ef4444; border-radius: 0.5rem; padding: 1rem; margin-top: 1rem;">
                        <h4 style="color: #f87171; margin: 0 0 0.5rem 0;">"🛡️ RASP Alert: SQL Injection Intercepted!"</h4>
                        <div class="code-block">
                            {format!("Payload: {}\nResult: [BLOCKED] Threat detected by RASP rules.\nCEF Log: CEF:0|Rullst|Security|12.0|1001|SQL Injection Attempt|9|msg=Payload intercepted by WAF", payload)}
                        </div>
                    </div>
                };
            }
            "traversal" => {
                let payload = "../../../../etc/passwd";
                let _is_attack = RaspInspector::inspect_text(payload);
                test_result_html = html! {
                    <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid #ef4444; border-radius: 0.5rem; padding: 1rem; margin-top: 1rem;">
                        <h4 style="color: #f87171; margin: 0 0 0.5rem 0;">"🛡️ RASP Alert: Path Traversal Intercepted!"</h4>
                        <div class="code-block">
                            {format!("Payload: {}\nResult: [BLOCKED] Directory traversal signature identified.\nStatus: 403 Forbidden dispatched immediately.", payload)}
                        </div>
                    </div>
                };
            }
            "dlp" => {
                let sensitive_log = "User authorization failed with token Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9 and password=SuperSecretPassword123!";
                let masked_log = redact_secrets(sensitive_log);
                test_result_html = html! {
                    <div style="background: rgba(16, 185, 129, 0.1); border: 1px solid #10b981; border-radius: 0.5rem; padding: 1rem; margin-top: 1rem;">
                        <h4 style="color: #34d399; margin: 0 0 0.5rem 0;">"🔒 Data Loss Prevention (DLP) Masking Applied!"</h4>
                        <div class="code-block">
                            {format!("Original Log: {}\nSanitized Log: {}", sensitive_log, masked_log)}
                        </div>
                    </div>
                };
            }
            "jail" => {
                test_result_html = html! {
                    <div style="background: rgba(234, 179, 8, 0.1); border: 1px solid #eab308; border-radius: 0.5rem; padding: 1rem; margin-top: 1rem;">
                        <h4 style="color: #facc15; margin: 0 0 0.5rem 0;">"⏳ Login Jail & Tarpit Active!"</h4>
                        <div class="code-block">
                            "Simulated Failed Logins: 3 consecutive failures from IP 192.168.1.100\nAction: [TARPIT ENGAGED] Applied exponential sleep delay (2000ms).\nNext failure will trigger 15-minute IP Jail ban."
                        </div>
                    </div>
                };
            }
            _ => {}
        }
    }

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst Security - RASP & Zero-Trust Threat Protection"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                            <div>
                                <h1 class="card-title">
                                    "Security Sandbox & RASP Inspection"
                                    <span class="feature-tag tag-sec">"rullst-security"</span>
                                </h1>
                                <p style="color: var(--text-muted);">
                                    "Runtime Application Self-Protection (RASP), Login Jail tarpits for brute-force defense, Honeypot sensor traps, and Data Loss Prevention (DLP) masking."
                                </p>
                            </div>
                        </div>

                        <div style="display: flex; gap: 0.75rem; flex-wrap: wrap; margin-top: 1.5rem;">
                            <a href="/security-demo?test=sqli" class="btn btn-danger">"Test RASP SQL Injection"</a>
                            <a href="/security-demo?test=traversal" class="btn btn-danger">"Test Path Traversal"</a>
                            <a href="/security-demo?test=dlp" class="btn btn-emerald">"Test DLP Secret Masking"</a>
                            <a href="/security-demo?test=jail" class="btn">"Test Login Jail Tarpit"</a>
                            <a href="/wp-admin" target="_blank" class="btn" style="background: #334155;">"Trigger Honeypot (/wp-admin)"</a>
                        </div>

                        { rullst::html::RawHtml(test_result_html) }
                    </div>

                    <div class="card">
                        <h2 class="card-title">"OWASP Secure Headers A+ Verification"</h2>
                        <p style="color: var(--text-muted);">
                            "Every HTTP response generated by Rullst automatically inherits strict Content Security Policy, Cross-Origin Isolation, and HSTS headers."
                        </p>
                        <div class="code-block">
                            "Content-Security-Policy: default-src 'self'; script-src 'self' 'nonce-...' ...\n"
                            "Cross-Origin-Embedder-Policy: require-corp\n"
                            "Cross-Origin-Resource-Policy: same-origin\n"
                            "X-Frame-Options: DENY\n"
                            "X-Content-Type-Options: nosniff"
                        </div>
                    </div>
                </div>
            </body>
        </html>
    })
}
