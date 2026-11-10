//! The tags `add tailwind` and `add alpine` put in a layout keep a page
//! inside Rullst's default Content-Security-Policy: rendered through the real
//! security baseline, the page needs no inline script or style without the
//! request nonce and nothing that would require `'unsafe-eval'`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{alpine, tailwind};
use rullst_core::config::{Environment, SecurityConfig};
use rullst_core::html::Markup;
use rullst_core::htmx::{HtmxRequest, render_page_with_head};
use tower::ServiceExt;

/// Each generated `html!` line and what it renders to while no server has
/// mounted `static/` (`rullst::assets::url` returns the plain URL).
const RENDERED: [(&str, &str); 3] = [
    (
        tailwind::LINK_LINE,
        r#"<link rel="stylesheet" href="/static/app.css" />"#,
    ),
    (
        alpine::SCRIPT_LINES[0],
        r#"<script defer src="/static/js/alpine-components.js"></script>"#,
    ),
    (
        alpine::SCRIPT_LINES[1],
        r#"<script defer src="/static/vendor/alpine-csp.min.js"></script>"#,
    ),
];

/// [`alpine::EXAMPLE`] as `html!` renders it.
const EXAMPLE_RENDERED: &str = r#"<div x-data="charCounter(160)"><label for="message">Message</label><textarea id="message" name="message" x-model="text"></textarea><p aria-live="polite" x-text="remaining + ' characters left'"></p><button type="button" x-bind="clear">Clear</button></div>"#;

/// The path passed to `rullst::assets::url` in a generated line.
fn asset_path(line: &str) -> &str {
    let start = line.find("rullst::assets::url(\"").unwrap() + "rullst::assets::url(\"".len();
    let end = start + line[start..].find('"').unwrap();
    &line[start..end]
}

#[test]
fn the_rendered_forms_are_those_of_the_generated_lines() {
    for (line, rendered) in RENDERED {
        let path = asset_path(line);
        assert!(
            rendered.contains(&format!("=\"{}\"", rullst_core::assets::url(path))),
            "{line}"
        );
        let tag = &line[..line.find(' ').unwrap()];
        assert!(rendered.starts_with(tag), "{line}");
        assert_eq!(
            line.contains("defer"),
            rendered.contains(" defer "),
            "{line}"
        );
    }
    // The example's tags and attributes, in order.
    let compact: String = alpine::EXAMPLE
        .lines()
        .map(str::trim)
        .collect::<String>()
        .replace("\"Message\"", "Message")
        .replace("\"Clear\"", "Clear");
    assert_eq!(compact, EXAMPLE_RENDERED);
}

async fn page(htmx: HtmxRequest) -> axum::response::Html<String> {
    let head: String = RENDERED.iter().map(|(_, rendered)| *rendered).collect();
    render_page_with_head(
        &htmx,
        "en",
        "Frontend kits",
        Markup::from_trusted(head),
        Markup::from_trusted(EXAMPLE_RENDERED),
    )
}

/// Every `<tag …>` opening tag of `html` named `name`.
fn tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    html.match_indices(&format!("<{name}"))
        .map(|(start, _)| &html[start..start + html[start..].find('>').unwrap() + 1])
        .collect()
}

#[tokio::test]
async fn generated_tags_need_no_inline_code_or_eval_under_the_default_policy() {
    let app = axum::Router::new().route("/", axum::routing::get(page));
    let app = rullst_core::security::apply_security_baseline(
        app,
        SecurityConfig::default(),
        Environment::Production,
    )
    .unwrap();
    let response = app
        .oneshot(
            axum::http::Request::get("/")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let policy = response.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .to_owned();
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    for directive in [
        "script-src 'self' 'nonce-",
        "style-src 'self' 'nonce-",
        "default-src 'self'",
    ] {
        assert!(policy.contains(directive), "{directive}: {policy}");
    }
    for forbidden in [
        "unsafe-eval",
        "unsafe-inline",
        "unsafe-hashes",
        "strict-dynamic",
        "*",
    ] {
        assert!(!policy.contains(forbidden), "{forbidden}: {policy}");
    }
    let nonce = policy
        .split("'nonce-")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap();

    for (_, rendered) in RENDERED {
        assert!(html.contains(rendered), "{rendered}\n{html}");
    }
    // Scripts are same-origin files (`script-src 'self'`) or carry the nonce.
    let scripts = tags(&html, "script");
    assert_eq!(scripts.len(), 3, "{html}");
    for script in &scripts {
        let same_origin = script.contains(" src=\"/static/");
        let nonced = script.contains(&format!(" nonce=\"{nonce}\""));
        assert!(same_origin, "{script}");
        if !script.contains("htmx") {
            assert!(!nonced, "generated scripts need no nonce: {script}");
            assert!(script.contains(" defer "), "{script}");
        }
    }
    for style in tags(&html, "style") {
        assert!(style.contains(&format!(" nonce=\"{nonce}\"")), "{style}");
    }
    for link in tags(&html, "link rel=\"stylesheet\"") {
        assert!(link.contains(" href=\"/static/"), "{link}");
    }
    // No inline style attribute, event handler attribute or script URL.
    assert!(!html.contains(" style=\""), "{html}");
    assert!(!html.contains("javascript:"), "{html}");
    for attribute in html
        .split(' ')
        .filter_map(|word| word.split_once('=').map(|(name, _)| name))
    {
        assert!(
            !(attribute.starts_with("on") && attribute.len() > 2),
            "{attribute} in {html}"
        );
    }
    assert!(!html.contains("x-html"), "{html}");
}

/// Constructs the Alpine CSP parser rejects (https://alpinejs.dev/advanced/csp,
/// and `src/parser.js` of `@alpinejs/csp` 3.17.5).
const UNSUPPORTED_IN_DIRECTIVES: [&str; 13] = [
    "=>", "`", "${", "...", ";", "?.", "??", "+=", "window", "document", "console", "Math", "JSON",
];

#[test]
fn the_example_and_component_stay_within_the_csp_builds_rules() {
    for attribute in EXAMPLE_RENDERED.split(" x-").skip(1) {
        let value = attribute
            .split_once("=\"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(value, _)| value)
            .unwrap();
        for construct in UNSUPPORTED_IN_DIRECTIVES {
            assert!(!value.contains(construct), "x-{attribute}: {construct}");
        }
    }
    // The component file is plain same-origin JavaScript: no string
    // evaluation and no HTML injection.
    let code: String = alpine::COMPONENTS_JS
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "eval(",
        "Function(",
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
        "setTimeout(\"",
        "setInterval(\"",
        "x-html",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    assert!(code.contains("document.addEventListener(\"alpine:init\""));
    assert!(code.contains("Alpine.data(\"charCounter\""));
    // Components load first: their `alpine:init` listener must exist when
    // the CSP build starts Alpine (a microtask after it runs).
    assert!(alpine::SCRIPT_LINES[0].contains("js/alpine-components.js"));
    assert!(alpine::SCRIPT_LINES[1].contains("vendor/alpine-csp.min.js"));
    assert_eq!(
        crate::generators::frontend::alpine_release::ENTRY,
        "package/dist/cdn.min.js"
    );
}
