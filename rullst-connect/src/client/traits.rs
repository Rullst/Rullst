use async_trait::async_trait;
use serde_json::Value;

/// The request structure passed to the HttpClient.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: std::borrow::Cow<'static, str>,
    pub url: String,
    pub headers: reqwest::header::HeaderMap,
    pub form: Option<String>,
    pub json: Option<Value>,
    pub basic_auth: Option<(String, Option<String>)>,
    pub bearer_auth: Option<String>,
}

/// The response structure returned by the HttpClient.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
}

/// The trait that custom HTTP clients must implement.
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError>;
}
