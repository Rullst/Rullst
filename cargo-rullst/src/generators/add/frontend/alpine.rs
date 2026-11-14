//! The plan of `cargo rullst add alpine`.

use super::layout::{self, Search};
use super::project::Project;
use super::{Change, Download, FrontendError, Kit, Manual, Plan, read};
use crate::generators::frontend::{ALPINE_COMPONENTS, ALPINE_VENDORED, alpine, alpine_release};
use std::path::Path;

/// The example component file. Alpine's CSP build evaluates directive
/// values with its own parser instead of `new Function`, so logic lives in
/// components registered with `Alpine.data` in a same-origin file.
pub(crate) const COMPONENTS_JS: &str = r#"// Alpine.js components (`cargo rullst add alpine`). Loaded before the
// vendored CSP build (static/vendor/alpine-csp.min.js), which runs under
// Rullst's Content-Security-Policy because it never evaluates strings with
// eval or new Function. Its parser accepts simple expressions in directives
// (property access, calls, + - * / %, comparisons, && || !, ?:, = and ++/--)
// but no arrow functions, template literals, destructuring, spread, `;`
// between statements, globals (window, document, console, Math, JSON) or
// x-html: put that logic in a component here.
// Rules and limits: https://rullst.github.io/Rullst/book/frontend.html
// and https://alpinejs.dev/advanced/csp
//
// Use in html! (since 13.0.0-alpha.5 it accepts Alpine attribute names such
// as @click and :disabled with string-literal values; an `x-bind` object such
// as `clear` below keeps several directives together):
//
//   <div x-data="charCounter(160)">
//       <textarea name="message" x-model="text"></textarea>
//       <p aria-live="polite" x-text="remaining + ' characters left'"></p>
//       <button type="button" x-bind="clear">"Clear"</button>
//   </div>
document.addEventListener("alpine:init", () => {
  Alpine.data("charCounter", (limit = 160) => ({
    text: "",
    limit,
    get remaining() {
      return this.limit - this.text.length;
    },
    // Directives bound with x-bind="clear".
    clear: {
      ["@click"]() {
        this.text = "";
      },
      [":disabled"]() {
        return this.text.length === 0;
      },
    },
  }));
});
"#;

/// The lines the layout's `<head>` gains: components first, so their
/// `alpine:init` listener exists when the CSP build starts Alpine.
pub(crate) const SCRIPT_LINES: [&str; 2] = [
    r#"<script defer src={rullst::assets::url("js/alpine-components.js")}></script>"#,
    r#"<script defer src={rullst::assets::url("vendor/alpine-csp.min.js")}></script>"#,
];

/// The markup of [`COMPONENTS_JS`]'s example, printed for a page.
pub(crate) const EXAMPLE: &str = r#"<div x-data="charCounter(160)">
    <label for="message">"Message"</label>
    <textarea id="message" name="message" x-model="text"></textarea>
    <p aria-live="polite" x-text="remaining + ' characters left'"></p>
    <button type="button" x-bind="clear">"Clear"</button>
</div>"#;

fn kit() -> &'static Kit {
    &super::KITS[1]
}

pub(crate) fn plan(root: &Path, project: &Project) -> Result<Plan, FrontendError> {
    let mut plan = Plan {
        kit: kit(),
        changes: Vec::new(),
        download: None,
        manual: Vec::new(),
        notes: Vec::new(),
    };
    match read(root, ALPINE_VENDORED)? {
        Some(existing) if alpine::is_pinned(&existing) => plan.notes.push(format!(
            "{ALPINE_VENDORED} is already @alpinejs/csp {}",
            alpine_release::VERSION
        )),
        existing => {
            plan.download = Some(Download::AlpineCsp {
                replaces: existing.is_some(),
            });
        }
    }
    match read(root, ALPINE_COMPONENTS)? {
        Some(_) => plan
            .notes
            .push(format!("{ALPINE_COMPONENTS} exists; kept as it is")),
        None => plan.changes.push(Change {
            path: ALPINE_COMPONENTS.to_owned(),
            before: None,
            after: COMPONENTS_JS.to_owned(),
            summary: "create the example charCounter component (Alpine.data)".to_owned(),
        }),
    }
    scripts(root, project, &mut plan);
    plan.manual.push(Manual {
        place: "a page (optional example; native <details>, <dialog> and popover need no script)"
            .to_owned(),
        code: EXAMPLE.to_owned(),
    });
    Ok(plan)
}

fn scripts(root: &Path, project: &Project, plan: &mut Plan) {
    if let Some(file) = layout::mentioning(root, &["vendor/alpine-csp.min.js"]) {
        plan.notes.push(format!("{file} already loads Alpine"));
        return;
    }
    let search = layout::find(root);
    let edit = match &search {
        Search::Found(found) if project.supports_assets => {
            layout::insert_before_head(found, &SCRIPT_LINES).map(|after| (found, after))
        }
        _ => None,
    };
    if let Some((found, after)) = edit {
        plan.changes.push(Change {
            path: found.path.clone(),
            before: Some(found.text.clone()),
            after,
            summary: "load the components and the CSP build (deferred, same origin)".to_owned(),
        });
        return;
    }
    plan.manual.push(super::tailwind::manual_link(
        &search,
        project,
        &SCRIPT_LINES.join("\n"),
    ));
}
