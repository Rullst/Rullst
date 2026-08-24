use axum::{extract::State, response::Html};
use std::sync::Arc;

use crate::nexus::ai_chat::detect_ai_provider;
use crate::nexus::types::NexusState;
use crate::nexus::ui::{render_shell, render_sidebar};

/// GET /nexus/telemetry — Microsecond Telemetry & Async Spans in Rullst Nexus.
#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_telemetry_page(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let (ai_active, provider_name) = detect_ai_provider();
    let ai_metric_sub = if ai_active {
        format!("Provider configured: {provider_name}; no generation sampled")
    } else {
        "No LLM provider configured".to_string()
    };

    let snapshot = rullst_core::radar::RadarSnapshot::collect_async().await;
    let rss_metric = snapshot
        .memory_rss_mb
        .map(|value| format!("{value:.1} MB"))
        .unwrap_or_else(|| "Unavailable".to_string());
    let tokio_latency = snapshot
        .tokio_latency_micros
        .map(|value| format!("{value} µs"))
        .unwrap_or_else(|| "Unavailable".to_string());

    let recorded_spans = rullst_core::telemetry_spans::global_span_collector().snapshot();
    let mut spans_html = String::new();

    if recorded_spans.is_empty() {
        spans_html.push_str(
            r#"<div style="background: var(--bg-800); padding: 24px 16px; border-radius: 8px; border: 1px dashed var(--border); text-align: center; color: var(--text-dim); font-size: 13px;">
                No active telemetry spans recorded yet. Send HTTP requests or execute ORM queries to stream live microsecond traces.
            </div>"#,
        );
    } else {
        for s in recorded_spans.iter().take(15) {
            let badge_color = match s.kind.as_str() {
                "http" => "#22d3ee",
                "sql" => "#fbbf24",
                "ai" => "#c084fc",
                _ => "#f59e0b",
            };
            spans_html.push_str(&format!(
                r#"<div style="background: var(--bg-800); padding: 12px 16px; border-radius: 8px; border: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center;">
                    <div>
                        <span style="color: {}; font-weight: 700;">{}</span>
                        <span style="color: var(--text-muted); margin-left: 16px;">{}</span>
                    </div>
                    <span style="color: #34d399; font-weight: 700; font-size: 12px;">{} µs</span>
                </div>"#,
                badge_color,
                rullst_core::html::escape_str(&s.kind),
                rullst_core::html::escape_str(&s.name),
                s.duration_us
            ));
        }
    }

    let mut content = String::new();
    content.push_str(&format!(
        r#"
<div class="nexus-card" style="display: flex; flex-direction: column; gap: 24px;">
    <div style="display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 16px;">
        <div>
            <h2 style="margin: 0; color: #22d3ee; display: flex; align-items: center; gap: 10px; font-size: 20px;">
                <span>⚡ Telemetry Spans &amp; Microsecond Metrics</span>
                <span class="nexus-badge" style="background: rgba(34, 211, 238, 0.2); color: #22d3ee; border: 1px solid rgba(34, 211, 238, 0.4);">TOKIO MONITOR</span>
            </h2>
            <p style="margin: 4px 0 0 0; font-size: 13px; color: var(--text-muted);">Microsecond execution latency, Tokio event loop metrics, RSS memory usage, and OpenTelemetry spans.</p>
        </div>
    </div>

    <!-- 4 Telemetry Metric Cards -->
    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px;">
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;">Tokio Runtime Latency</div>
            <div style="font-size: 32px; font-weight: 800; color: #22d3ee; margin-top: 4px;">{}</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">Observed scheduler yield</div>
        </div>
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;">RSS RAM Usage (Real Proc)</div>
            <div style="font-size: 32px; font-weight: 800; color: #34d399; margin-top: 4px;">{}</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">Platform process probe</div>
        </div>
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;">AI Generation Latency</div>
            <div style="font-size: 32px; font-weight: 800; color: #c084fc; margin-top: 4px;">Unavailable</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">{}</div>
        </div>
        <div style="background: var(--bg-900); padding: 18px; border-radius: 10px; border: 1px solid var(--border);">
            <div style="font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;">OpenTelemetry Exporter</div>
            <div style="font-size: 32px; font-weight: 800; color: #94a3b8; margin-top: 4px;">Not reported</div>
            <div style="font-size: 11px; color: var(--text-dim); margin-top: 4px;">No exporter health source connected</div>
        </div>
    </div>

    <!-- Active Async Telemetry Spans -->
    <div style="background: var(--bg-900); padding: 20px; border-radius: 10px; border: 1px solid var(--border);">
        <h3 style="margin-top: 0; color: var(--text-main); font-size: 15px;">Active Async Telemetry Spans</h3>
        <div style="display: flex; flex-direction: column; gap: 10px; font-size: 13px; font-family: var(--font-mono); margin-top: 14px;">
            {}
        </div>
    </div>
</div>
"#,
        tokio_latency, rss_metric, ai_metric_sub, spans_html
    ));

    if headers.contains_key("hx-request") {
        Html(content)
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, Some("telemetry")),
            &content,
        ))
    }
}
