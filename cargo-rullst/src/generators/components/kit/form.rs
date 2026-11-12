//! Form and form fields, copied by `cargo rullst ui add form`; the
//! application owns this file.
//!
//! ```text
//! use crate::ui::form::{self, Control, Field, FormOptions, InputType};
//!
//! async fn page(Extension(csrf): Extension<rullst::security::CsrfToken>) -> Markup {
//!     form::form("/contacts", csrf.as_str(), &FormOptions::default(), html! {
//!         {form::field(&Field {
//!             id: "email",
//!             label: "Email",
//!             control: Control::Input(InputType::Email),
//!             autocomplete: Some("email"),
//!             required: true,
//!             error: errors.get("email").map(String::as_str),
//!             ..Default::default()
//!         })}
//!         <button type="submit">"Save"</button>
//!     })
//! }
//! ```
//!
//! Accessibility: every control has a `<label for>`. The help text and the
//! error are linked with `aria-describedby`; an error also sets
//! `aria-invalid="true"`, so a screen reader announces it with the field.
//! Required fields carry `required` (announced) and a visible `*`.
//!
//! CSRF: [`form`] writes the `_token` field that Rullst's double-submit check
//! reads, first in the form (a `multipart/form-data` body must send it before
//! any file). Pass the request's `rullst::security::CsrfToken`, from
//! `Extension<CsrfToken>`; `GET` forms get no token.

use rullst::html;
use rullst::html::Markup;

/// The HTTP method of a [`form`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Post,
    Get,
}

/// Options of [`form`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormOptions<'a> {
    pub method: Method,
    /// `hx-boost="true"`: htmx submits the form and swaps the page's body,
    /// keeping a plain form submission when JavaScript is off.
    pub boost: bool,
    /// `enctype="multipart/form-data"`, for file fields.
    pub multipart: bool,
    pub id: Option<&'a str>,
}

/// A `<form>` holding `body`, with the CSRF `_token` field for `POST`.
pub fn form(action: &str, csrf_token: &str, options: &FormOptions<'_>, body: Markup) -> Markup {
    let (method, token) = match options.method {
        Method::Post => ("post", Some(csrf_token)),
        Method::Get => ("get", None),
    };
    let enctype = options.multipart.then_some("multipart/form-data");
    let boost = options.boost.then_some("true");
    html! {
        <form method={method} action={action} id?={options.id} enctype?={enctype} hx-boost?={boost} class="space-y-6">
            @if let Some(token) = token {
                <input type="hidden" name="_token" value={token} />
            }
            {body}
        </form>
    }
}

/// The `type` of an `<input>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputType {
    #[default]
    Text,
    Email,
    Password,
    Number,
    Tel,
    Url,
    Search,
    Date,
}

impl InputType {
    fn as_str(self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Email => "email",
            InputType::Password => "password",
            InputType::Number => "number",
            InputType::Tel => "tel",
            InputType::Url => "url",
            InputType::Search => "search",
            InputType::Date => "date",
        }
    }
}

/// The control of a [`Field`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control<'a> {
    Input(InputType),
    Textarea {
        rows: u8,
    },
    /// `(value, label)` pairs; the one equal to [`Field::value`] is selected.
    Select(&'a [(&'a str, &'a str)]),
}

impl Default for Control<'_> {
    fn default() -> Self {
        Control::Input(InputType::Text)
    }
}

/// A labelled control with optional help text and error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Field<'a> {
    /// The control's `id`; the help and error are `<id>-help` and `<id>-error`.
    pub id: &'a str,
    /// The submitted name; the `id` when empty.
    pub name: &'a str,
    pub label: &'a str,
    pub control: Control<'a>,
    pub value: Option<&'a str>,
    pub placeholder: Option<&'a str>,
    pub autocomplete: Option<&'a str>,
    pub help: Option<&'a str>,
    /// The validation message to show; `None` when the value is valid.
    pub error: Option<&'a str>,
    pub required: bool,
    pub disabled: bool,
}

/// Classes of text inputs, text areas and selects.
fn control_classes(invalid: bool) -> &'static str {
    if invalid {
        "block w-full rounded-md border border-red-600 bg-white px-3 py-2 text-sm text-slate-900 shadow-sm placeholder:text-slate-500 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-red-600 disabled:opacity-60 dark:border-red-400 dark:bg-slate-900 dark:text-slate-100 dark:placeholder:text-slate-400 dark:focus-visible:outline-red-400"
    } else {
        "block w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 shadow-sm placeholder:text-slate-500 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-sky-600 disabled:opacity-60 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100 dark:placeholder:text-slate-400 dark:focus-visible:outline-sky-400"
    }
}

const LABEL: &str = "block text-sm font-medium text-slate-900 dark:text-slate-100";
const HELP: &str = "text-sm text-slate-600 dark:text-slate-400";
const ERROR: &str = "text-sm font-medium text-red-700 dark:text-red-400";

/// The ids `aria-describedby` lists: the help, then the error.
fn described_by(field: &Field<'_>) -> Option<String> {
    let ids: Vec<String> = [
        field.help.map(|_| format!("{}-help", field.id)),
        field.error.map(|_| format!("{}-error", field.id)),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!ids.is_empty()).then(|| ids.join(" "))
}

/// The help text and error below a control.
fn messages(field: &Field<'_>) -> Markup {
    html! {
        @if let Some(help) = field.help {
            <p id={format!("{}-help", field.id)} class={HELP}>{help}</p>
        }
        @if let Some(error) = field.error {
            <p id={format!("{}-error", field.id)} class={ERROR}>{error}</p>
        }
    }
}

/// A label, a control, its help text and its error.
pub fn field(field: &Field<'_>) -> Markup {
    let name = if field.name.is_empty() {
        field.id
    } else {
        field.name
    };
    let described = described_by(field);
    let invalid = field.error.map(|_| "true");
    let classes = control_classes(field.error.is_some());
    let control = match field.control {
        Control::Input(kind) => html! {
            <input type={kind.as_str()} id={field.id} name={name} value?={field.value} placeholder?={field.placeholder} autocomplete?={field.autocomplete} required?={field.required} disabled?={field.disabled} aria-describedby?={described.as_deref()} aria-invalid?={invalid} class={classes} />
        },
        Control::Textarea { rows } => html! {
            <textarea id={field.id} name={name} rows={rows} placeholder?={field.placeholder} required?={field.required} disabled?={field.disabled} aria-describedby?={described.as_deref()} aria-invalid?={invalid} class={classes}>{field.value.unwrap_or_default()}</textarea>
        },
        Control::Select(options) => html! {
            <select id={field.id} name={name} required?={field.required} disabled?={field.disabled} aria-describedby?={described.as_deref()} aria-invalid?={invalid} class={classes}>
                @for (value, label) in options {
                    <option value={*value} selected?={field.value == Some(*value)}>{*label}</option>
                }
            </select>
        },
    };
    html! {
        <div class="space-y-2">
            <label for={field.id} class={LABEL}>
                {field.label}
                @if field.required {
                    <span class="text-red-700 dark:text-red-400" aria-hidden="true">" *"</span>
                }
            </label>
            {control}
            {messages(field)}
        </div>
    }
}

/// A checkbox with its label on the right; [`Field::value`] is the submitted
/// value (`on` when `None`) and `checked` its state. The control is ignored.
pub fn checkbox(field: &Field<'_>, checked: bool) -> Markup {
    let name = if field.name.is_empty() {
        field.id
    } else {
        field.name
    };
    let described = described_by(field);
    let invalid = field.error.map(|_| "true");
    html! {
        <div class="space-y-2">
            <div class="flex items-center gap-3">
                <input type="checkbox" id={field.id} name={name} value?={field.value} checked?={checked} required?={field.required} disabled?={field.disabled} aria-describedby?={described.as_deref()} aria-invalid?={invalid} class="size-4 shrink-0 cursor-pointer rounded border-slate-300 accent-sky-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 dark:border-slate-600 dark:accent-sky-400 dark:focus-visible:outline-sky-400" />
                <label for={field.id} class={LABEL}>{field.label}</label>
            </div>
            {messages(field)}
        </div>
    }
}
