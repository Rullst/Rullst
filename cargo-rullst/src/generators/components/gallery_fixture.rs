//! The page of the browser fixture: every component of `cargo rullst ui
//! add`, rendered with Core's `render_page_with_head`.
//! .github/check-ui-components.py copies this file to the fixture's
//! `src/gallery.rs` (next to `mod ui;`); this crate's tests compile and check
//! it against the kit. Its ids are the ones
//! .github/starter-browsers/ui-components.mjs selects.

use super::ui::button::{ButtonOptions, Kind, Size, Variant};
use super::ui::{alert, badge, button, card, dialog, form, menu, pagination, table, tabs, toast};
use rullst::html;
use rullst::html::Markup;
use rullst::htmx::{HtmxRequest, render_page_with_head};
use rullst::response::Html;

fn section(title: &str, body: Markup) -> Markup {
    html! {
        <section class="space-y-4">
            <h2 class="text-xl font-semibold">{title}</h2>
            {body}
        </section>
    }
}

fn buttons(csrf: &str) -> Markup {
    let variants = [
        ("Primary", Variant::Primary),
        ("Secondary", Variant::Secondary),
        ("Outline", Variant::Outline),
        ("Ghost", Variant::Ghost),
        ("Destructive", Variant::Destructive),
        ("Link", Variant::Link),
    ];
    html! {
        <div class="flex flex-wrap items-center gap-3">
            @for (label, variant) in variants {
                {button::button(label, &ButtonOptions { variant, ..Default::default() })}
            }
            {button::button("Small", &ButtonOptions { size: Size::Sm, variant: Variant::Secondary, ..Default::default() })}
            {button::button("Disabled", &ButtonOptions { disabled: true, ..Default::default() })}
            {button::link("Button link", "/?page=2", Variant::Outline, Size::Md)}
        </div>
        <div class="flex flex-wrap items-center gap-3">
            <form id="slow-form" method="post" action="/slow" hx-post="/slow" hx-swap="none" hx-indicator="#slow-button" hx-disabled-elt="#slow-button">
                <input type="hidden" name="_token" value={csrf} />
                {button::button("Save slowly", &ButtonOptions { id: Some("slow-button".to_owned()), kind: Kind::Submit, loading: true, ..Default::default() })}
            </form>
            <form id="toast-form" method="post" action="/toast" hx-post="/toast" hx-swap="none">
                <input type="hidden" name="_token" value={csrf} />
                {button::button("Show a toast", &ButtonOptions { id: Some("toast-button".to_owned()), kind: Kind::Submit, variant: Variant::Secondary, ..Default::default() })}
            </form>
        </div>
    }
}

fn feedback() -> Markup {
    html! {
        <div class="flex flex-wrap gap-2">
            {badge::badge("Neutral", badge::Tone::Neutral)}
            {badge::badge("Info", badge::Tone::Info)}
            {badge::badge("Paid", badge::Tone::Success)}
            {badge::badge("Due soon", badge::Tone::Warning)}
            {badge::badge("Overdue", badge::Tone::Danger)}
        </div>
        <div class="grid gap-3">
            {alert::alert(&alert::AlertOptions { tone: alert::Tone::Info, title: Some("Heads up") }, html! { "The gallery renders every component." })}
            {alert::alert(&alert::AlertOptions { tone: alert::Tone::Success, title: None }, html! { "Everything was saved." })}
            {alert::alert(&alert::AlertOptions { tone: alert::Tone::Warning, title: Some("Check this") }, html! { "Two invoices are due soon." })}
            {alert::alert(&alert::AlertOptions { tone: alert::Tone::Danger, title: Some("Not saved") }, html! { "Fix the highlighted fields." })}
        </div>
        {card::card(
            &card::CardOptions { title: Some("Pro plan"), description: Some("Billed monthly"), id: Some("plan"), ..Default::default() },
            html! { <p>"20 seats, priority support."</p> },
            Some(button::link("Change plan", "/?page=3", Variant::Primary, Size::Sm)),
        )}
    }
}

fn fields(csrf: &str) -> Markup {
    let countries = [("br", "Brazil"), ("pt", "Portugal"), ("ao", "Angola")];
    form::form(
        "/contacts",
        csrf,
        &form::FormOptions {
            id: Some("contact-form"),
            ..Default::default()
        },
        html! {
            {form::field(&form::Field { id: "name", label: "Name", autocomplete: Some("name"), help: Some("As it appears on invoices."), required: true, ..Default::default() })}
            {form::field(&form::Field { id: "email", label: "Email", control: form::Control::Input(form::InputType::Email), value: Some("not-an-address"), error: Some("Enter an address such as name@example.com."), required: true, ..Default::default() })}
            {form::field(&form::Field { id: "message", label: "Message", control: form::Control::Textarea { rows: 3 }, placeholder: Some("How can we help?"), ..Default::default() })}
            {form::field(&form::Field { id: "country", label: "Country", control: form::Control::Select(&countries), value: Some("pt"), ..Default::default() })}
            {form::checkbox(&form::Field { id: "terms", label: "I accept the terms", ..Default::default() }, false)}
            {button::button("Send", &ButtonOptions { kind: Kind::Submit, ..Default::default() })}
        },
    )
}

fn data() -> Markup {
    let columns = [
        table::Column::new("Invoice"),
        table::Column::new("Status"),
        table::Column {
            label: "Total",
            align: table::Align::End,
        },
    ];
    let rows = [("A-1001", "Paid", "120.00"), ("A-1002", "Due", "80.50")]
        .iter()
        .map(|(number, status, total)| {
            vec![html! { {*number} }, html! { {*status} }, html! { {*total} }]
        })
        .collect();
    html! {
        {table::table(&table::TableOptions { id: "invoices", caption: "Invoices", ..Default::default() }, &columns, rows)}
        {table::table(&table::TableOptions { id: "refunds", caption: "Refunds", show_caption: false, empty: "No refunds yet." }, &columns, Vec::new())}
        {pagination::pagination(&pagination::PaginationOptions { page: 3, pages: 12, ..Default::default() }, |page| format!("/?page={page}"))}
    }
}

fn overlays() -> Markup {
    html! {
        <div class="flex flex-wrap items-center gap-3">
            {dialog::trigger("confirm", "Delete project", Variant::Destructive)}
            {menu::menu(&menu::MenuOptions { id: "actions", label: "Actions", ..Default::default() }, &[
                menu::Item::Link { label: "Edit", href: "/?page=4" },
                menu::Item::Link { label: "Duplicate", href: "/?page=5" },
                menu::Item::Separator,
                menu::Item::Submit { label: "Show a toast", form: "toast-form" },
            ])}
        </div>
        {dialog::dialog(
            &dialog::DialogOptions { id: "confirm", title: "Delete the project?", description: Some("Its invoices are kept for five years."), ..Default::default() },
            html! { <p>"Type nothing: this is a gallery."</p> },
            Some(html! {
                {dialog::close("confirm", "Cancel", Variant::Outline)}
                <form method="dialog">
                    <button type="submit" value="confirmed" class={button::classes(Variant::Destructive, Size::Md)}>"Delete"</button>
                </form>
            }),
        )}
        {tabs::tabs(&tabs::TabsOptions { id: "settings", label: "Settings", selected: 0 }, vec![
            tabs::Tab { key: "profile", label: "Profile", panel: html! { <p>"Your name and photo."</p> } },
            tabs::Tab { key: "billing", label: "Billing", panel: html! { <p>"Invoices and payment methods."</p> } },
            tabs::Tab { key: "team", label: "Team", panel: html! { <p>"People who can sign in."</p> } },
        ])}
    }
}

/// The gallery page; Tailwind's stylesheet is linked with the asset helper.
pub fn page(htmx: &HtmxRequest, csrf: &str) -> Html<String> {
    let head = html! {
        <link rel="stylesheet" href={rullst::assets::url("app.css")} />
    };
    render_page_with_head(htmx, "en", "UI components", head, content(csrf))
}

/// The page's content, on a surface that follows the colour scheme (Core's
/// `<html>` is dark).
pub fn content(csrf: &str) -> Markup {
    html! {
        <div class="min-h-full bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100">
            <main class="mx-auto max-w-4xl space-y-12 px-4 py-10">
                <h1 class="text-3xl font-bold">"UI components"</h1>
                {section("Buttons", buttons(csrf))}
                {section("Feedback", feedback())}
                {section("Form", fields(csrf))}
                {section("Data", data())}
                {section("Overlays and tabs", overlays())}
            </main>
            {toast::region(&toast::RegionOptions::default(), &[])}
        </div>
    }
}
