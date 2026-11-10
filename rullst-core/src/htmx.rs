use axum::{
    extract::{FromRequestParts, Request},
    http::{HeaderMap, HeaderValue, header, request::Parts},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};

use crate as rullst;
use crate::html::Markup;

/// Extract's HTMX request headers to determine context and re-act re-actively.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct HtmxRequest {
    /// True if the request was triggered by HTMX in the browser (`HX-Request: true`).
    pub is_htmx: bool,
    /// The ID of the triggered element if sent by HTMX (`HX-Trigger`).
    pub trigger: Option<String>,
    /// The ID of the target element if sent by HTMX (`HX-Target`).
    pub target: Option<String>,
    /// The user response inputted into the prompt if sent by HTMX (`HX-Prompt`).
    ///
    /// Like `trigger`, `target` and `current_url`, it is decoded as htmx sent
    /// it: Latin-1 bytes, or percent-encoded UTF-8 when htmx adds
    /// `HX-Prompt-URI-AutoEncoded: true`, so answers such as `José` or `日本`
    /// arrive intact.
    pub prompt: Option<String>,
    /// The browser's active URL when the request was initiated (`HX-Current-URL`).
    pub current_url: Option<String>,
    csp_nonce: Option<crate::security::CspNonce>,
    /// `HX-History-Restore-Request: true`: htmx missed its history cache and
    /// replaces the whole `<body>` with the response.
    history_restore: bool,
    /// `HX-Boosted: true`: a boosted link or form, which expects a page.
    boosted: bool,
}

impl HtmxRequest {
    /// Whether [`render_page`] answers with the content alone: an htmx
    /// request that is neither a history restore nor boosted.
    fn wants_fragment(&self) -> bool {
        self.is_htmx && !self.history_restore && !self.boosted
    }
}

impl<S> FromRequestParts<S> for HtmxRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let is_htmx = header_is_true(&parts.headers, "HX-Request");
        let history_restore = header_is_true(&parts.headers, "HX-History-Restore-Request");
        let boosted = header_is_true(&parts.headers, "HX-Boosted");

        let trigger = htmx_header(&parts.headers, "HX-Trigger");
        let target = htmx_header(&parts.headers, "HX-Target");
        let prompt = htmx_header(&parts.headers, "HX-Prompt");
        let current_url = htmx_header(&parts.headers, "HX-Current-URL");

        let csp_nonce = parts.extensions.get::<crate::security::CspNonce>().cloned();

        Ok(HtmxRequest {
            is_htmx,
            trigger,
            target,
            prompt,
            current_url,
            csp_nonce,
            history_restore,
            boosted,
        })
    }
}

/// Whether htmx sent the boolean header `name` as `true`.
fn header_is_true(headers: &HeaderMap, name: &str) -> bool {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "true")
}

/// Reads a header as htmx's browser client sent it.
///
/// `XMLHttpRequest` sends a value made of code points up to U+00FF as one
/// byte per character (Latin-1), so `José` arrives as non-ASCII bytes that
/// `HeaderValue::to_str` rejects. Any other value makes htmx resend it
/// `encodeURIComponent`-encoded and add `<name>-URI-AutoEncoded: true`, in
/// which case it is percent-decoded as UTF-8. A value that does not decode
/// is `None`.
fn htmx_header(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let value = headers.get(name)?;
    let encoded = headers
        .get(format!("{name}-URI-AutoEncoded"))
        .is_some_and(|flag| flag.as_bytes().eq_ignore_ascii_case(b"true"));
    if encoded {
        percent_decode_utf8(value.as_bytes())
    } else {
        Some(
            value
                .as_bytes()
                .iter()
                .map(|&byte| char::from(byte))
                .collect(),
        )
    }
}

/// Decodes `%XX` escapes, requiring the result to be UTF-8.
fn percent_decode_utf8(bytes: &[u8]) -> Option<String> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let hex = bytes
                .get(index + 1..index + 3)
                .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))?;
            decoded.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// A highly ergonomic, builder-style HTMX responder to set dynamic headers in client side.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct HtmxResponse {
    /// The inner HTML content to be sent in the response body.
    pub content: String,
    /// Event name to trigger a custom client-side event (`HX-Trigger`).
    pub trigger: Option<String>,
    /// Target path to redirect the client side to a new page (`HX-Redirect`).
    pub redirect: Option<String>,
    /// Set to true to trigger a full page refresh on the client (`HX-Refresh: true`).
    pub refresh: bool,
}

impl HtmxResponse {
    /// Creates a new base HTMX response with raw HTML content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            trigger: None,
            redirect: None,
            refresh: false,
        }
    }

    /// Triggers a custom event in the browser on response completion.
    pub fn trigger(mut self, event: impl Into<String>) -> Self {
        self.trigger = Some(event.into());
        self
    }

    /// Triggers a client-side browser redirect to a new path.
    pub fn redirect(mut self, url: impl Into<String>) -> Self {
        self.redirect = Some(url.into());
        self
    }

    /// Triggers a full browser refresh when the client processes the response.
    pub fn refresh(mut self) -> Self {
        self.refresh = true;
        self
    }
}

impl IntoResponse for HtmxResponse {
    fn into_response(self) -> Response {
        let mut res = Html(self.content).into_response();
        let headers = res.headers_mut();

        if let Some(ref trigger) = self.trigger
            && let Ok(val) = HeaderValue::from_str(trigger)
        {
            headers.insert("HX-Trigger", val);
        }

        if let Some(ref redirect) = self.redirect
            && let Ok(val) = HeaderValue::from_str(redirect)
        {
            headers.insert("HX-Redirect", val);
        }

        if self.refresh {
            headers.insert("HX-Refresh", HeaderValue::from_static("true"));
        }

        res
    }
}

/// The `htmx-config` that stops htmx 1.9.12 from inserting its indicator
/// rules as an inline `<style>`, which has no CSP nonce.
const HTMX_INDICATOR_CONFIG: &str = r#"{"includeIndicatorStyles":false}"#;

/// The rules htmx 1.9.12 inserts with `includeIndicatorStyles` (its default
/// `htmx-indicator` and `htmx-request` classes), sent with the request's nonce.
const HTMX_INDICATOR_STYLES: &str = ".htmx-indicator{opacity:0}\
.htmx-request .htmx-indicator{opacity:1;transition:opacity 200ms ease-in}\
.htmx-request.htmx-indicator{opacity:1;transition:opacity 200ms ease-in}";

/// The version-pinned htmx client below `static/` that the page loads.
const HTMX_SCRIPT: &str = "htmx-1.9.12.min.js";

/// Helper function to render a hybrid SSR layout page.
/// - An htmx request (`HX-Request: true`) receives just the inner `content`
///   as a fragment.
/// - Every other request receives `content` wrapped in an HTML5 skeleton that
///   loads the generated same-origin stylesheet and the version-pinned HTMX
///   browser client. CLI scaffolds supply `/static/htmx-1.9.12.min.js`;
///   manually built applications must serve that asset themselves. Since
///   13.0.0-alpha.5 both are linked with [`crate::assets::url`]: while the
///   server serves `static/`, `/static/rullst.<hash>.css` and
///   `/static/htmx-1.9.12.min.<hash>.js`, cached as immutable, so a deploy
///   that changes `static/rullst.css` reaches every browser and CDN at once;
///   outside a server (unit tests), the plain URLs. Since
///   13.0.0-rc.1 that includes the two htmx requests that expect a page: a
///   history restore after a cache miss (`HX-History-Restore-Request: true`,
///   which replaces the whole `<body>`) and a boosted link or form
///   (`HX-Boosted: true`); before, both received the fragment and the page
///   lost its layout.
///
/// The page and the fragment share a URL, so under [`crate::Server`] HTML
/// responses of the application's routes carry `Vary: HX-Request`, and a
/// cache keeps the two apart.
///
/// Since 13.0.0-rc.1, under Rullst's security headers (a request CSP nonce),
/// the page also sends `<meta name="htmx-config"
/// content='{"includeIndicatorStyles":false}'>` and htmx's `.htmx-indicator`
/// rules in a `<style>` carrying the nonce: htmx's own inline indicator
/// `<style>` has none, and the strict `style-src` would block it. `hx-indicator` keeps working. Head markup passed to
/// [`render_page_with_head`] that contains its own `htmx-config` is left in
/// charge of htmx's configuration. Without a nonce the page is unchanged.
///
/// `content` is [`Markup`], usually an `html!` result, which already escapes
/// interpolated values. HTML from elsewhere must be passed explicitly as
/// trusted with [`Markup::from_trusted`] or [`RawHtml`](crate::html::RawHtml).
///
/// The page declares `lang="pt-BR"`, kept for compatibility; use
/// [`render_page_with_lang`] to declare the language the content is written
/// in.
pub fn render_page(htmx: &HtmxRequest, title: &str, content: impl Into<Markup>) -> Html<String> {
    render_page_with_lang(htmx, "pt-BR", title, content)
}

/// [`render_page`] with the document language (a BCP 47 tag such as `en` or
/// `pt-BR`) declared on `<html lang>`, which screen readers, translation and
/// spell-checking use. The tag is HTML-escaped.
///
/// Added in v13.
pub fn render_page_with_lang(
    htmx: &HtmxRequest,
    lang: &str,
    title: &str,
    content: impl Into<Markup>,
) -> Html<String> {
    render_page_with_head(htmx, lang, title, Markup::default(), content)
}

/// [`render_page_with_lang`] with extra `head` markup placed at the end of
/// the document's `<head>`, after the stylesheet and the htmx script: for
/// example the web app manifest link, `theme-color` and the service worker
/// registration script that `cargo rullst make:pwa` adds. An htmx request
/// still receives only `content`, as with [`render_page`].
///
/// `head` is [`Markup`] like `content`, so `html!` output is escaped. A
/// `<script>` placed there must load a same-origin file (`src="/static/…"`)
/// or carry the request's CSP nonce.
///
/// Added in v13.0.0-alpha.3.
pub fn render_page_with_head(
    htmx: &HtmxRequest,
    lang: &str,
    title: &str,
    head: impl Into<Markup>,
    content: impl Into<Markup>,
) -> Html<String> {
    let content = content.into();
    if htmx.wants_fragment() {
        return Html(content.into_string());
    }
    let head = head.into();
    let htmx_source = crate::assets::url(HTMX_SCRIPT);
    let htmx_script = if let Some(nonce) = htmx.csp_nonce.as_ref() {
        // htmx's own indicator `<style>` would carry no nonce: turn it off
        // (unless `head` brings its own htmx-config) and send the same rules
        // in a `<style>` with the request's nonce.
        let config = (!head.as_str().contains("htmx-config")).then(|| {
            crate::html! { <meta name="htmx-config" content={HTMX_INDICATOR_CONFIG} /> }
        });
        crate::html! {
            <script nonce={nonce.as_str()} src={htmx_source.as_str()}></script>
            {config}
            <style nonce={nonce.as_str()}>{HTMX_INDICATOR_STYLES}</style>
        }
    } else {
        crate::html! {
            <script src={htmx_source.as_str()}></script>
        }
    };
    let page = crate::html! {
        <!DOCTYPE html>
        <html lang={lang} class="h-full bg-slate-950 text-slate-100">
            <head>
                <meta charset="utf-8" />
                <title>{title}</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <link rel="icon" type="image/png" href="/static/rullst.png" />
                <link rel="stylesheet" href={crate::assets::url("rullst.css")} />
                {htmx_script}
                {head}
            </head>
            <body class="h-full">
                {content}
            </body>
        </html>
    };
    Html(page.into_string())
}

/// Adds `Vary: HX-Request` to the HTML responses of the application's
/// routes, which [`crate::Server`] wraps in this middleware. [`render_page`]
/// and handlers that read [`HtmxRequest`] answer one URL with a page or a
/// fragment depending on that header, so a cache that stored one must not
/// serve it for the other. A `Vary` that already names the header, or is
/// `*`, is kept as it is.
pub(crate) async fn vary_on_htmx_request(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    vary_html_on_htmx_request(response.headers_mut());
    response
}

fn vary_html_on_htmx_request(headers: &mut HeaderMap) {
    let is_html = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("text/html"));
    let varies = headers.get_all(header::VARY).iter().any(|value| {
        value.to_str().is_ok_and(|names| {
            names
                .split(',')
                .map(str::trim)
                .any(|name| name == "*" || name.eq_ignore_ascii_case("HX-Request"))
        })
    });
    if is_html && !varies {
        headers.append(header::VARY, HeaderValue::from_static("HX-Request"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "htmx_tests.rs"]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "htmx_page_tests.rs"]
mod page_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "htmx_assets_tests.rs"]
mod assets_tests;
