//! The text `cargo rullst ui add` and `cargo rullst ui list` print.

use super::Facts;
use super::catalog::{COMPONENTS, Component};
use super::plan::Plan;
use crate::ui::style::{self, Style};

/// What [`render`] reports.
pub(super) struct Inputs<'a> {
    pub components: &'a [&'static Component],
    /// Components copied only because others use them.
    pub added: &'a [&'static str],
    pub plan: &'a Plan,
    pub facts: Facts,
    pub dry_run: bool,
    pub force: bool,
    pub style: Style,
}

fn note(out: &mut String, style: Style, text: &str) {
    out.push_str(&format!(
        "  {} {}\n",
        style.paint("·", style::MUTED),
        style.paint(text, style::MUTED)
    ));
}

/// `cargo rullst ui list`.
pub(super) fn list(style: Style) -> String {
    let mut out = format!(
        "{} {}\n",
        style.bold("◆", style::ACCENT),
        style.bold(
            "UI components (cargo rullst ui add <name>...)",
            style::BRIGHT
        )
    );
    for component in &COMPONENTS {
        let mut files = component.source_path();
        if let Some(script) = component.script_path() {
            files.push_str(&format!(" + {script}"));
        }
        if !component.requires.is_empty() {
            files.push_str(&format!(", uses {}", component.requires.join(", ")));
        }
        out.push_str(&format!(
            "  {:<11} {}\n  {:<11} {}\n",
            style.paint(component.name, style::PASS),
            component.summary,
            "",
            style.paint(&files, style::MUTED)
        ));
    }
    out.push_str(&format!(
        "\n  {}\n",
        style.paint(&format!("Guide: {}", super::DOCS), style::MUTED)
    ));
    out
}

pub(super) fn render(inputs: &Inputs<'_>) -> String {
    let style = inputs.style;
    let names: Vec<&str> = inputs
        .components
        .iter()
        .map(|component| component.name)
        .collect();
    let mut out = format!(
        "{} {}\n",
        style.bold("◆", style::ACCENT),
        style.bold(
            &format!("cargo rullst ui add {}", names.join(" ")),
            style::BRIGHT
        )
    );
    let mark = if inputs.dry_run { "~" } else { "✓" };
    for change in &inputs.plan.changes {
        let summary = if inputs.dry_run {
            format!("would {}", change.summary)
        } else {
            change.summary.clone()
        };
        out.push_str(&format!(
            "  {} {:<26} {}\n",
            style.paint(mark, style::PASS),
            change.path,
            style.paint(&summary, style::MUTED)
        ));
    }
    if !inputs.added.is_empty() {
        note(
            &mut out,
            style,
            &format!(
                "also copied, because the others use it: {}",
                inputs.added.join(", ")
            ),
        );
    }
    if !inputs.plan.current.is_empty() {
        note(
            &mut out,
            style,
            &format!("already up to date: {}", inputs.plan.current.join(", ")),
        );
    }
    if !inputs.force && !inputs.plan.modified.is_empty() {
        out.push_str(&format!(
            "  {} {}\n",
            style.paint("!", style::WARN),
            style.paint(
                &format!(
                    "{} differ from this CLI's copies; without --force they are kept and the command stops.",
                    inputs.plan.modified.join(", ")
                ),
                style::WARN
            )
        ));
    }
    if inputs.plan.changes.is_empty() {
        note(
            &mut out,
            style,
            "Nothing to change: the project already has them.",
        );
    }
    notes(&mut out, inputs);
    if inputs.dry_run {
        diff(&mut out, inputs);
    }
    if !inputs.plan.manual.is_empty() {
        out.push_str(&format!(
            "\n{}\n",
            style.bold("Wire it in (not edited automatically)", style::BRIGHT)
        ));
        for (place, code) in &inputs.plan.manual {
            out.push_str(&format!(
                "  {}\n    {code}\n",
                style.paint(&format!("{place}:"), style::MUTED)
            ));
        }
    }
    if inputs.dry_run {
        out.push_str(&format!(
            "\n{}\n",
            style.paint(
                "Dry run: nothing was written. Run the same command without --dry-run to apply it.",
                style::MUTED
            )
        ));
    } else {
        out.push_str(&format!(
            "\n  {}\n",
            style.paint(
                &format!("Use them as crate::ui::<name>; examples: {}", super::DOCS),
                style::MUTED
            )
        ));
    }
    out
}

/// The project facts that change what the copies need.
fn notes(out: &mut String, inputs: &Inputs<'_>) {
    let style = inputs.style;
    let facts = inputs.facts;
    if !facts.tailwind {
        out.push_str(&format!(
            "  {} {}\n",
            style.paint("!", style::WARN),
            style.paint(
                "This project does not use Tailwind CSS: the components work and stay accessible, but render unstyled. Run `cargo rullst add tailwind` to style them (it scans src/**/*.rs, src/ui/ included).",
                style::WARN
            )
        ));
    } else if !facts.managed_tailwind {
        note(
            out,
            style,
            "Tailwind is built by the project's own setup: make sure its sources include src/ui/*.rs.",
        );
    }
    let scripted = inputs
        .components
        .iter()
        .any(|component| component.script.is_some());
    if scripted && !facts.supports_assets {
        out.push_str(&format!(
            "  {} {}\n",
            style.paint("!", style::WARN),
            style.paint(
                "The locked rullst is older than 13.0.0-alpha.5, which added rullst::assets::url (used to load static/ui/*.js): run `cargo rullst upgrade` first.",
                style::WARN
            )
        ));
    }
    if inputs
        .components
        .iter()
        .any(|component| component.name == "toast")
    {
        note(
            out,
            style,
            "toast: render ui::toast::region once per page, in the layout; handlers send ui::toast::trigger(...) as HX-Trigger.",
        );
    }
}

fn diff(out: &mut String, inputs: &Inputs<'_>) {
    let diff_style = crate::term::Style {
        color: !inputs.style.is_plain(),
    };
    for change in &inputs.plan.changes {
        let header = match change.before {
            Some(_) => format!("--- {0}\n+++ {0} (planned)", change.path),
            None => format!("--- /dev/null\n+++ {} (new file)", change.path),
        };
        out.push_str(&format!(
            "\n{}\n",
            inputs.style.paint(&header, style::BRIGHT)
        ));
        let (text, _) = crate::diff::render(
            change.before.as_deref().unwrap_or_default(),
            &change.after,
            diff_style,
        );
        out.push_str(&text);
    }
}
