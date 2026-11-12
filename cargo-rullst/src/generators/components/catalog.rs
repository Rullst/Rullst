//! The components `cargo rullst ui add` copies: Rust sources for `src/ui/`
//! and, for the four that need client behaviour, a same-origin ES module for
//! `static/ui/`. The sources under `kit/` are compiled and rendered by this
//! module's tests, so a copied component always builds.

/// One component.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Component {
    pub name: &'static str,
    pub summary: &'static str,
    /// `src/ui/<name>.rs`.
    pub source: &'static str,
    /// `static/ui/<name>.js`, loaded by the component's markup.
    pub script: Option<&'static str>,
    /// Components whose functions this one calls (`super::<name>`).
    pub requires: &'static [&'static str],
}

impl Component {
    pub(crate) fn source_path(&self) -> String {
        format!("src/ui/{}.rs", self.name)
    }

    pub(crate) fn script_path(&self) -> Option<String> {
        self.script.map(|_| format!("static/ui/{}.js", self.name))
    }
}

pub(crate) const COMPONENTS: [Component; 11] = [
    Component {
        name: "button",
        summary: "buttons and button links: variants, sizes, htmx loading spinner",
        source: include_str!("kit/button.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "badge",
        summary: "status labels in five tones",
        source: include_str!("kit/badge.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "alert",
        summary: "inline messages with role=\"alert\" or role=\"status\"",
        source: include_str!("kit/alert.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "card",
        summary: "a surface with heading, description, body and footer",
        source: include_str!("kit/card.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "form",
        summary: "CSRF-aware form, labelled fields with aria-describedby and aria-invalid errors",
        source: include_str!("kit/form.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "table",
        summary: "a captioned table in a focusable scroll region, with an empty state",
        source: include_str!("kit/table.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "pagination",
        summary: "page links with aria-current and gaps",
        source: include_str!("kit/pagination.rs"),
        script: None,
        requires: &[],
    },
    Component {
        name: "dialog",
        summary: "a native modal <dialog> opened with commandfor/command",
        source: include_str!("kit/dialog.rs"),
        script: Some(include_str!("kit/static/dialog.js")),
        requires: &["button"],
    },
    Component {
        name: "menu",
        summary: "a dropdown menu on the popover API with arrow-key navigation",
        source: include_str!("kit/menu.rs"),
        script: Some(include_str!("kit/static/menu.js")),
        requires: &["button"],
    },
    Component {
        name: "tabs",
        summary: "the ARIA tabs pattern with arrow keys, Home and End",
        source: include_str!("kit/tabs.rs"),
        script: Some(include_str!("kit/static/tabs.js")),
        requires: &[],
    },
    Component {
        name: "toast",
        summary: "an aria-live notification region fed by htmx HX-Trigger events",
        source: include_str!("kit/toast.rs"),
        script: Some(include_str!("kit/static/toast.js")),
        requires: &[],
    },
];

/// The component names, in catalog order (the `ui add` values).
pub(crate) const NAMES: [&str; 11] = [
    "button",
    "badge",
    "alert",
    "card",
    "form",
    "table",
    "pagination",
    "dialog",
    "menu",
    "tabs",
    "toast",
];

pub(crate) fn find(name: &str) -> Option<&'static Component> {
    COMPONENTS.iter().find(|component| component.name == name)
}

/// The components to copy for `names`, with the ones they require, in
/// catalog order, and the names added only as requirements. `Err` holds the
/// first unknown name.
pub(crate) fn resolve<'a>(
    names: impl IntoIterator<Item = &'a str>,
) -> Result<(Vec<&'static Component>, Vec<&'static str>), String> {
    let mut wanted = Vec::new();
    for name in names {
        let component = find(name).ok_or_else(|| name.to_owned())?;
        wanted.push(component.name);
    }
    let mut required = Vec::new();
    for name in &wanted {
        for requirement in find(name).map_or(&[][..], |component| component.requires) {
            if !wanted.contains(requirement) && !required.contains(requirement) {
                required.push(*requirement);
            }
        }
    }
    let selected = COMPONENTS
        .iter()
        .filter(|component| wanted.contains(&component.name) || required.contains(&component.name))
        .collect();
    let added = NAMES
        .iter()
        .copied()
        .filter(|name| required.contains(name))
        .collect();
    Ok((selected, added))
}
