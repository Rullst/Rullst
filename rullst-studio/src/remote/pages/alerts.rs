//! The alerts page: recent alerts and each project's watch, rules and webhook.

use super::{esc, heading, notice, table};
use crate::remote::alerts::AlertRules;
use crate::remote::history::AlertRow;
use crate::remote::projects::Project;

fn optional<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn checked(value: bool) -> &'static str {
    if value { " checked" } else { "" }
}

const INPUT: &str = "w-full bg-slate-950 border border-slate-700 rounded-lg px-2 py-1 text-slate-100 focus:outline-none focus:border-sky-500";

fn settings_form(project: &Project, form_token: &str) -> String {
    let rules: &AlertRules = &project.rules;
    let webhook = if project.webhook.is_some() {
        "configured — leave empty to keep"
    } else if project.webhook_configured {
        "stored but unreadable — enter it again"
    } else {
        "not configured"
    };
    let number_input = |name: &str, label: &str, value: String| {
        format!(
            r#"<label class="text-xs text-slate-400 font-semibold space-y-1"><span class="block">{label}</span><input name="{name}" inputmode="decimal" value="{}" placeholder="off" class="{INPUT}" /></label>"#,
            esc(&value)
        )
    };
    format!(
        r#"<form method="post" action="/studio/alerts/settings" class="p-4 bg-slate-900/80 border border-slate-800 rounded-xl space-y-3" autocomplete="off">
  <input type="hidden" name="form_token" value="{form_token}" />
  <input type="hidden" name="name" value="{name}" />
  <div class="flex flex-wrap items-center gap-4"><span class="text-lg font-bold text-sky-300">{name}</span>
    <label class="text-xs text-slate-300 flex items-center gap-2"><input type="checkbox" name="watch" value="on"{watch} /> Watch (poll and keep history)</label>
    <label class="text-xs text-slate-300 flex items-center gap-2"><input type="checkbox" name="unreachable" value="on"{unreachable} /> Alert when unreachable</label></div>
  <div class="grid grid-cols-1 md:grid-cols-3 gap-3">{error_rate}{p95}{pending}{failures}{cooldown}
    <label class="text-xs text-slate-400 font-semibold space-y-1"><span class="block">Webhook (https://, {webhook})</span><input name="webhook" type="password" maxlength="2048" class="{INPUT}" /></label></div>
  <div class="flex items-center gap-4"><label class="text-xs text-slate-300 flex items-center gap-2"><input type="checkbox" name="clear_webhook" value="on" /> Remove the webhook</label>
    <button type="submit" class="px-4 py-1 rounded-lg border border-sky-700 bg-sky-950 text-sky-300 text-xs font-bold hover:border-sky-500 transition">Save</button></div>
</form>"#,
        form_token = esc(form_token),
        name = esc(&project.name),
        watch = checked(project.watch),
        unreachable = checked(rules.unreachable),
        error_rate = number_input(
            "error_rate_percent",
            "5xx rate ≥ (%)",
            optional(rules.error_rate_percent)
        ),
        p95 = number_input("p95_ms", "p95 latency ≥ (ms)", optional(rules.p95_ms)),
        pending = number_input(
            "queue_pending",
            "Pending jobs ≥",
            optional(rules.queue_pending)
        ),
        failures = number_input(
            "queue_failures",
            "Queue failures per poll ≥",
            optional(rules.queue_failures)
        ),
        cooldown = number_input(
            "cooldown_minutes",
            "Cool-down (minutes)",
            rules.cooldown_minutes.to_string()
        ),
        webhook = esc(webhook),
    )
}

/// The alerts page. `message` is a fixed status.
pub(crate) fn page(
    alerts: Option<&[AlertRow]>,
    projects: &[Project],
    form_token: &str,
    message: Option<(&str, &str, &str)>,
    poll_seconds: u64,
) -> String {
    let mut out = heading(
        "Alerts",
        &format!(
            "Watched projects are read every {poll_seconds} s while this Studio runs; rules fire at most once per cool-down."
        ),
    );
    if let Some((tone, title, detail)) = message {
        out.push_str(&notice(tone, title, detail));
    }
    match alerts {
        None => out.push_str(&notice(
            "warn",
            "History is unavailable",
            "The history database in your data directory could not be opened; alerts are not evaluated.",
        )),
        Some([]) => out.push_str(&notice("ok", "No alerts", "No rule has fired.")),
        Some(alerts) => {
            let rows: Vec<Vec<String>> = alerts
                .iter()
                .map(|alert| {
                    vec![
                        alert.at.to_string(),
                        alert.project.clone(),
                        alert.rule.clone(),
                        alert.message.clone(),
                        alert.delivery.clone(),
                        if alert.acknowledged { "yes" } else { "no" }.to_string(),
                    ]
                })
                .collect();
            out.push_str(&table(
                &["Unix time", "Project", "Rule", "Message", "Delivery", "Acknowledged"],
                &rows,
            ));
            out.push_str(&format!(
                r#"<form method="post" action="/studio/alerts/acknowledge"><input type="hidden" name="form_token" value="{}" /><button type="submit" class="px-4 py-1 rounded-lg border border-slate-700 text-slate-300 text-xs font-bold hover:border-sky-500 transition">Acknowledge all</button></form>"#,
                esc(form_token)
            ));
        }
    }
    if projects.is_empty() {
        out.push_str(&notice(
            "ok",
            "No projects yet",
            "Add a project on the Projects page first.",
        ));
    }
    for project in projects {
        out.push_str(&settings_form(project, form_token));
    }
    out
}
