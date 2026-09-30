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
            r#"<div class="nexus-span-empty">
                No active telemetry spans recorded yet. Send HTTP requests or execute ORM queries to stream live microsecond traces.
            </div>"#,
        );
    } else {
        // The collector keeps spans oldest-first; show the most recent ones.
        for s in recorded_spans.iter().rev().take(15) {
            let tone = match s.kind.as_str() {
                "http" => "nexus-tone-cyan",
                "sql" => "nexus-tone-amber",
                "ai" => "nexus-tone-violet",
                _ => "nexus-tone-orange",
            };
            spans_html.push_str(&format!(
                r#"<div class="nexus-span-row">
                    <div>
                        <span class="nexus-event-type {}">{}</span>
                        <span class="nexus-span-name">{}</span>
                    </div>
                    <span class="nexus-span-duration">{} µs</span>
                </div>"#,
                tone,
                rullst_core::html::escape_str(&s.kind),
                rullst_core::html::escape_str(&s.name),
                s.duration_us
            ));
        }
    }

    let mut content = String::new();
    content.push_str(&format!(
        r#"
<div class="nexus-card nexus-stack">
    <div class="nexus-panel-header">
        <div>
            <h2 class="nexus-panel-title nexus-tone-cyan">
                <span>⚡ Telemetry Spans &amp; Microsecond Metrics</span>
                <span class="nexus-badge nexus-badge-tokio">TOKIO MONITOR</span>
            </h2>
            <p class="nexus-panel-lead">Microsecond execution latency, Tokio event loop metrics, RSS memory usage, and OpenTelemetry spans.</p>
        </div>
    </div>

    <!-- 4 Telemetry Metric Cards -->
    <div class="nexus-metric-grid">
        <div class="nexus-metric">
            <div class="nexus-metric-label">Tokio Runtime Latency</div>
            <div class="nexus-metric-value nexus-tone-cyan">{}</div>
            <div class="nexus-metric-hint">Observed scheduler yield</div>
        </div>
        <div class="nexus-metric">
            <div class="nexus-metric-label">RSS RAM Usage (Real Proc)</div>
            <div class="nexus-metric-value nexus-tone-emerald">{}</div>
            <div class="nexus-metric-hint">Platform process probe</div>
        </div>
        <div class="nexus-metric">
            <div class="nexus-metric-label">AI Generation Latency</div>
            <div class="nexus-metric-value nexus-tone-violet">Unavailable</div>
            <div class="nexus-metric-hint">{}</div>
        </div>
        <div class="nexus-metric">
            <div class="nexus-metric-label">OpenTelemetry Exporter</div>
            <div class="nexus-metric-value nexus-tone-slate">Not reported</div>
            <div class="nexus-metric-hint">No exporter health source connected</div>
        </div>
    </div>

    <!-- Active Async Telemetry Spans -->
    <div class="nexus-box">
        <h3 class="nexus-box-title">Active Async Telemetry Spans</h3>
        <div class="nexus-feed">
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

#[cfg(test)]
mod tests {
    use super::*;
    use rullst_core::telemetry_spans::{TraceSpan, global_span_collector};

    #[tokio::test]
    async fn telemetry_lists_the_most_recent_spans() {
        for index in 0..20 {
            global_span_collector().record(TraceSpan {
                name: format!("nexus-span-order-{index:02}"),
                kind: "job".to_string(),
                duration_us: 10,
                timestamp: 1_700_000_000,
            });
        }
        let state = Arc::new(NexusState {
            registry: Arc::new(Vec::new()),
            brand: Arc::new("Nexus".to_string()),
            audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
        });
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("hx-request", axum::http::HeaderValue::from_static("true"));
        let Html(html) = nexus_telemetry_page(State(state), headers).await;

        assert!(html.contains("nexus-span-order-19"));
        assert!(html.contains("nexus-span-order-05"));
        assert!(!html.contains("nexus-span-order-04"));
        let newest = html.find("nexus-span-order-19").expect("newest span");
        let older = html.find("nexus-span-order-18").expect("older span");
        assert!(newest < older, "the newest span is listed first");
    }
}
