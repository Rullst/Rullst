use axum::{
    extract::FromRequestParts,
    http::{HeaderValue, request::Parts},
    response::{Html, IntoResponse, Response},
};

use crate as rullst;

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
}

impl<S> FromRequestParts<S> for HtmxRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let is_htmx = parts
            .headers
            .get("HX-Request")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);

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
        })
    }
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

/// Helper function to render a hybrid SSR layout page.
/// - If it is triggered by HTMX, it returns just the inner `content` as a fragment.
/// - Otherwise, it wraps `content` in an HTML5 skeleton that loads the generated
///   same-origin stylesheet and the version-pinned HTMX browser client.
///   CLI scaffolds supply `/static/htmx-1.9.12.min.js`; manually built
///   applications must serve that asset themselves. `content` is trusted HTML:
///   escape untrusted values before constructing this fragment.
///
/// The page declares `lang="pt-BR"`, kept for compatibility; use
/// [`render_page_with_lang`] to declare the language the content is written
/// in.
pub fn render_page(htmx: &HtmxRequest, title: &str, content: String) -> Html<String> {
    render_page_with_lang(htmx, "pt-BR", title, content)
}

/// [`render_page`] with the document language (a BCP 47 tag such as `en` or
/// `pt-BR`) declared on `<html lang>`, which screen readers, translation and
/// spell-checking use. The tag is HTML-escaped.
///
/// Unpublished v13 API.
pub fn render_page_with_lang(
    htmx: &HtmxRequest,
    lang: &str,
    title: &str,
    content: String,
) -> Html<String> {
    if htmx.is_htmx {
        Html(content)
    } else {
        let htmx_script = if let Some(nonce) = htmx.csp_nonce.as_ref() {
            crate::html! {
                <script nonce={nonce.as_str()} src="/static/htmx-1.9.12.min.js"></script>
            }
        } else {
            crate::html! {
                <script src="/static/htmx-1.9.12.min.js"></script>
            }
        };
        let html_content = crate::html! {
            <html lang={lang} class="h-full bg-slate-950 text-slate-100">
                <head>
                    <meta charset="utf-8" />
                    <title>{title}</title>
                    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                    <link rel="icon" type="image/png" href="/static/rullst.png" />
                    <link rel="stylesheet" href="/static/rullst.css" />
                    { crate::html::RawHtml(htmx_script) }
                </head>
                <body class="h-full">
                    { crate::html::RawHtml(content) }
                </body>
            </html>
        };
        Html(format!("<!DOCTYPE html>{}", html_content))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[tokio::test]
    async fn test_htmx_request_extractor_empty() {
        let req = Request::builder().body(()).unwrap();
        let (mut parts, _) = req.into_parts();
        let htmx_req = HtmxRequest::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert!(!htmx_req.is_htmx);
        assert!(htmx_req.trigger.is_none());
        assert!(htmx_req.target.is_none());
        assert!(htmx_req.prompt.is_none());
        assert!(htmx_req.current_url.is_none());
    }

    #[tokio::test]
    async fn test_htmx_request_extractor_headers() {
        let nonce = crate::security::CspNonce::generate();
        let mut req = Request::builder()
            .header("HX-Request", "true")
            .header("HX-Trigger", "my-btn")
            .header("HX-Target", "content-div")
            .header("HX-Prompt", "hello")
            .header("HX-Current-URL", "http://localhost/home")
            .body(())
            .unwrap();
        req.extensions_mut().insert(nonce.clone());
        let (mut parts, _) = req.into_parts();
        let htmx_req = HtmxRequest::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert!(htmx_req.is_htmx);
        assert_eq!(htmx_req.trigger.as_deref(), Some("my-btn"));
        assert_eq!(htmx_req.target.as_deref(), Some("content-div"));
        assert_eq!(htmx_req.prompt.as_deref(), Some("hello"));
        assert_eq!(
            htmx_req.current_url.as_deref(),
            Some("http://localhost/home")
        );
        assert_eq!(htmx_req.csp_nonce.as_ref(), Some(&nonce));
    }

    #[test]
    fn test_htmx_response_builder() {
        let res = HtmxResponse::new("Hello world")
            .trigger("custom-event")
            .redirect("/new-path")
            .refresh();

        assert_eq!(res.content, "Hello world");
        assert_eq!(res.trigger.as_deref(), Some("custom-event"));
        assert_eq!(res.redirect.as_deref(), Some("/new-path"));
        assert!(res.refresh);
    }

    #[tokio::test]
    async fn test_htmx_response_into_response() {
        use axum::response::IntoResponse;
        let res = HtmxResponse::new("Hello world")
            .trigger("my-trigger")
            .redirect("/some-redirect")
            .refresh()
            .into_response();

        let headers = res.headers();
        assert_eq!(headers.get("HX-Trigger").unwrap(), "my-trigger");
        assert_eq!(headers.get("HX-Redirect").unwrap(), "/some-redirect");
        assert_eq!(headers.get("HX-Refresh").unwrap(), "true");
    }

    #[test]
    fn test_htmx_response_refresh_builder() {
        let res = HtmxResponse::new("Hello").refresh();
        assert!(res.refresh);
    }

    #[tokio::test]
    async fn test_htmx_response_refresh_into_response() {
        use axum::response::IntoResponse;
        let res = HtmxResponse::new("Hello world").refresh().into_response();
        let headers = res.headers();
        assert_eq!(headers.get("HX-Refresh").unwrap(), "true");
        assert!(headers.get("HX-Trigger").is_none());
        assert!(headers.get("HX-Redirect").is_none());
    }

    #[test]
    fn test_render_page_helper() {
        let req_htmx = HtmxRequest {
            is_htmx: true,
            trigger: None,
            target: None,
            prompt: None,
            current_url: None,
            csp_nonce: None,
        };
        let req_normal = HtmxRequest {
            is_htmx: false,
            trigger: None,
            target: None,
            prompt: None,
            current_url: None,
            csp_nonce: None,
        };

        // HTMX request -> only the inner content fragment
        let res_htmx = render_page(&req_htmx, "Title", "<div>Fragment</div>".to_string());
        assert_eq!(res_htmx.0, "<div>Fragment</div>");

        // Normal request -> wraps in HTML template
        let res_normal = render_page(
            &req_normal,
            "My Page Title",
            "<div>Body Content</div>".to_string(),
        );
        assert!(res_normal.0.contains("<!DOCTYPE html>"));
        assert!(res_normal.0.contains("<title>My Page Title</title>"));
        assert!(res_normal.0.contains("<div>Body Content</div>"));
        assert!(res_normal.0.contains("/static/rullst.css"));
        assert!(res_normal.0.contains("/static/rullst.png"));
        assert!(!res_normal.0.contains("https://cdn.tailwindcss.com"));
        assert!(res_normal.0.contains("/static/htmx-1.9.12.min.js"));
        assert!(!res_normal.0.contains("https://unpkg.com"));
    }

    #[test]
    fn test_render_page_edge_cases() {
        let req_normal = HtmxRequest {
            is_htmx: false,
            trigger: None,
            target: None,
            prompt: None,
            current_url: None,
            csp_nonce: None,
        };

        // Empty content
        let res_empty = render_page(&req_normal, "Empty Title", "".to_string());
        assert!(res_empty.0.contains("<title>Empty Title</title>"));
        assert!(res_empty.0.contains("<body class=\"h-full\">"));

        // Unusual titles
        let res_special = render_page(
            &req_normal,
            "<script>alert(1)</script>",
            "content".to_string(),
        );
        assert!(
            res_special
                .0
                .contains("<title>&lt;script&gt;alert(1)&lt;/script&gt;</title>")
        );
    }

    #[tokio::test]
    async fn prompt_answers_outside_ascii_are_decoded() {
        let mut latin1 = axum::http::HeaderMap::new();
        latin1.insert(
            "HX-Prompt",
            HeaderValue::from_bytes(b"Jos\xe9 Concei\xe7\xe3o").unwrap(),
        );
        assert_eq!(
            htmx_header(&latin1, "HX-Prompt").as_deref(),
            Some("José Conceição")
        );

        let request = Request::builder()
            .header("HX-Prompt", "%E6%97%A5%E6%9C%AC%20%E2%82%AC")
            .header("HX-Prompt-URI-AutoEncoded", "true")
            .header("HX-Target", "plain-id")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();
        let htmx = HtmxRequest::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(htmx.prompt.as_deref(), Some("日本 €"));
        assert_eq!(htmx.target.as_deref(), Some("plain-id"));

        let mut malformed = axum::http::HeaderMap::new();
        malformed.insert("HX-Prompt", HeaderValue::from_static("%E6%9"));
        malformed.insert(
            "HX-Prompt-URI-AutoEncoded",
            HeaderValue::from_static("true"),
        );
        assert_eq!(htmx_header(&malformed, "HX-Prompt"), None);
        malformed.insert("HX-Prompt", HeaderValue::from_static("%+f"));
        assert_eq!(htmx_header(&malformed, "HX-Prompt"), None);
        malformed.insert("HX-Prompt", HeaderValue::from_static("%FF"));
        assert_eq!(htmx_header(&malformed, "HX-Prompt"), None);
    }

    #[test]
    fn page_language_defaults_to_pt_br_and_can_be_declared() {
        let request = HtmxRequest {
            is_htmx: false,
            trigger: None,
            target: None,
            prompt: None,
            current_url: None,
            csp_nonce: None,
        };
        let page = render_page(&request, "Painel", String::new());
        assert!(page.0.contains("<html lang=\"pt-BR\""), "{}", page.0);

        let page = render_page_with_lang(&request, "en", "Dashboard", String::new());
        assert!(page.0.contains("<html lang=\"en\""), "{}", page.0);

        let page = render_page_with_lang(&request, "\"><script>", "x", String::new());
        assert!(!page.0.contains("\"><script>"), "{}", page.0);
    }

    #[tokio::test]
    async fn rendered_htmx_script_uses_the_request_csp_nonce() {
        let nonce = crate::security::CspNonce::generate();
        let mut request = Request::new(());
        request.extensions_mut().insert(nonce.clone());
        let (mut parts, _) = request.into_parts();
        let htmx = HtmxRequest::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        let response = render_page(&htmx, "Nonce", "content".to_string());
        assert!(
            response
                .0
                .contains(&format!("nonce=\"{}\"", nonce.as_str()))
        );
    }
}
