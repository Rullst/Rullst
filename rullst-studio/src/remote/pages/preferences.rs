//! The settings page: desktop notifications.

use super::{esc, heading, notice};
use crate::remote::preferences::StudioSettings;

const INPUT: &str = "w-full bg-slate-950 border border-slate-700 rounded-lg px-2 py-1 text-slate-100 focus:outline-none focus:border-sky-500";
const BUTTON: &str = "px-4 py-1 rounded-lg border border-sky-700 bg-sky-950 text-sky-300 text-xs font-bold hover:border-sky-500 transition";

fn notifications_form(settings: &StudioSettings, form_token: &str) -> String {
    let notifications = &settings.notifications;
    let checked = if notifications.enabled {
        " checked"
    } else {
        ""
    };
    format!(
        r#"<form method="post" action="/studio/settings/notifications" class="p-4 bg-slate-900/80 border border-slate-800 rounded-xl space-y-3" autocomplete="off">
  <input type="hidden" name="form_token" value="{form_token}" />
  <h2 class="text-lg font-bold text-sky-300">Desktop notifications</h2>
  <p class="text-xs text-slate-400">Alerts of watched projects (the rules on the Alerts page) appear as notifications of your operating system while this Studio runs and one of its pages is open in your browser. A project is notified at most once per quiet period; alerts in between are counted in its next notification.</p>
  <div class="grid grid-cols-1 md:grid-cols-3 gap-3 items-center">
    <label class="text-xs text-slate-300 flex items-center gap-2"><input type="checkbox" name="enabled" value="on"{checked} /> Show alerts as desktop notifications</label>
    <label class="text-xs text-slate-400 font-semibold space-y-1"><span class="block">Quiet period per project (minutes, 1 to 1440)</span><input name="quiet_minutes" inputmode="numeric" value="{quiet}" class="{INPUT}" /></label>
    <button type="submit" class="{BUTTON}">Save</button>
  </div>
  <div class="flex flex-wrap items-center gap-3"><button type="button" id="studio-notify-permission" class="{BUTTON}">Allow in this browser</button><span id="studio-notify-status" class="text-xs text-slate-400">This browser's permission is shown once the page has loaded.</span></div>
</form>"#,
        form_token = esc(form_token),
        quiet = notifications.quiet_minutes,
    )
}

/// The settings page. `message` is a fixed status.
pub(crate) fn page(
    settings: &StudioSettings,
    form_token: &str,
    message: Option<(&str, &str, &str)>,
) -> String {
    let mut out = heading(
        "Settings",
        "Settings of this Studio, kept in its project list on this machine.",
    );
    if let Some((tone, title, detail)) = message {
        out.push_str(&notice(tone, title, detail));
    }
    out.push_str(&notifications_form(settings, form_token));
    out
}
