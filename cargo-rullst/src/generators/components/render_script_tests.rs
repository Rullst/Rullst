//! The components with a same-origin module (dialog, menu, tabs, toast), a
//! page with every component under the strict Content-Security-Policy, and
//! checks of the sources and modules themselves.

use super::{attribute, button, dialog, ids, menu, tabs, toast};
use rullst::html;
use rullst::html::Markup;

#[test]
fn dialogs_are_named_commanded_and_load_their_module() {
    let options = dialog::DialogOptions {
        id: "delete",
        title: "Delete?",
        description: Some("It cannot be undone."),
        ..Default::default()
    };
    let html = dialog::dialog(
        &options,
        html! { <p>"Body"</p> },
        Some(dialog::close("delete", "Cancel", button::Variant::Outline)),
    );
    assert_eq!(
        attribute(&html, "dialog", "aria-labelledby"),
        Some("delete-title")
    );
    assert_eq!(
        attribute(&html, "dialog", "aria-describedby"),
        Some("delete-description")
    );
    assert!(html.contains("<h2 id=\"delete-title\""));
    assert!(html.contains("<p id=\"delete-description\""));
    assert_eq!(
        html.matches("commandfor=\"delete\" command=\"close\"")
            .count(),
        2
    );
    assert!(html.contains("aria-label=\"Close\""));
    assert!(html.contains("m-auto") && html.contains("backdrop:bg-slate-950/60"));
    assert!(html.ends_with("<script type=\"module\" src=\"/static/ui/dialog.js\"></script>"));
    assert!(!html.contains(" open"));

    let trigger = dialog::trigger("delete", "Delete", button::Variant::Destructive);
    assert!(
        trigger.contains("commandfor=\"delete\" command=\"show-modal\" aria-haspopup=\"dialog\"")
    );
    let plain = dialog::dialog(&dialog::DialogOptions::default(), Markup::default(), None);
    assert!(!plain.contains("aria-describedby") && !plain.contains("border-t"));
}

#[test]
fn menus_are_popovers_of_menu_items() {
    let html = menu::menu(
        &menu::MenuOptions {
            id: "account",
            label: "Account",
            ..Default::default()
        },
        &[
            menu::Item::Link {
                label: "Profile",
                href: "/profile",
            },
            menu::Item::Separator,
            menu::Item::Submit {
                label: "Sign out",
                form: "logout",
            },
        ],
    );
    assert!(html.contains("id=\"account-button\" popovertarget=\"account\" aria-haspopup=\"menu\" aria-controls=\"account\" aria-expanded=\"false\""));
    assert!(
        html.contains(
            "<div id=\"account\" popover role=\"menu\" aria-labelledby=\"account-button\""
        )
    );
    assert!(html.contains("<a role=\"menuitem\" tabindex=\"-1\" href=\"/profile\""));
    assert!(
        html.contains("<button role=\"menuitem\" tabindex=\"-1\" type=\"submit\" form=\"logout\"")
    );
    assert!(html.contains("<div role=\"separator\""));
    assert!(html.ends_with("src=\"/static/ui/menu.js\"></script>"));
}

#[test]
fn tabs_render_the_selected_tab_and_hide_the_other_panels() {
    let html = tabs::tabs(
        &tabs::TabsOptions {
            id: "settings",
            label: "Settings",
            selected: 1,
        },
        vec![
            tabs::Tab {
                key: "profile",
                label: "Profile",
                panel: html! { "P" },
            },
            tabs::Tab {
                key: "billing",
                label: "Billing",
                panel: html! { "B" },
            },
        ],
    );
    assert!(html.contains("<div role=\"tablist\" aria-label=\"Settings\""));
    assert!(html.contains("id=\"settings-tab-profile\" aria-controls=\"settings-panel-profile\" aria-selected=\"false\" tabindex=\"-1\""));
    assert!(html.contains("id=\"settings-tab-billing\" aria-controls=\"settings-panel-billing\" aria-selected=\"true\" tabindex=\"0\""));
    assert!(html.contains("id=\"settings-panel-profile\" aria-labelledby=\"settings-tab-profile\" tabindex=\"0\" hidden"));
    assert!(html.contains("id=\"settings-panel-billing\" aria-labelledby=\"settings-tab-billing\" tabindex=\"0\" class"));
    assert!(html.ends_with("src=\"/static/ui/tabs.js\"></script>"));

    let clamped = tabs::tabs(
        &tabs::TabsOptions {
            selected: 9,
            ..Default::default()
        },
        vec![tabs::Tab {
            key: "only",
            label: "Only",
            panel: Markup::default(),
        }],
    );
    assert!(clamped.contains("aria-selected=\"true\""));
    assert!(tabs::tabs(&tabs::TabsOptions::default(), Vec::new()).contains("role=\"tablist\""));
}

#[test]
fn toasts_have_a_live_region_templates_and_an_ascii_trigger() {
    assert_eq!(
        toast::trigger("Saved", toast::Tone::Success),
        r#"{"ui:toast":{"message":"Saved","tone":"success"}}"#
    );
    let escaped = toast::trigger("\"Ol\u{e1}\" \\ \u{1f389}\n", toast::Tone::Error);
    assert_eq!(
        escaped,
        "{\"ui:toast\":{\"message\":\"\\\"Ol\\u00e1\\\" \\\\ \\ud83c\\udf89\\u000a\",\"tone\":\"error\"}}"
    );
    assert!(escaped.is_ascii());
    let value: serde_json::Value = serde_json::from_str(&escaped).unwrap();
    assert_eq!(value["ui:toast"]["message"], "\"Ol\u{e1}\" \\ \u{1f389}\n");
    assert!(rullst::http::HeaderValue::from_str(&escaped).is_ok());

    let region = toast::region(
        &toast::RegionOptions::default(),
        &[("Welcome <back>", toast::Tone::Info)],
    );
    assert!(region.starts_with("<section aria-label=\"Notifications\""));
    assert!(
        region.contains("<div data-ui-toasts aria-live=\"polite\" aria-relevant=\"additions\"")
    );
    assert!(region.contains("Welcome &lt;back&gt;"));
    for tone in toast::Tone::ALL {
        assert!(region.contains(&format!(
            "<template data-ui-toast-template=\"{}\"><div data-ui-toast data-tone=\"{0}\"",
            tone.name()
        )));
    }
    assert_eq!(region.matches("aria-label=\"Dismiss\"").count(), 5);
    assert!(region.ends_with("src=\"/static/ui/toast.js\"></script>"));
}

#[test]
fn every_component_stays_inside_the_strict_content_security_policy() {
    // The browser fixture's page (it renders every component).
    let page = super::gallery::content("token").into_string();
    // No inline style or handler attributes, no inline script.
    assert!(!page.contains(" style="), "{page}");
    let handler = regex::Regex::new(r#"\son[a-z]+=""#).unwrap();
    assert!(!handler.is_match(&page), "an on* handler attribute");
    assert!(!page.contains("javascript:"));
    let scripts: Vec<&str> = page
        .match_indices("<script")
        .map(|(at, _)| &page[at..])
        .collect();
    assert_eq!(scripts.len(), 4);
    for script in scripts {
        assert!(
            script.starts_with("<script type=\"module\" src=\"/static/ui/")
                && script.contains("\"></script>"),
            "{script}"
        );
    }
    // Every id is unique and every reference names one of them.
    let ids = ids(&page);
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "{ids:?}");
    for name in [
        "aria-labelledby",
        "aria-describedby",
        "aria-controls",
        "for",
        "commandfor",
        "popovertarget",
    ] {
        for (at, _) in page.match_indices(&format!(" {name}=\"")) {
            let start = at + name.len() + 3;
            let value = &page[start..start + page[start..].find('"').unwrap()];
            for id in value.split(' ') {
                assert!(ids.contains(&id), "{name}={id} names no element");
            }
        }
    }
    for selector in ["hx-indicator", "hx-disabled-elt"] {
        for (at, _) in page.match_indices(&format!(" {selector}=\"#")) {
            let start = at + selector.len() + 4;
            let id = &page[start..start + page[start..].find('"').unwrap()];
            assert!(ids.contains(&id), "{selector}=#{id} names no element");
        }
    }
    // Every component is on the page.
    for marker in [
        "role=\"tablist\"",
        "<dialog",
        "role=\"menu\"",
        "data-ui-toasts",
        "<table",
        "<nav",
        "role=\"alert\"",
        "rounded-full",
        "<form method=\"post\" action=\"/contacts\"",
    ] {
        assert!(page.contains(marker), "{marker}");
    }
}

#[test]
fn the_modules_never_evaluate_strings_or_write_html() {
    for component in super::super::catalog::COMPONENTS.iter() {
        let Some(script) = component.script else {
            continue;
        };
        for forbidden in [
            "eval(",
            "new Function",
            "innerHTML",
            "outerHTML",
            "insertAdjacentHTML",
            "document.write",
            "setAttribute(\"style\"",
            "setAttribute(\"on",
            "import(",
        ] {
            assert!(
                !script.contains(forbidden),
                "{}: {forbidden}",
                component.name
            );
        }
        assert!(script.contains("same-origin module"), "{}", component.name);
    }
}

#[test]
fn the_sources_have_no_panicking_calls() {
    for component in super::super::catalog::COMPONENTS.iter() {
        // The needles of generators::tests' runtime-literal audit, exactly.
        for forbidden in [
            ".unwrap(",
            ".expect(",
            "panic!(",
            "unreachable!(",
            "todo!(",
            "unimplemented!(",
            "unsafe",
        ] {
            assert!(
                !component.source.contains(forbidden),
                "{}: {forbidden}",
                component.name
            );
        }
    }
}
