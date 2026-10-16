//! Studio API reference for an application-supplied OpenAPI document.
//!
//! `/studio/api` renders the document's operations on the server inside
//! `studio_layout`, and `/studio/api/openapi.json` serves the document itself.
//! The page loads no script and no remote asset, so it works offline and
//! under a strict Content Security Policy. It is a read-only reference: it
//! does not send requests to the application.

use crate::data_browser::studio_layout;
use axum::{
    Router,
    body::Bytes,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use rullst_core::html::escape_str;
use std::fmt::Write;
use utoipa::openapi::{
    Deprecated, OpenApi, RefOr, Required,
    path::{Operation, Parameter, ParameterIn, PathItem},
};

/// Canonical location of the reference page.
const PAGE_PATH: &str = "/studio/api";
/// Location of the OpenAPI document served as JSON.
const DOCUMENT_PATH: &str = "/studio/api/openapi.json";
/// Operations rendered on the page; the JSON document always carries them all.
const MAX_RENDERED_OPERATIONS: usize = 500;
/// Characters of free text (titles, summaries, descriptions) shown per field.
const MAX_TEXT_CHARS: usize = 512;

/// Mounts the reference page and the JSON document for `openapi`.
///
/// `/studio/api/` (the former Swagger UI location) redirects to the page.
pub fn router(openapi: OpenApi) -> Router {
    let page = Html(studio_layout(render_reference(&openapi), None, &[]));
    let document = serde_json::to_vec(&openapi).ok().map(Bytes::from);
    Router::new()
        .route(
            PAGE_PATH,
            get(move || {
                let page = page.clone();
                async move { page }
            }),
        )
        .route(
            "/studio/api/",
            get(|| async { Redirect::permanent(PAGE_PATH) }),
        )
        .route(
            DOCUMENT_PATH,
            get(move || {
                let document = document.clone();
                async move { document_response(document) }
            }),
        )
}

fn document_response(document: Option<Bytes>) -> Response {
    match document {
        Some(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "The OpenAPI document could not be serialized.",
        )
            .into_response(),
    }
}

/// Renders the page body: document header, then one card per operation.
pub(crate) fn render_reference(openapi: &OpenApi) -> String {
    let mut cards = String::new();
    let mut total = 0_usize;
    for (path, item) in openapi.paths.paths.iter() {
        for (method, operation) in operations_of(item) {
            total += 1;
            if total <= MAX_RENDERED_OPERATIONS {
                render_operation(&mut cards, method, path, item, operation);
            }
        }
    }
    if total == 0 {
        cards.push_str(
            r#"<p class="p-5 rounded-xl border border-slate-800 text-center text-xs text-slate-500">The supplied OpenAPI document declares no operations.</p>"#,
        );
    } else if total > MAX_RENDERED_OPERATIONS {
        let _ = write!(
            cards,
            r#"<p class="p-4 rounded-xl border border-amber-800/50 bg-amber-950/20 text-xs text-amber-300">Showing the first {MAX_RENDERED_OPERATIONS} of {total} operations; the JSON document lists all of them.</p>"#,
        );
    }

    let info = &openapi.info;
    let description = info
        .description
        .as_deref()
        .map(|text| {
            format!(
                r#"<p class="text-sm text-slate-400 mt-2">{}</p>"#,
                escape_str(&bounded(text))
            )
        })
        .unwrap_or_default();
    format!(
        r#"<div class="p-8 space-y-6 max-w-6xl mx-auto"><header class="space-y-2"><div class="flex flex-wrap items-center gap-3"><h1 class="text-3xl font-extrabold text-white">{title}</h1><span class="px-2 py-1 rounded text-xs font-bold bg-sky-500/10 text-sky-400 border border-sky-400/20">v{version}</span><span class="px-2 py-1 rounded text-xs font-bold bg-slate-800 text-slate-300">{total} operations</span></div>{description}<p class="text-xs text-slate-500 font-mono">API reference rendered from the OpenAPI document supplied to Studio::with_openapi. Document: <a class="text-sky-400 hover:text-white" href="{DOCUMENT_PATH}">{DOCUMENT_PATH}</a></p></header><div class="space-y-4">{cards}</div></div>"#,
        title = escape_str(&bounded(&info.title)),
        version = escape_str(&bounded(&info.version)),
    )
}

/// The operations of one path, in HTTP-method order, then any additional ones.
fn operations_of(item: &PathItem) -> impl Iterator<Item = (&str, &Operation)> {
    [
        ("GET", &item.get),
        ("PUT", &item.put),
        ("POST", &item.post),
        ("DELETE", &item.delete),
        ("OPTIONS", &item.options),
        ("HEAD", &item.head),
        ("PATCH", &item.patch),
        ("TRACE", &item.trace),
        ("QUERY", &item.query),
    ]
    .into_iter()
    .filter_map(|(method, operation)| operation.as_ref().map(|operation| (method, operation)))
    .chain(
        item.additional_operations
            .iter()
            .map(|(method, operation)| (method.as_str(), operation)),
    )
}

fn method_badge_class(method: &str) -> &'static str {
    match method {
        "GET" => "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
        "POST" => "bg-sky-500/10 text-sky-400 border-sky-400/20",
        "PUT" | "PATCH" => "bg-amber-500/10 text-amber-400 border-amber-500/20",
        "DELETE" => "bg-rose-500/10 text-rose-400 border-rose-500/20",
        _ => "bg-purple-500/10 text-purple-400 border-purple-500/20",
    }
}

fn render_operation(
    out: &mut String,
    method: &str,
    path: &str,
    item: &PathItem,
    operation: &Operation,
) {
    let _ = write!(
        out,
        r#"<article class="p-5 rounded-xl border border-slate-800 bg-slate-900/80 space-y-3"><div class="flex flex-wrap items-center gap-3"><span class="px-2 py-1 rounded border text-xs font-bold font-mono {badge}">{method}</span><code class="text-sm text-white break-all">{path}</code>"#,
        badge = method_badge_class(method),
        method = escape_str(&bounded(method)),
        path = escape_str(&bounded(path)),
    );
    if operation.deprecated == Some(Deprecated::True) {
        out.push_str(
            r#"<span class="px-2 py-1 rounded text-xs font-bold bg-rose-500/10 text-rose-400">deprecated</span>"#,
        );
    }
    out.push_str("</div>");
    if let Some(summary) = operation.summary.as_deref().or(item.summary.as_deref()) {
        let _ = write!(
            out,
            r#"<p class="text-sm text-slate-300">{}</p>"#,
            escape_str(&bounded(summary))
        );
    }
    if let Some(description) = operation
        .description
        .as_deref()
        .or(item.description.as_deref())
    {
        let _ = write!(
            out,
            r#"<p class="text-xs text-slate-400">{}</p>"#,
            escape_str(&bounded(description))
        );
    }
    render_identity(out, operation);
    render_parameters(out, item, operation);
    render_request_body(out, operation);
    render_responses(out, operation);
    out.push_str("</article>");
}

fn render_identity(out: &mut String, operation: &Operation) {
    let mut facts = Vec::new();
    if let Some(id) = operation.operation_id.as_deref() {
        facts.push(format!(
            "operationId <code>{}</code>",
            escape_str(&bounded(id))
        ));
    }
    if let Some(tags) = operation.tags.as_ref().filter(|tags| !tags.is_empty()) {
        let tags = tags
            .iter()
            .map(|tag| escape_str(&bounded(tag)).into_owned())
            .collect::<Vec<_>>()
            .join(", ");
        facts.push(format!("tags {tags}"));
    }
    if !facts.is_empty() {
        let _ = write!(
            out,
            r#"<p class="text-xs text-slate-500 font-mono">{}</p>"#,
            facts.join(" · ")
        );
    }
}

/// Path-level parameters apply to every operation of the path; an operation
/// parameter with the same name and location overrides one of them.
fn render_parameters(out: &mut String, item: &PathItem, operation: &Operation) {
    let operation_parameters = operation.parameters.as_deref().unwrap_or_default();
    let overridden = |shared: &Parameter| {
        operation_parameters
            .iter()
            .any(|candidate| match candidate {
                RefOr::T(own) => own.name == shared.name && own.parameter_in == shared.parameter_in,
                RefOr::Ref(_) => false,
            })
    };
    let shared = item
        .parameters
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|parameter| match parameter {
            RefOr::T(parameter) => !overridden(parameter),
            RefOr::Ref(_) => true,
        });
    let mut rows = String::new();
    for parameter in shared.chain(operation_parameters) {
        match parameter {
            RefOr::T(parameter) => {
                let _ = write!(
                    rows,
                    r#"<tr><td class="px-4 py-3 font-mono text-sky-300">{name}</td><td class="px-4 py-3 text-slate-400">{location}</td><td class="px-4 py-3 text-slate-400">{required}</td></tr>"#,
                    name = escape_str(&bounded(&parameter.name)),
                    location = parameter_location(&parameter.parameter_in),
                    required = if parameter.required == Required::True {
                        "required"
                    } else {
                        "optional"
                    },
                );
            }
            RefOr::Ref(reference) => {
                let _ = write!(
                    rows,
                    r#"<tr><td class="px-4 py-3 font-mono text-sky-300" colspan="3">{}</td></tr>"#,
                    escape_str(&bounded(&reference.ref_location))
                );
            }
        }
    }
    if !rows.is_empty() {
        let _ = write!(
            out,
            r#"<div class="overflow-x-auto rounded-lg border border-slate-800"><table class="w-full text-left text-xs"><thead class="bg-slate-950 uppercase text-slate-400"><tr><th class="px-4 py-3">Parameter</th><th class="px-4 py-3">In</th><th class="px-4 py-3">Presence</th></tr></thead><tbody class="divide-y divide-slate-800">{rows}</tbody></table></div>"#,
        );
    }
}

fn parameter_location(location: &ParameterIn) -> &'static str {
    match location {
        ParameterIn::Query => "query",
        ParameterIn::Path => "path",
        ParameterIn::Header => "header",
        ParameterIn::Cookie => "cookie",
        ParameterIn::QueryString => "querystring",
    }
}

fn render_request_body(out: &mut String, operation: &Operation) {
    let Some(body) = operation.request_body.as_ref() else {
        return;
    };
    let text = match body {
        RefOr::T(body) => {
            let presence = if body.required == Some(Required::True) {
                "required"
            } else {
                "optional"
            };
            let media_types = body
                .content
                .keys()
                .map(|media_type| escape_str(&bounded(media_type)).into_owned())
                .collect::<Vec<_>>()
                .join(", ");
            format!("Request body ({presence}) {media_types}")
        }
        RefOr::Ref(reference) => format!(
            "Request body {}",
            escape_str(&bounded(&reference.ref_location))
        ),
    };
    let _ = write!(
        out,
        r#"<p class="text-xs text-slate-400 font-mono">{text}</p>"#
    );
}

fn render_responses(out: &mut String, operation: &Operation) {
    let mut items = String::new();
    for (status, response) in &operation.responses.responses {
        let description = match response {
            RefOr::T(response) => escape_str(&bounded(&response.description)).into_owned(),
            RefOr::Ref(reference) => escape_str(&bounded(&reference.ref_location)).into_owned(),
        };
        let _ = write!(
            items,
            r#"<li><code class="text-emerald-300">{}</code> <span class="text-slate-400">{description}</span></li>"#,
            escape_str(&bounded(status))
        );
    }
    if !items.is_empty() {
        let _ = write!(
            out,
            r#"<ul class="space-y-2 text-xs font-mono">{items}</ul>"#
        );
    }
}

/// Keeps at most [`MAX_TEXT_CHARS`] characters of developer-supplied text.
fn bounded(text: &str) -> std::borrow::Cow<'_, str> {
    match text.char_indices().nth(MAX_TEXT_CHARS) {
        Some((cut, _)) => std::borrow::Cow::Owned(format!("{}…", &text[..cut])),
        None => std::borrow::Cow::Borrowed(text),
    }
}

#[cfg(test)]
#[path = "api_playground_tests.rs"]
mod tests;
