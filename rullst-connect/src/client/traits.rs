use async_trait::async_trait;
use serde_json::Value;
use std::fmt;

/// The request structure passed to the HttpClient.
#[derive(Clone)]
#[non_exhaustive]
pub struct HttpRequest {
    pub method: std::borrow::Cow<'static, str>,
    pub url: String,
    pub headers: reqwest::header::HeaderMap,
    pub form: Option<String>,
    pub json: Option<Value>,
    pub basic_auth: Option<(String, Option<String>)>,
    pub bearer_auth: Option<String>,
}

impl HttpRequest {
    /// A request without headers, body or credentials; set those fields on
    /// the value. Rullst builds the requests it sends; this constructor is
    /// for tests of a custom [`HttpClient`]. The struct is
    /// `#[non_exhaustive]` since 13.0.0-alpha.6.
    pub fn new(method: impl Into<std::borrow::Cow<'static, str>>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        }
    }
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header_names: Vec<_> = self.headers.keys().map(|name| name.as_str()).collect();
        let debug_url = redacted_debug_url(&self.url);
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &debug_url)
            .field("header_names", &header_names)
            .field("has_form", &self.form.is_some())
            .field("has_json", &self.json.is_some())
            .field("has_basic_auth", &self.basic_auth.is_some())
            .field("has_bearer_auth", &self.bearer_auth.is_some())
            .finish()
    }
}

fn redacted_debug_url(value: &str) -> String {
    let Ok(mut url) = url::Url::parse(value) else {
        return "<invalid-url>".to_string();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

/// The response structure returned by the HttpClient.
#[derive(Clone)]
#[non_exhaustive]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
}

impl HttpResponse {
    /// The response a custom [`HttpClient`] returns: the HTTP status and the
    /// JSON body. The struct is `#[non_exhaustive]` since 13.0.0-alpha.6, so
    /// clients outside this crate build it here.
    pub fn new(status: u16, body: Value) -> Self {
        Self { status, body }
    }
}

impl fmt::Debug for HttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body_kind = match &self.body {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        formatter
            .debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("body_kind", &body_kind)
            .finish()
    }
}

/// The trait that custom HTTP clients must implement.
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError>;
}
