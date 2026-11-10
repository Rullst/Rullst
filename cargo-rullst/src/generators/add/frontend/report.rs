//! The text `cargo rullst add tailwind|alpine` prints.

use super::{Download, Outcome, Plan};
use crate::generators::frontend::{
    ALPINE_VENDORED, TAILWIND_OUTPUT, alpine_release, tailwind_release,
};
use crate::ui::style::{self, Style};

fn line(style: Style, mark: &str, file: &str, summary: &str) -> String {
    format!(
        "  {} {:<22} {}\n",
        style.paint(mark, style::PASS),
        file,
        style.paint(summary, style::MUTED)
    )
}

fn download_summary(download: Download, outcome: &Outcome, dry_run: bool) -> (String, String) {
    match download {
        Download::TailwindCli { cached } => {
            let file = format!("tailwindcss v{}", tailwind_release::VERSION);
            let summary = match (&outcome.tailwind, dry_run, cached) {
                (Some(verified), _, _) if verified.downloaded => format!(
                    "downloaded and SHA-256 verified: {}",
                    verified.path.display()
                ),
                (Some(verified), _, _) => {
                    format!("cached, SHA-256 verified: {}", verified.path.display())
                }
                (None, true, true) => {
                    "cached in the user cache (verified before each use)".to_owned()
                }
                _ => "would download the pinned CLI into the user cache and verify its SHA-256"
                    .to_owned(),
            };
            (file, summary)
        }
        Download::AlpineCsp { replaces } => {
            let verb = match (dry_run, replaces) {
                (true, true) => "would replace with",
                (true, false) => "would vendor",
                (false, true) => "replaced with",
                (false, false) => "vendored",
            };
            (
                ALPINE_VENDORED.to_owned(),
                format!(
                    "{verb} @alpinejs/csp {} (npm integrity verified)",
                    alpine_release::VERSION
                ),
            )
        }
    }
}

pub(super) fn render(plan: &Plan, outcome: &Outcome, dry_run: bool, style: Style) -> String {
    let mut out = format!(
        "{} {}\n",
        style.bold("◆", style::ACCENT),
        style.bold(
            &format!("cargo rullst add {}", plan.kit.name),
            style::BRIGHT
        )
    );
    let mark = if dry_run { "~" } else { "✓" };
    if let Some(download) = plan.download {
        let (file, summary) = download_summary(download, outcome, dry_run);
        out.push_str(&line(style, mark, &file, &summary));
    }
    for change in &plan.changes {
        let summary = if dry_run {
            format!("would {}", change.summary)
        } else {
            change.summary.clone()
        };
        out.push_str(&line(style, mark, &change.path, &summary));
    }
    match &outcome.built {
        Some(Ok(())) => out.push_str(&line(style, "✓", TAILWIND_OUTPUT, "built by tailwindcss")),
        Some(Err(error)) => out.push_str(&format!(
            "  {} {:<22} {}\n",
            style.paint("!", style::WARN),
            TAILWIND_OUTPUT,
            style.paint(
                &format!("first build failed ({error}); `cargo rullst dev` retries it"),
                style::WARN
            )
        )),
        None => {}
    }
    for note in &plan.notes {
        out.push_str(&format!(
            "  {} {}\n",
            style.paint("·", style::MUTED),
            style.paint(note, style::MUTED)
        ));
    }
    if plan.changes.is_empty()
        && plan
            .download
            .is_none_or(|download| matches!(download, Download::TailwindCli { cached: true }))
    {
        out.push_str(&format!(
            "  {}\n",
            style.paint(
                "Nothing to change: the project already has it.",
                style::MUTED
            )
        ));
    }
    if dry_run {
        let diff_style = crate::term::Style {
            color: !style.is_plain(),
        };
        for change in &plan.changes {
            let header = match change.before {
                Some(_) => format!("--- {0}\n+++ {0} (planned)", change.path),
                None => format!("--- /dev/null\n+++ {} (new file)", change.path),
            };
            out.push_str(&format!("\n{}\n", style.paint(&header, style::BRIGHT)));
            let (diff, _) = crate::diff::render(
                change.before.as_deref().unwrap_or_default(),
                &change.after,
                diff_style,
            );
            out.push_str(&diff);
        }
    }
    if !plan.manual.is_empty() {
        out.push_str(&format!(
            "\n{}\n",
            style.bold("Wire it in (not edited automatically)", style::BRIGHT)
        ));
        for manual in &plan.manual {
            out.push_str(&format!(
                "  {}\n",
                style.paint(&format!("{}:", manual.place), style::MUTED)
            ));
            for code in manual.code.lines() {
                out.push_str(&format!("    {code}\n"));
            }
        }
    }
    if dry_run {
        out.push_str(&format!(
            "\n{}\n",
            style.paint(
                "Dry run: nothing was downloaded or written. Run the same command without --dry-run to apply it.",
                style::MUTED
            )
        ));
    }
    out
}
