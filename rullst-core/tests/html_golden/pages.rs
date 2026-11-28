//! `html!` input copied from the workspace when the cases were recorded: the
//! Benchmarks pages, Studio's error panel and page shell, a UI kit component
//! and tutorial examples, with fixed values in place of request data.

use rullst::html;
use rullst::html::{Markup, RawHtml};

use super::Case;

struct Fortune {
    id: i32,
    message: String,
}

/// `Rullst/Benchmarks` `frameworks/rullst`, `GET /fortunes`.
fn fortunes(fortunes: &[Fortune]) -> Markup {
    html! {
        <!DOCTYPE html>
        <html><head><title>Fortunes</title></head><body><table>
        <tr><th>id</th><th>message</th></tr>
        @for fortune in fortunes {
            <tr><td>{fortune.id}</td><td>{fortune.message}</td></tr>
        }
        </table></body></html>
    }
}

/// `Rullst/Benchmarks` `frameworks/rullst`, `GET /form` and `POST /form`.
fn form_pages(csrf_token: &str, name: &str, random_number: i32) -> Markup {
    let form = html! {
        <!DOCTYPE html>
        <html><head><title>Form</title></head><body>
        <form method="post" action="/form">
            <input type="hidden" name="_token" value={csrf_token} />
            <input type="text" name="name" />
            <input type="number" name="randomNumber" />
            <button type="submit">Send</button>
        </form>
        </body></html>
    };
    let reply = html! {
        <!DOCTYPE html>
        <html><head><title>Form</title></head><body><h1>"Hello, "{name}</h1><p>{random_number}</p></body></html>
    };
    html! { {form}{reply} }
}

/// `rullst-studio` `data_browser/handlers/dashboard.rs`, the error panel.
fn studio_error(error: &str) -> Markup {
    html! {
        <div class="flex-grow flex flex-col items-center justify-center p-8 max-w-2xl mx-auto text-left">
            <div class="w-full bg-slate-900 border border-amber-500/30 p-6 rounded-2xl space-y-4 shadow-xl">
                <div class="flex items-center gap-3 text-amber-400 font-bold text-base">
                    <svg aria-hidden="true" class="h-6 w-6 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                    <span>"Database Connection Offline"</span>
                </div>
                <div class="p-3 bg-slate-950 border border-slate-800 rounded-xl font-mono text-xs text-rose-400 overflow-x-auto">
                    { error.to_string() }
                </div>
                <ul class="text-xs text-slate-300 space-y-1.5 list-disc list-inside">
                    <li>"Verify credentials in your project's .env file (DATABASE_URL)."</li>
                </ul>
            </div>
        </div>
    }
}

/// `rullst-studio` `data_browser/layout.rs`, the page shell (one link of
/// the navigation kept).
fn studio_shell(content: String) -> Markup {
    html! {
        <html lang="en" class="min-h-full bg-slate-950">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>"Rullst Studio Control Center"</title>
            <link href="/studio/assets/studio.css" rel="stylesheet" />
        </head>
        <body class="min-h-screen text-slate-100 flex flex-col antialiased selection:bg-sky-500/30 selection:text-sky-200">
            <nav class="order-3 w-full min-w-0 lg:order-none lg:w-auto flex items-center gap-1.5">
                <a href="/studio/radar" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white">
                    <span>"📡 Radar & Telemetry"</span>
                </a>
            </nav>
            <div class="flex-grow flex min-w-0">
                <main id="studio-content" class="w-full min-w-0 flex-grow flex flex-col bg-slate-950">
                    { RawHtml(content) }
                </main>
            </div>
        </body>
        </html>
    }
}

/// `cargo-rullst` UI kit `badge.rs`.
fn badge(text: &str, tone: &str) -> Markup {
    html! {
        <span class={format!("inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium {tone}")}>{text}</span>
    }
}

/// Tutorial 06 and the guide's view composition example.
fn tutorial(count: i32, names: &[&str]) -> Markup {
    let rows: Markup = names
        .iter()
        .map(|name| html! { <li class="py-2 text-slate-200">{name}</li> })
        .collect();
    let flagged = true;
    let note = flagged.then(|| html! { <em>"flagged"</em> });
    html! {
        <div x-data="{ open: false }" @click.outside="open = false">
            <button @click="open = !open" :aria-expanded="open">Menu ({count})</button>
            <form hx-post="/notes" hx-on::after-request="this.reset()">
                Write a note, then press <kbd>Enter</kbd>.
            </form>
        </div>
        <ul id="user-list" class="divide-y divide-slate-700">
            {rows}
        </ul>
        <p>{note}</p>
    }
}

/// A form field as the UI kit writes one: label, described-by and error.
fn field(name: &str, value: Option<&str>, error: Option<&str>, required: bool) -> Markup {
    let described = error.map(|_| format!("{name}-error"));
    html! {
        <div class="space-y-1">
            <label for={name} class="text-sm font-medium">{name}</label>
            <input id={name} name={name} value?={value} required?={required} aria-invalid?={error.is_some()} aria-describedby?={described.as_deref()} />
            @if let Some(error) = error { <p id={format!("{name}-error")} class="text-red-600">{error}</p> }
        </div>
    }
}

pub fn cases() -> Vec<Case> {
    let mut rows: Vec<Fortune> = [
        (1, "fortune: No such file or directory"),
        (
            2,
            "A computer scientist is someone who fixes things that aren't broken.",
        ),
        (3, "After enough decimal places, nobody gives a damn."),
        (
            11,
            "<script>alert(\"This should not be displayed in a browser alert box.\");</script>",
        ),
        (12, "フレームワークのベンチマーク"),
        (0, "Additional fortune added at request time."),
    ]
    .into_iter()
    .map(|(id, message)| Fortune {
        id,
        message: message.to_string(),
    })
    .collect();
    rows.sort_by(|a, b| a.message.cmp(&b.message));
    vec![
        ("pages/benchmarks-fortunes", fortunes(&rows)),
        ("pages/benchmarks-fortunes-empty", fortunes(&[])),
        (
            "pages/benchmarks-form",
            form_pages("tok&en\"<1>", "<Ana & Bob>", -17),
        ),
        (
            "pages/studio-error",
            studio_error("error returned from database: <pool timed out> & \"retry\""),
        ),
        (
            "pages/studio-shell",
            studio_shell("<section>trusted</section>".to_string()),
        ),
        (
            "pages/ui-badge",
            badge("Paid & <due>", "border-emerald-300 bg-emerald-50"),
        ),
        ("pages/tutorial", tutorial(2, &["Ana", "<Bob>"])),
        (
            "pages/field-valid",
            field("email", Some("a@b.c"), None, true),
        ),
        (
            "pages/field-invalid",
            field("name", None, Some("Name is <required>"), false),
        ),
    ]
}
