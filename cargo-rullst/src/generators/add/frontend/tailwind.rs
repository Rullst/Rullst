//! The plan of `cargo rullst add tailwind`.

use super::layout::{self, Search};
use super::project::Project;
use super::{Change, Download, FrontendError, Kit, Manual, Plan, Toolchain, read};
use crate::generators::frontend::tailwind::{binary_path, unsupported_host};
use crate::generators::frontend::{
    Fetcher, TAILWIND_INPUT, TAILWIND_OUTPUT, docker, has_managed_tailwind, imports_tailwind,
    tailwind_release, uses_tailwind,
};
use std::path::Path;

/// The input stylesheet `add tailwind` creates.
pub(crate) const INPUT_CSS: &str = r#"/* Tailwind CSS input, created by `cargo rullst add tailwind`.
 * The pinned standalone CLI builds it into static/app.css: `cargo rullst dev`
 * rebuilds it on every change, `cargo rullst build` and the Dockerfile minify
 * it. Link it with `rullst::assets::url("app.css")`.
 *
 * `source(none)` turns off automatic detection, so only the paths below are
 * scanned (not static/vendor or target/). */
@import "tailwindcss" source(none);

/* Class names written in html! and other Rust source. A class assembled at
 * run time, such as format!("text-{color}-600"), is not found: write each
 * complete name (text-red-600, text-green-600) somewhere in the source. Add
 * an @source line for templates kept elsewhere. */
@source "../../src/**/*.rs";
"#;

/// The line the layout's `<head>` gains.
pub(crate) const LINK_LINE: &str =
    r#"<link rel="stylesheet" href={rullst::assets::url("app.css")} />"#;

/// The `.gitignore` block for the built stylesheet.
const GITIGNORE_BLOCK: &str = "# Built from assets/css/app.css by the Tailwind CSS CLI (`cargo rullst add tailwind`)\n/static/app.css\n";

/// Whether the tracked `static/app.css` shows that the project commits
/// built assets (`git ls-files` lists it).
fn output_is_tracked(root: &Path) -> bool {
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch", "--", TAILWIND_OUTPUT])
        .current_dir(root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Whether a `.gitignore` already ignores the built stylesheet by name.
pub(crate) fn ignores_output(gitignore: &str) -> bool {
    gitignore.lines().map(str::trim).any(|line| {
        matches!(
            line,
            "/static/app.css" | "static/app.css" | "/static/" | "static/" | "/static" | "static"
        )
    })
}

fn kit() -> &'static Kit {
    &super::KITS[0]
}

pub(crate) fn plan<F: Fetcher>(
    root: &Path,
    project: &Project,
    toolchain: &Toolchain<'_, F>,
) -> Result<Plan, FrontendError> {
    let asset = toolchain.tailwind.ok_or_else(unsupported_host)?;
    let mut plan = Plan {
        kit: kit(),
        changes: Vec::new(),
        download: None,
        manual: Vec::new(),
        notes: Vec::new(),
    };
    match read(root, TAILWIND_INPUT)? {
        Some(css) if imports_tailwind(&css) => plan.notes.push(format!(
            "{TAILWIND_INPUT} already imports tailwindcss; kept as it is"
        )),
        Some(_) => {
            return Err(FrontendError::Conflict {
                file: TAILWIND_INPUT,
                why: "exists and does not import tailwindcss",
                fix: "Add `@import \"tailwindcss\" source(none);` and `@source \"../../src/**/*.rs\";` at its top (or move the file), then run the command again.",
            });
        }
        None => plan.changes.push(Change {
            path: TAILWIND_INPUT.to_owned(),
            before: None,
            after: INPUT_CSS.to_owned(),
            summary: "create the input: @import \"tailwindcss\", @source src/**/*.rs".to_owned(),
        }),
    }
    if read(root, TAILWIND_OUTPUT)?.is_some_and(|css| !css.starts_with("/*! tailwindcss")) {
        return Err(FrontendError::Conflict {
            file: TAILWIND_OUTPUT,
            why: "exists and was not built by Tailwind CSS; the build would overwrite it",
            fix: "Move its rules to assets/css/app.css (after the @import) or rename it, then run the command again.",
        });
    }
    if uses_tailwind(root) && !has_managed_tailwind(root) {
        plan.notes.push(
            "the project also builds Tailwind itself (tailwind.config.* or package.json); keep one of the two setups"
                .to_owned(),
        );
    }
    gitignore(root, &mut plan)?;
    link(root, project, &mut plan);
    dockerfile(root, &mut plan)?;
    let cached = toolchain
        .cache_base
        .as_deref()
        .is_some_and(|base| binary_path(base, asset).is_file());
    plan.download = Some(Download::TailwindCli { cached });
    Ok(plan)
}

/// Ignores the built stylesheet unless the project tracks it (then it
/// commits built assets and keeps doing so).
fn gitignore(root: &Path, plan: &mut Plan) -> Result<(), FrontendError> {
    let Some(before) = read(root, ".gitignore")? else {
        return Ok(());
    };
    if ignores_output(&before) {
        return Ok(());
    }
    if output_is_tracked(root) {
        plan.notes.push(format!(
            "{TAILWIND_OUTPUT} is tracked by Git, so it stays committed (not added to .gitignore)"
        ));
        return Ok(());
    }
    let mut after = before.clone();
    if !after.is_empty() && !after.ends_with('\n') {
        after.push('\n');
    }
    if !after.is_empty() {
        after.push('\n');
    }
    after.push_str(GITIGNORE_BLOCK);
    plan.changes.push(Change {
        path: ".gitignore".to_owned(),
        before: Some(before),
        after,
        summary: "ignore the built /static/app.css".to_owned(),
    });
    Ok(())
}

/// The layout link, or the line to paste.
fn link(root: &Path, project: &Project, plan: &mut Plan) {
    if let Some(file) =
        layout::mentioning(root, &["assets::url(\"app.css\")", "\"/static/app.css\""])
    {
        plan.notes
            .push(format!("{file} already links the stylesheet"));
        return;
    }
    let search = layout::find(root);
    let edit = match &search {
        Search::Found(found) if project.supports_assets => {
            layout::insert_before_head(found, &[LINK_LINE]).map(|after| (found, after))
        }
        _ => None,
    };
    if let Some((found, after)) = edit {
        plan.changes.push(Change {
            path: found.path.clone(),
            before: Some(found.text.clone()),
            after,
            summary: "link /static/app.css with rullst::assets::url".to_owned(),
        });
        return;
    }
    plan.manual.push(manual_link(&search, project, LINK_LINE));
}

/// Where to paste `line` when the layout was not edited.
pub(crate) fn manual_link(search: &Search, project: &Project, line: &str) -> Manual {
    let upgrade = if project.supports_assets {
        String::new()
    } else {
        format!(
            " (after `cargo rullst upgrade`: rullst {} has no rullst::assets, added in 13.0.0-alpha.5)",
            project.locked.as_deref().unwrap_or("?")
        )
    };
    match search {
        Search::Found(found) => Manual {
            place: format!("{}, inside <head>{upgrade}", found.path),
            code: line.to_owned(),
        },
        Search::Ambiguous(files) => Manual {
            place: format!("the <head> of each layout ({}){upgrade}", files.join(", ")),
            code: line.to_owned(),
        },
        Search::NotFound => Manual {
            place: format!(
                "where pages are rendered: no html! in src/ closes <head>; with render_page*, pass the line as head{upgrade}"
            ),
            code: format!(
                "rullst::htmx::render_page_with_head(&htmx, \"en\", title,\n    rullst::html! {{ {line} }},\n    content)"
            ),
        },
    }
}

fn dockerfile(root: &Path, plan: &mut Plan) -> Result<(), FrontendError> {
    let Some(before) = read(root, "Dockerfile")? else {
        return Ok(());
    };
    if before.contains(docker::STAGES_MARKER) {
        plan.notes
            .push("Dockerfile already builds the stylesheet".to_owned());
        return Ok(());
    }
    match docker::add_to(&before) {
        Some(after) => plan.changes.push(Change {
            path: "Dockerfile".to_owned(),
            before: Some(before),
            after,
            summary: format!(
                "build static/app.css with tailwindcss v{} (checksum-pinned)",
                tailwind_release::VERSION
            ),
        }),
        None => plan.manual.push(Manual {
            place: "Dockerfile (not the generated one): before the runtime stage, then copy /css/app.css to static/app.css in it".to_owned(),
            code: format!("{}{}", docker::stages(), docker::COPY_LINE),
        }),
    }
    Ok(())
}
