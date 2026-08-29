//! HTML views for the loopback-only legacy Studio command server.
//!
//! The richer `rullst-studio` crate owns `/studio/*`. These views remain for
//! `cargo rullst studio` compatibility and must not fabricate live state.

use axum::response::Html;
use std::fmt::Write as _;

pub(crate) fn is_ai_configured() -> bool {
    let _ = dotenvy::dotenv();
    [
        "GEMINI_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "DEEPSEEK_API_KEY",
    ]
    .iter()
    .any(|key| std::env::var(key).is_ok_and(|value| !value.trim().is_empty()))
}

fn layout(title: &str, body: String) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} — Rullst Studio</title>
  <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="min-h-full bg-slate-950 text-slate-100 font-mono">
  <header class="border-b border-slate-800 bg-slate-900/80 px-6 py-4">
    <div class="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-4">
      <a href="/studio" class="font-bold text-sky-400">Rullst Studio · local compatibility UI</a>
      <nav class="flex flex-wrap gap-3 text-xs text-slate-300">
        <a href="/studio/data">Database</a><a href="/studio/ai">AI</a>
        <a href="/studio/telemetry">Telemetry</a><a href="/studio/security">Security</a>
        <a href="/studio/capital">Capital</a><a href="/studio/traces">Spans</a>
      </nav>
    </div>
  </header>
  <main class="mx-auto max-w-7xl space-y-6 p-8">
    <div class="rounded-xl border border-amber-700/50 bg-amber-950/30 p-4 text-sm text-amber-200">
      This command server is bound to 127.0.0.1:5555 for local development. It has no shared-environment authentication mode; do not expose it publicly.
    </div>
    {body}
  </main>
</body>
</html>"#,
        title = crate::html::escape_str(title),
    ))
}

fn page_header(title: &str, description: &str) -> String {
    format!(
        r#"<header class="border-b border-slate-800 pb-5">
  <h1 class="text-3xl font-bold text-white">{title}</h1>
  <p class="mt-2 text-sm text-slate-400">{description}</p>
</header>"#,
        title = crate::html::escape_str(title),
        description = crate::html::escape_str(description),
    )
}

pub(crate) async fn studio_home_handler() -> Html<String> {
    let body = format!(
        r#"{}
<div class="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
  <a href="/studio/data" class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-purple-400">Database guidance</h2><p class="mt-2 text-sm text-slate-400">Review explicit CLI commands; no application registry is inferred.</p></a>
  <a href="/studio/telemetry" class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-sky-400">Process telemetry</h2><p class="mt-2 text-sm text-slate-400">View only the OS and Tokio probes available in this process.</p></a>
  <a href="/studio/traces" class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-indigo-400">Local spans</h2><p class="mt-2 text-sm text-slate-400">View records explicitly submitted to the bounded SpanCollector.</p></a>
  <a href="/studio/security" class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-amber-400">Security boundaries</h2><p class="mt-2 text-sm text-slate-400">Review available controls without simulated incident telemetry.</p></a>
  <a href="/studio/ai" class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-cyan-400">AI configuration</h2><p class="mt-2 text-sm text-slate-400">Check whether a supported provider key is present; this does not test reachability.</p></a>
  <a href="/studio/capital" class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-emerald-400">Capital boundary</h2><p class="mt-2 text-sm text-slate-400">Provider analytics require real application data and are not mocked here.</p></a>
</div>"#,
        page_header(
            "Rullst Studio Control Center",
            "Local command and observability surfaces with explicit unavailable states.",
        )
    );
    layout("Control Center", body)
}

pub(crate) async fn studio_data_handler() -> Html<String> {
    let body = format!(
        r#"{}
<section class="rounded-xl border border-slate-800 bg-slate-900 p-6">
  <p class="text-sm leading-6 text-slate-300">This compatibility server has no application migration or seeder registry. Run these commands from the project root and inspect their terminal output:</p>
  <pre class="mt-4 whitespace-pre-wrap rounded-xl border border-slate-800 bg-slate-950 p-5 text-sm text-slate-200">cargo rullst db:migrate
cargo rullst db:rollback
cargo rullst db:seed</pre>
  <p class="mt-4 text-sm text-amber-300">The legacy POST endpoints return 501 instead of reporting success for an empty registry.</p>
</section>"#,
        page_header(
            "Database Guidance",
            "Use the application CLI where its compiled migration and seeder registries are available.",
        )
    );
    layout("Database", body)
}

pub(crate) async fn studio_security_handler() -> Html<String> {
    let ai_state = if is_ai_configured() {
        "A supported provider key is present; provider reachability and guardrail effectiveness are unverified."
    } else {
        "No supported cloud-provider key was found."
    };
    let body = format!(
        r#"{}
<div class="grid gap-5 md:grid-cols-2">
  <section class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-amber-400">Implemented controls</h2><p class="mt-3 text-sm leading-6 text-slate-300">Core supplies a composable CSRF/WAF/header/PII baseline. The dedicated security crate supplies explicit RASP, DLP, honeypot, RBAC, abuse-control, Vault, and telemetry helpers.</p></section>
  <section class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-cyan-400">AI configuration observation</h2><p class="mt-3 text-sm leading-6 text-slate-300">{ai_state}</p></section>
</div>
<div class="rounded-xl border border-rose-800/60 bg-rose-950/20 p-5 text-sm text-rose-200">This page does not claim that middleware is mounted in an application, that an HMAC chain is externally anchored, or that attacks are prevented. Verify the deployed stack and its event sink.</div>"#,
        page_header(
            "Security Boundaries",
            "Capability summary only; this legacy view has no live security-event source.",
        ),
        ai_state = crate::html::escape_str(ai_state),
    );
    layout("Security", body)
}

fn display_metric<T: std::fmt::Display>(value: Option<T>, suffix: &str) -> String {
    value
        .map(|value| format!("{value}{suffix}"))
        .unwrap_or_else(|| "Unavailable".to_string())
}

pub(crate) async fn studio_telemetry_handler() -> Html<String> {
    let snapshot = crate::radar::RadarSnapshot::collect_async().await;
    let body = format!(
        r#"{}
<div class="grid gap-5 md:grid-cols-2 lg:grid-cols-5">
  <div class="rounded-xl border border-slate-800 bg-slate-900 p-5"><span class="text-xs text-slate-500">TOKIO YIELD</span><strong class="mt-2 block text-xl text-cyan-400">{latency}</strong></div>
  <div class="rounded-xl border border-slate-800 bg-slate-900 p-5"><span class="text-xs text-slate-500">TOKIO TASKS</span><strong class="mt-2 block text-xl text-sky-400">{tasks}</strong></div>
  <div class="rounded-xl border border-slate-800 bg-slate-900 p-5"><span class="text-xs text-slate-500">PROCESS RSS</span><strong class="mt-2 block text-xl text-emerald-400">{rss}</strong></div>
  <div class="rounded-xl border border-slate-800 bg-slate-900 p-5"><span class="text-xs text-slate-500">PROCESS CPU</span><strong class="mt-2 block text-xl text-amber-400">{cpu}</strong></div>
  <div class="rounded-xl border border-slate-800 bg-slate-900 p-5"><span class="text-xs text-slate-500">UPTIME</span><strong class="mt-2 block text-xl text-indigo-400">{uptime}s</strong></div>
</div>
<p class="rounded-xl border border-slate-800 bg-slate-900 p-5 text-sm text-slate-400">These are point-in-time local observations. A scheduler yield is not request latency, and unsupported probes remain unavailable.</p>"#,
        page_header(
            "Process & Tokio Telemetry",
            "A live RadarSnapshot collected for this page request.",
        ),
        latency = display_metric(snapshot.tokio_latency_micros, " µs"),
        tasks = display_metric(snapshot.active_tokio_tasks, ""),
        rss = display_metric(
            snapshot.memory_rss_mb.map(|value| format!("{value:.1}")),
            " MB"
        ),
        cpu = display_metric(
            snapshot
                .cpu_usage_percent
                .map(|value| format!("{value:.1}")),
            "%"
        ),
        uptime = snapshot.uptime_seconds,
    );
    layout("Telemetry", body)
}

pub(crate) async fn studio_ai_handler() -> Html<String> {
    let state = if is_ai_configured() {
        "Configured: a supported provider key is present. No request was sent and reachability was not tested."
    } else {
        "Unconfigured: no supported cloud-provider key is present. Ollama can be selected explicitly with OLLAMA_HOST and OLLAMA_MODEL."
    };
    let body = format!(
        r#"{}
<section class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-cyan-400">Provider configuration</h2><p class="mt-3 text-sm text-slate-300">{state}</p></section>
<section class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-slate-200">Boundary</h2><p class="mt-3 text-sm leading-6 text-slate-400">Prompt-injection and PII checks are heuristics, not authorization or complete DLP. This compatibility page does not submit prompts or display simulated model responses.</p></section>"#,
        page_header(
            "AI Configuration",
            "Environment observation without network calls or mock telemetry.",
        ),
        state = crate::html::escape_str(state),
    );
    layout("AI", body)
}

pub(crate) async fn studio_capital_handler() -> Html<String> {
    let body = format!(
        r#"{}
<section class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-emerald-400">No data source connected</h2><p class="mt-3 text-sm leading-6 text-slate-300">This legacy Core view does not query Capital providers or an application database, so it intentionally shows no MRR, ARR, subscriber, or webhook values.</p></section>
<section class="rounded-xl border border-slate-800 bg-slate-900 p-6"><h2 class="font-bold text-slate-200">Application responsibility</h2><p class="mt-3 text-sm leading-6 text-slate-400">Mount authenticated application-owned analytics and persist verified webhook events before presenting financial figures. Offline mocks prove adapter protocols only.</p></section>"#,
        page_header(
            "Capital Data Boundary",
            "Financial dashboards must be backed by real persisted application data.",
        )
    );
    layout("Capital", body)
}

pub(crate) async fn studio_traces_handler() -> Html<String> {
    let spans = crate::telemetry_spans::global_span_collector().snapshot();
    let mut rows = String::new();
    if spans.is_empty() {
        rows.push_str("<p class=\"text-sm text-slate-400\">No local spans have been recorded. Recording is explicit.</p>");
    } else {
        rows.push_str("<div class=\"space-y-3\">");
        for span in spans.iter().rev().take(50) {
            let _ = write!(
                rows,
                r#"<div class="flex flex-wrap justify-between gap-3 rounded-lg border border-slate-800 bg-slate-950 p-4 text-sm"><span><strong class="text-indigo-400">{kind}</strong> <span class="text-slate-300">{name}</span></span><span class="text-slate-400">{duration} µs · {timestamp}s epoch</span></div>"#,
                kind = crate::html::escape_str(&span.kind),
                name = crate::html::escape_str(&span.name),
                duration = span.duration_us,
                timestamp = span.timestamp,
            );
        }
        rows.push_str("</div>");
    }
    let body = format!(
        r#"{}
<section class="rounded-xl border border-slate-800 bg-slate-900 p-6">{rows}</section>
<p class="text-sm text-slate-500">SpanCollector is a bounded process-local buffer, not persistent or distributed tracing.</p>"#,
        page_header(
            "Local Span Records",
            "Values explicitly submitted to this process's SpanCollector.",
        )
    );
    layout("Spans", body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compatibility_views_do_not_render_fabricated_kpis() {
        let home = studio_home_handler().await.0;
        let telemetry = studio_telemetry_handler().await.0;
        let capital = studio_capital_handler().await.0;

        assert!(!home.contains("&lt; 0.15 ms"));
        assert!(!telemetry.contains("~14 MB"));
        assert!(telemetry.contains("point-in-time local observations"));
        assert!(capital.contains("No data source connected"));
    }
}
