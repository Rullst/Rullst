//! The project list: add and remove projects.

use super::{Page, esc, heading, notice, page_link};
use crate::remote::projects::Project;

/// One row of the list with the status of its last read, if any.
pub(crate) struct ProjectRow<'a> {
    pub(crate) project: &'a Project,
    pub(crate) status: Option<String>,
}

/// The project list page. `message` is a fixed status (never request input).
pub(crate) fn page(
    rows: &[ProjectRow<'_>],
    form_token: &str,
    message: Option<(&str, &str, &str)>,
    readable_by_others: bool,
) -> String {
    let mut out = heading(
        "Remote projects",
        "Applications whose read-only monitoring endpoint this Studio reads. Tokens are encrypted on this machine and sent only to their project.",
    );
    if let Some((tone, title, detail)) = message {
        out.push_str(&notice(tone, title, detail));
    }
    if readable_by_others {
        out.push_str(&notice(
            "warn",
            "The project list is readable by other users",
            "Restrict the project list and its key file to your user (chmod 600).",
        ));
    }
    if rows.is_empty() {
        out.push_str(&notice(
            "ok",
            "No projects yet",
            "Add a project below: its base URL (https://, or http:// to a loopback SSH tunnel) and a monitor token from its RULLST_MONITOR_TOKENS.",
        ));
    } else {
        out.push_str(r#"<div class="space-y-3">"#);
        for row in rows {
            let project = row.project;
            let token = if project.token.is_some() {
                r#"<span class="text-emerald-400">token stored</span>"#
            } else {
                r#"<span class="text-rose-400">token unreadable: remove and add again</span>"#
            };
            let status = row
                .status
                .as_deref()
                .map(|status| {
                    format!(
                        r#"<div class="text-xs text-slate-400 mt-1">{}</div>"#,
                        esc(status)
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!(
                r#"<div class="p-4 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col md:flex-row md:items-center justify-between gap-3">
  <div class="min-w-0"><a href="{href}" class="text-lg font-bold text-sky-300 hover:text-white">{name}</a>
    <div class="text-xs font-mono text-slate-400 break-all">{url}</div>
    <div class="text-xs mt-1">{token}</div>{status}</div>
  <form method="post" action="/studio/projects/remove" class="flex items-center gap-2">
    <input type="hidden" name="form_token" value="{form_token}" />
    <input type="hidden" name="name" value="{name}" />
    <button type="submit" class="px-3 py-1 rounded-lg border border-rose-800 text-rose-400 text-xs font-bold hover:border-rose-500 transition">Remove</button>
  </form>
</div>"#,
                href = esc(&page_link(Page::Overview, Some(&project.name))),
                name = esc(&project.name),
                url = esc(project.base_url.as_str()),
                form_token = esc(form_token),
            ));
        }
        out.push_str("</div>");
    }
    out.push_str(&format!(
        r#"<form method="post" action="/studio/projects" class="p-4 bg-slate-900/80 border border-slate-800 rounded-xl grid grid-cols-1 md:grid-cols-3 gap-3" autocomplete="off">
  <input type="hidden" name="form_token" value="{form_token}" />
  <label class="text-xs text-slate-400 font-semibold space-y-1"><span class="block">Name</span>
    <input name="name" required maxlength="48" pattern="[A-Za-z0-9][A-Za-z0-9._\-]*" placeholder="shop-production" class="w-full bg-slate-950 border border-slate-700 rounded-lg px-2 py-1 text-slate-100 focus:outline-none focus:border-sky-500" /></label>
  <label class="text-xs text-slate-400 font-semibold space-y-1"><span class="block">Base URL</span>
    <input name="base_url" type="url" required maxlength="512" placeholder="https://shop.example" class="w-full bg-slate-950 border border-slate-700 rounded-lg px-2 py-1 text-slate-100 focus:outline-none focus:border-sky-500" /></label>
  <label class="text-xs text-slate-400 font-semibold space-y-1"><span class="block">Monitor token</span>
    <input name="token" type="password" required maxlength="512" class="w-full bg-slate-950 border border-slate-700 rounded-lg px-2 py-1 text-slate-100 focus:outline-none focus:border-sky-500" /></label>
  <div><button type="submit" class="px-4 py-1 rounded-lg border border-emerald-800 bg-emerald-950 text-emerald-300 text-xs font-bold hover:border-emerald-500 transition">Add project</button></div>
</form>"#,
        form_token = esc(form_token),
    ));
    out
}
