use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue},
    response::{Html, IntoResponse, Response},
};

use crate as rullst;

/// Extract's HTMX request headers to determine context and re-act re-actively.
#[derive(Debug, Clone)]
pub struct HtmxRequest {
    /// True if the request was triggered by HTMX in the browser (`HX-Request: true`).
    pub is_htmx: bool,
    /// The ID of the triggered element if sent by HTMX (`HX-Trigger`).
    pub trigger: Option<String>,
    /// The ID of the target element if sent by HTMX (`HX-Target`).
    pub target: Option<String>,
    /// The user response inputted into the prompt if sent by HTMX (`HX-Prompt`).
    pub prompt: Option<String>,
    /// The browser's active URL when the request was initiated (`HX-Current-URL`).
    pub current_url: Option<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for HtmxRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let is_htmx = parts.headers
            .get("HX-Request")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);

        let trigger = parts.headers
            .get("HX-Trigger")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let target = parts.headers
            .get("HX-Target")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let prompt = parts.headers
            .get("HX-Prompt")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let current_url = parts.headers
            .get("HX-Current-URL")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok(HtmxRequest {
            is_htmx,
            trigger,
            target,
            prompt,
            current_url,
        })
    }
}

/// A highly ergonomic, builder-style HTMX responder to set dynamic headers in client side.
#[derive(Debug, Clone)]
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

        if let Some(ref trigger) = self.trigger {
            if let Ok(val) = HeaderValue::from_str(trigger) {
                headers.insert("HX-Trigger", val);
            }
        }

        if let Some(ref redirect) = self.redirect {
            if let Ok(val) = HeaderValue::from_str(redirect) {
                headers.insert("HX-Redirect", val);
            }
        }

        if self.refresh {
            headers.insert("HX-Refresh", HeaderValue::from_static("true"));
        }

        res
    }
}

/// Helper function to render a hybrid SSR layout page.
/// - If it is triggered by HTMX, it returns just the inner `content` as a fragment.
/// - Otherwise, it automatically wraps the `content` inside a beautiful HTML5 skeleton pre-configured with TailwindCSS and HTMX script links.
pub fn render_page(htmx: &HtmxRequest, title: &str, content: String) -> Html<String> {
    if htmx.is_htmx {
        Html(content)
    } else {
        let html_content = crate::html! {
            <html lang="pt-BR" class="h-full bg-slate-950 text-slate-100">
                <head>
                    <meta charset="utf-8" />
                    <title>{title}</title>
                    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                    <script src="https://cdn.tailwindcss.com"></script>
                    <script src="https://unpkg.com/htmx.org@1.9.12"></script>
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
mod tests {
    use super::*;
    use axum::http::Request;

    #[tokio::test]
    async fn test_htmx_request_extractor_empty() {
        let req = Request::builder().body(()).unwrap();
        let (mut parts, _) = req.into_parts();
        let htmx_req = HtmxRequest::from_request_parts(&mut parts, &()).await.unwrap();

        assert!(!htmx_req.is_htmx);
        assert!(htmx_req.trigger.is_none());
        assert!(htmx_req.target.is_none());
        assert!(htmx_req.prompt.is_none());
        assert!(htmx_req.current_url.is_none());
    }

    #[tokio::test]
    async fn test_htmx_request_extractor_headers() {
        let req = Request::builder()
            .header("HX-Request", "true")
            .header("HX-Trigger", "my-btn")
            .header("HX-Target", "content-div")
            .header("HX-Prompt", "hello")
            .header("HX-Current-URL", "http://localhost/home")
            .body(())
            .unwrap();
        let (mut parts, _) = req.into_parts();
        let htmx_req = HtmxRequest::from_request_parts(&mut parts, &()).await.unwrap();

        assert!(htmx_req.is_htmx);
        assert_eq!(htmx_req.trigger.as_deref(), Some("my-btn"));
        assert_eq!(htmx_req.target.as_deref(), Some("content-div"));
        assert_eq!(htmx_req.prompt.as_deref(), Some("hello"));
        assert_eq!(htmx_req.current_url.as_deref(), Some("http://localhost/home"));
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
    fn test_render_page_helper() {
        let req_htmx = HtmxRequest {
            is_htmx: true,
            trigger: None,
            target: None,
            prompt: None,
            current_url: None,
        };
        let req_normal = HtmxRequest {
            is_htmx: false,
            trigger: None,
            target: None,
            prompt: None,
            current_url: None,
        };

        // HTMX request -> only the inner content fragment
        let res_htmx = render_page(&req_htmx, "Title", "<div>Fragment</div>".to_string());
        assert_eq!(res_htmx.0, "<div>Fragment</div>");

        // Normal request -> wraps in HTML template
        let res_normal = render_page(&req_normal, "My Page Title", "<div>Body Content</div>".to_string());
        assert!(res_normal.0.contains("<!DOCTYPE html>"));
        assert!(res_normal.0.contains("<title>My Page Title</title>"));
        assert!(res_normal.0.contains("<div>Body Content</div>"));
        assert!(res_normal.0.contains("https://cdn.tailwindcss.com"));
        assert!(res_normal.0.contains("https://unpkg.com/htmx.org"));
    }
}

