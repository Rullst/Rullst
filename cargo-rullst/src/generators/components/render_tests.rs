//! The copied components compile against Core (`rullst` is `rullst_core` in
//! this crate's tests) and render the markup their documentation promises:
//! ARIA wiring, ids that match, CSRF tokens, scripts loaded from `/static/ui/`
//! and nothing the strict Content-Security-Policy would block.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[allow(dead_code)]
#[path = "kit/alert.rs"]
mod alert;
#[allow(dead_code)]
#[path = "kit/badge.rs"]
mod badge;
#[allow(dead_code)]
#[path = "kit/button.rs"]
mod button;
#[allow(dead_code)]
#[path = "kit/card.rs"]
mod card;
#[allow(dead_code)]
#[path = "kit/dialog.rs"]
mod dialog;
#[allow(dead_code)]
#[path = "kit/form.rs"]
mod form;
#[allow(dead_code)]
#[path = "kit/menu.rs"]
mod menu;
#[allow(dead_code)]
#[path = "kit/pagination.rs"]
mod pagination;
#[allow(dead_code)]
#[path = "kit/table.rs"]
mod table;
#[allow(dead_code)]
#[path = "kit/tabs.rs"]
mod tabs;
#[allow(dead_code)]
#[path = "kit/toast.rs"]
mod toast;

use rullst::html;
use rullst::html::Markup;

/// The value of attribute `name` in the first `<tag …>` of `html` that has it.
fn attribute<'a>(html: &'a str, tag: &str, name: &str) -> Option<&'a str> {
    html.match_indices(&format!("<{tag}"))
        .filter_map(|(start, _)| {
            let end = start + html[start..].find('>')?;
            let element = &html[start..end];
            let at = element.find(&format!(" {name}=\""))? + name.len() + 3;
            let value_end = at + element[at..].find('"')?;
            Some(&element[at..value_end])
        })
        .next()
}

/// Every `id="…"` of `html`.
fn ids(html: &str) -> Vec<&str> {
    html.match_indices(" id=\"")
        .map(|(at, _)| {
            let start = at + 5;
            &html[start..start + html[start..].find('"').unwrap()]
        })
        .collect()
}

#[test]
fn buttons_write_their_type_state_and_loading_spinner() {
    let plain = button::button("Save", &button::ButtonOptions::default());
    assert!(plain.starts_with("<button type=\"button\" class=\""));
    assert!(plain.contains("bg-sky-700"));
    assert!(!plain.contains(" disabled "));
    assert!(!plain.contains("<svg"));
    assert!(plain.ends_with("<span>Save</span></button>"));

    let loading = button::button(
        "Save <now>",
        &button::ButtonOptions {
            variant: button::Variant::Destructive,
            size: button::Size::Lg,
            kind: button::Kind::Submit,
            id: Some("save".to_owned()),
            disabled: true,
            loading: true,
        },
    );
    assert!(loading.starts_with("<button type=\"submit\" id=\"save\" disabled class=\""));
    assert!(loading.contains("bg-red-700") && loading.contains("h-12"));
    assert!(loading.contains("htmx-indicator hidden"));
    assert!(loading.contains("[.htmx-request_&]:inline-block"));
    assert!(loading.contains("Save &lt;now&gt;"));
    assert!(loading.contains("aria-hidden=\"true\""));

    for variant in [
        button::Variant::Primary,
        button::Variant::Secondary,
        button::Variant::Outline,
        button::Variant::Ghost,
        button::Variant::Destructive,
        button::Variant::Link,
    ] {
        let classes = button::classes(variant, button::Size::Icon);
        assert!(classes.contains("focus-visible:outline-2"), "{variant:?}");
        assert!(classes.contains("size-10"));
    }
    let link = button::link("Docs", "/docs", button::Variant::Link, button::Size::Sm);
    assert!(link.starts_with("<a href=\"/docs\" class=\""));
    assert!(link.contains("text-sky-700"));
}

#[test]
fn badges_alerts_and_cards_carry_their_roles_and_headings() {
    assert!(badge::badge("Paid", badge::Tone::Success).contains(">Paid</span>"));
    assert!(badge::classes(badge::Tone::Danger).contains("text-red-800"));

    let error = alert::alert(
        &alert::AlertOptions {
            tone: alert::Tone::Danger,
            title: Some("Not saved"),
        },
        html! { <p>"Check the fields."</p> },
    );
    assert!(error.starts_with("<div role=\"alert\""));
    assert!(error.contains("<p class=\"font-semibold\">Not saved</p>"));
    let notice = alert::alert(&alert::AlertOptions::default(), html! { "Saved" });
    assert!(notice.starts_with("<div role=\"status\""));
    assert!(!notice.contains("font-semibold"));

    let titled = card::card(
        &card::CardOptions {
            title: Some("Plan"),
            description: Some("Monthly"),
            heading: card::Heading::H3,
            id: Some("plan"),
        },
        html! { <p>"Pro"</p> },
        Some(html! { <a href="/upgrade">"Upgrade"</a> }),
    );
    assert!(titled.starts_with("<section id=\"plan\" aria-labelledby=\"plan-title\""));
    assert!(titled.contains("<h3 id=\"plan-title\""));
    assert!(titled.contains("Monthly") && titled.contains("border-t"));
    let plain = card::card(&card::CardOptions::default(), html! { "Body" }, None);
    assert!(plain.starts_with("<div class=\""));
    assert!(!plain.contains("<h2") && !plain.contains("border-t"));
}

#[test]
fn form_fields_link_their_label_help_and_error() {
    let field = form::field(&form::Field {
        id: "email",
        label: "Email",
        control: form::Control::Input(form::InputType::Email),
        value: Some("a@b.example"),
        autocomplete: Some("email"),
        help: Some("We never share it."),
        error: Some("Enter an address."),
        required: true,
        ..Default::default()
    });
    let html = field.as_str();
    assert_eq!(attribute(html, "label", "for"), Some("email"));
    assert_eq!(attribute(html, "input", "type"), Some("email"));
    assert_eq!(attribute(html, "input", "name"), Some("email"));
    assert_eq!(
        attribute(html, "input", "aria-describedby"),
        Some("email-help email-error")
    );
    assert_eq!(attribute(html, "input", "aria-invalid"), Some("true"));
    assert!(html.contains(" required "));
    assert!(html.contains("border-red-600"));
    assert!(html.contains("<p id=\"email-help\"") && html.contains("<p id=\"email-error\""));
    assert!(html.contains("aria-hidden=\"true\"> *</span>"));

    let valid = form::field(&form::Field {
        id: "bio",
        name: "profile[bio]",
        label: "Bio",
        control: form::Control::Textarea { rows: 4 },
        value: Some("<hi>"),
        ..Default::default()
    });
    assert!(!valid.contains("aria-invalid") && !valid.contains("aria-describedby"));
    assert!(valid.contains("name=\"profile[bio]\" rows=\"4\""));
    assert!(valid.contains(">&lt;hi&gt;</textarea>"));

    let choices = [("br", "Brazil"), ("pt", "Portugal")];
    let select = form::field(&form::Field {
        id: "country",
        label: "Country",
        control: form::Control::Select(&choices),
        value: Some("pt"),
        help: Some("Where you live."),
        ..Default::default()
    });
    assert!(select.contains("<option value=\"br\">Brazil</option>"));
    assert!(select.contains("<option value=\"pt\" selected>Portugal</option>"));
    assert_eq!(
        attribute(&select, "select", "aria-describedby"),
        Some("country-help")
    );

    let checkbox = form::checkbox(
        &form::Field {
            id: "terms",
            label: "I accept the terms",
            error: Some("Required."),
            ..Default::default()
        },
        true,
    );
    assert!(checkbox.contains("type=\"checkbox\" id=\"terms\" name=\"terms\" checked"));
    assert_eq!(
        attribute(&checkbox, "input", "aria-describedby"),
        Some("terms-error")
    );
}

#[test]
fn forms_send_the_csrf_token_first_and_only_for_post() {
    let post = form::form(
        "/contacts",
        "token123",
        &form::FormOptions {
            boost: true,
            multipart: true,
            ..Default::default()
        },
        html! { <input type="file" name="avatar" /> },
    );
    assert!(post.starts_with("<form method=\"post\" action=\"/contacts\" enctype=\"multipart/form-data\" hx-boost=\"true\""));
    let token = post.find("name=\"_token\" value=\"token123\"").unwrap();
    assert!(token < post.find("type=\"file\"").unwrap());

    let get = form::form(
        "/search",
        "token123",
        &form::FormOptions {
            method: form::Method::Get,
            ..Default::default()
        },
        Markup::default(),
    );
    assert!(get.starts_with("<form method=\"get\" action=\"/search\" class="));
    assert!(!get.contains("_token") && !get.contains("hx-boost"));
}

#[test]
fn tables_name_their_scroll_region_and_show_an_empty_state() {
    let columns = [
        table::Column::new("Number"),
        table::Column {
            label: "Total",
            align: table::Align::End,
        },
    ];
    let filled = table::table(
        &table::TableOptions {
            id: "invoices",
            caption: "Invoices",
            ..Default::default()
        },
        &columns,
        vec![vec![html! { "A-1" }, html! { "10.00" }]],
    );
    assert!(
        filled.starts_with(
            "<div role=\"region\" aria-labelledby=\"invoices-caption\" tabindex=\"0\""
        )
    );
    assert!(filled.contains("<caption id=\"invoices-caption\""));
    assert_eq!(filled.matches("<th scope=\"col\"").count(), 2);
    assert!(filled.contains("text-right tabular-nums\">10.00</td>"));
    assert!(!filled.contains("colspan"));

    let empty = table::table(
        &table::TableOptions {
            caption: "Nothing",
            show_caption: false,
            ..Default::default()
        },
        &columns,
        Vec::new(),
    );
    assert!(empty.contains("class=\"sr-only\">Nothing</caption>"));
    assert!(empty.contains("<td colspan=\"2\""));
    assert!(empty.contains("Nothing to show yet."));
}

#[test]
fn pagination_windows_mark_gaps_and_the_current_page() {
    use pagination::window;
    let pages = |items: Vec<Option<u32>>| {
        items
            .iter()
            .map(|item| item.map_or("…".to_owned(), |page| page.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(pages(window(1, 1)), "1");
    assert_eq!(pages(window(3, 7)), "1 2 3 4 5 6 7");
    assert_eq!(pages(window(1, 20)), "1 2 3 4 5 … 20");
    assert_eq!(pages(window(4, 20)), "1 2 3 4 5 … 20");
    assert_eq!(pages(window(5, 20)), "1 … 4 5 6 … 20");
    assert_eq!(pages(window(10, 20)), "1 … 9 10 11 … 20");
    assert_eq!(pages(window(17, 20)), "1 … 16 17 18 19 20");
    assert_eq!(pages(window(20, 20)), "1 … 16 17 18 19 20");
    assert_eq!(pages(window(4, 8)), "1 2 3 4 5 … 8");
    assert_eq!(pages(window(5, 8)), "1 … 4 5 6 7 8");
    assert_eq!(pages(window(99, 8)), "1 … 4 5 6 7 8");
    assert_eq!(pages(window(0, 0)), "");

    let options = pagination::PaginationOptions {
        page: 2,
        pages: 3,
        ..Default::default()
    };
    let nav = pagination::pagination(&options, |page| format!("/items?page={page}"));
    assert!(nav.starts_with("<nav aria-label=\"Pagination\">"));
    assert!(nav.contains("href=\"/items?page=1\" rel=\"prev\""));
    assert!(nav.contains("href=\"/items?page=3\" rel=\"next\""));
    assert!(nav.contains("href=\"/items?page=2\" aria-current=\"page\" aria-label=\"Page 2\""));
    assert_eq!(nav.matches("aria-current").count(), 1);

    let first = pagination::PaginationOptions {
        page: 1,
        pages: 2,
        ..Default::default()
    };
    let first = pagination::pagination(&first, |page| format!("?p={page}"));
    assert!(!first.contains("rel=\"prev\"") && first.contains("rel=\"next\""));
    let single = pagination::PaginationOptions::default();
    assert!(pagination::pagination(&single, |page| page.to_string()).is_empty());
}

/// The kit as the browser fixture's page reaches it (`super::ui`).
mod ui {
    pub(super) use super::{
        alert, badge, button, card, dialog, form, menu, pagination, table, tabs, toast,
    };
}
/// The browser fixture's page, compiled against the kit.
#[allow(dead_code)]
#[path = "gallery_fixture.rs"]
mod gallery;

// Components with a module, the full page and the sources.
#[path = "render_script_tests.rs"]
mod scripted;
