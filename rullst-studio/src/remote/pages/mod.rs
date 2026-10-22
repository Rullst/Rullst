//! Server-rendered pages of the remote mode, in Studio's design system.
//!
//! Every value that came from a remote project or from the project list goes
//! through [`esc`] before it reaches the HTML.

pub(crate) mod alerts;
pub(crate) mod history;
pub(crate) mod monitor;
pub(crate) mod projects;

/// The page being shown, for the navigation highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Projects,
    Overview,
    Routes,
    Queues,
    Queries,
    History,
    Alerts,
}

impl Page {
    pub(crate) fn path(self) -> &'static str {
        match self {
            Self::Projects => "/studio",
            Self::Overview => "/studio/radar",
            Self::Routes => "/studio/routes",
            Self::Queues => "/studio/queues",
            Self::Queries => "/studio/queries",
            Self::History => "/studio/history",
            Self::Alerts => "/studio/alerts",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Projects => "🗂️ Projects",
            Self::Overview => "📡 Overview",
            Self::Routes => "🛣️ Routes",
            Self::Queues => "📬 Queues",
            Self::Queries => "🔁 N+1 Queries",
            Self::History => "📈 History",
            Self::Alerts => "🚨 Alerts",
        }
    }
}

/// Pages shown in the navigation for a selected project.
const PROJECT_PAGES: [Page; 5] = [
    Page::Overview,
    Page::Routes,
    Page::Queues,
    Page::Queries,
    Page::History,
];

/// HTML-escapes text for element content and quoted attributes.
pub(crate) fn esc(value: &str) -> String {
    rullst_core::html::escape_str(value).into_owned()
}

/// Percent-encodes a query value.
pub(crate) fn query_value(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

/// The link to `page` for `project`.
pub(crate) fn page_link(page: Page, project: Option<&str>) -> String {
    match project {
        Some(project) if page != Page::Projects && page != Page::Alerts => {
            format!("{}?project={}", page.path(), query_value(project))
        }
        _ => page.path().to_string(),
    }
}

/// What the header shows: the project names and the selected one.
pub(crate) struct Chrome<'a> {
    pub(crate) projects: &'a [String],
    pub(crate) selected: Option<&'a str>,
    pub(crate) active: Page,
    /// Unacknowledged alerts, shown as a badge.
    pub(crate) open_alerts: u64,
}

/// The Studio page frame for remote mode.
pub(crate) fn layout(chrome: &Chrome<'_>, title: &str, content: &str) -> String {
    let nav_item = |page: Page| {
        let active = if page == chrome.active {
            "bg-slate-800 text-white"
        } else {
            "text-slate-300 hover:text-white hover:bg-slate-800"
        };
        let badge = if page == Page::Alerts && chrome.open_alerts > 0 {
            format!(
                r#" <span class="px-2 py-0.5 rounded-full bg-rose-500/10 text-rose-400 border border-rose-500/30">{}</span>"#,
                chrome.open_alerts
            )
        } else {
            String::new()
        };
        format!(
            r#"<a href="{href}" class="px-3 py-1 rounded-lg {active} transition flex items-center gap-1 whitespace-nowrap"><span>{label}</span>{badge}</a>"#,
            href = esc(&page_link(page, chrome.selected)),
            label = page.label(),
        )
    };
    let mut nav = nav_item(Page::Projects);
    if chrome.selected.is_some() {
        for page in PROJECT_PAGES {
            nav.push_str(&nav_item(page));
        }
    }
    nav.push_str(&nav_item(Page::Alerts));

    let switcher = if chrome.projects.is_empty() {
        String::new()
    } else {
        let target = if PROJECT_PAGES.contains(&chrome.active) {
            chrome.active.path()
        } else {
            Page::Overview.path()
        };
        let options: String = chrome
            .projects
            .iter()
            .map(|name| {
                let selected = if Some(name.as_str()) == chrome.selected {
                    " selected"
                } else {
                    ""
                };
                format!(r#"<option value="{0}"{selected}>{0}</option>"#, esc(name))
            })
            .collect();
        format!(
            r#"<form method="get" action="{target}" class="flex items-center gap-2" aria-label="Project switcher">
                <label for="studio-project" class="text-xs text-slate-400 font-semibold">Project</label>
                <select id="studio-project" name="project" class="bg-slate-950 border border-slate-700 rounded-lg px-2 py-1 text-xs text-slate-100 focus:outline-none focus:border-sky-500">{options}</select>
                <button type="submit" class="px-3 py-1 rounded-lg border border-sky-700 bg-sky-950 text-sky-300 text-xs font-bold hover:border-sky-500 transition">Switch</button>
            </form>"#
        )
    };

    format!(
        r#"<!DOCTYPE html><html lang="en" class="min-h-full bg-slate-950"><head><meta charset="UTF-8" /><meta name="viewport" content="width=device-width, initial-scale=1.0" /><title>{title} · Rullst Studio</title><link href="/studio/assets/studio.css" rel="stylesheet" /></head>
<body class="min-h-screen text-slate-100 flex flex-col antialiased">
<header class="sticky top-0 z-50 flex-shrink-0 bg-slate-900 border-b border-slate-800 px-4 sm:px-6 py-3 flex flex-wrap items-center justify-between shadow-lg gap-3 lg:gap-4">
  <a href="/studio" class="flex items-center gap-2 group">
    <span class="text-2xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">Rullst</span>
    <span class="text-xs font-bold tracking-widest px-2 py-0.5 rounded bg-sky-500/10 text-sky-400 border border-sky-400/20 uppercase">Studio</span>
  </a>
  <nav class="order-3 w-full min-w-0 lg:order-none lg:w-auto flex items-center gap-1 bg-slate-950/80 p-1 rounded-xl border border-slate-800/80 overflow-x-auto text-xs font-semibold">{nav}</nav>
  {switcher}
  <div class="flex items-center gap-2 bg-slate-950 border border-slate-800/80 px-3 py-1 rounded-full text-xs font-medium text-slate-300 whitespace-nowrap">
    <span class="relative flex h-2 w-2"><span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span><span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span></span>
    Remote projects · loopback only
  </div>
</header>
<main id="studio-content" class="w-full min-w-0 flex-grow flex flex-col bg-slate-950"><div class="p-4 sm:p-6 lg:p-8 space-y-6 max-w-7xl w-full mx-auto">{content}</div></main>
</body></html>"#,
        title = esc(title),
    )
}

/// A page heading with a subtitle.
pub(crate) fn heading(title: &str, subtitle: &str) -> String {
    format!(
        r#"<header class="pb-4 border-b border-slate-800"><h1 class="text-2xl md:text-3xl font-extrabold text-white tracking-tight">{}</h1><p class="text-slate-400 text-sm mt-1">{}</p></header>"#,
        esc(title),
        esc(subtitle)
    )
}

/// A status panel: `tone` is `ok`, `warn` or `error`.
pub(crate) fn notice(tone: &str, title: &str, detail: &str) -> String {
    let classes = match tone {
        "ok" => "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
        "warn" => "border-amber-500/30 bg-amber-500/10 text-amber-300",
        _ => "border-rose-500/30 bg-rose-500/10 text-red-300",
    };
    format!(
        r#"<div role="status" class="p-4 rounded-xl border {classes}"><div class="font-bold">{}</div><div class="text-sm mt-1 text-slate-300">{}</div></div>"#,
        esc(title),
        esc(detail)
    )
}

/// A small value card.
pub(crate) fn card(label: &str, value: &str, hint: &str) -> String {
    format!(
        r#"<div class="p-4 bg-slate-900/80 border border-slate-800 rounded-xl shadow-md min-w-0"><div class="text-slate-500 text-xs uppercase font-bold tracking-wider">{}</div><div class="text-2xl font-bold text-sky-300 mt-1 break-all">{}</div><div class="text-xs text-slate-400 mt-1">{}</div></div>"#,
        esc(label),
        esc(value),
        esc(hint)
    )
}

/// A table with escaped header and cell text.
pub(crate) fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let head: String = headers
        .iter()
        .map(|header| format!(r#"<th class="px-3 py-2 text-left">{}</th>"#, esc(header)))
        .collect();
    let body: String = rows
        .iter()
        .map(|row| {
            let cells: String = row
                .iter()
                .map(|cell| format!(r#"<td class="px-3 py-2 break-all">{}</td>"#, esc(cell)))
                .collect();
            format!(r#"<tr class="hover:bg-slate-900">{cells}</tr>"#)
        })
        .collect();
    format!(
        r#"<div class="overflow-x-auto rounded-xl border border-slate-800"><table class="w-full text-xs font-mono border-collapse"><thead class="bg-slate-900 text-slate-400 uppercase"><tr>{head}</tr></thead><tbody class="divide-y divide-slate-800 text-slate-200">{body}</tbody></table></div>"#
    )
}

/// `value` with `decimals` places, or `—`.
pub(crate) fn number(value: Option<f64>, decimals: usize) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.decimals$}"))
        .unwrap_or_else(|| "—".to_string())
}

/// A duration in seconds as days, hours and minutes.
pub(crate) fn duration(seconds: u64) -> String {
    let (days, hours, minutes) = (
        seconds / 86_400,
        seconds % 86_400 / 3_600,
        seconds % 3_600 / 60,
    );
    match (days, hours) {
        (0, 0) => format!("{minutes}m {}s", seconds % 60),
        (0, _) => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h"),
    }
}
