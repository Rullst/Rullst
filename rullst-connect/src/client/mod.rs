mod offline;
pub mod request_builder;
pub mod reqwest_client;
pub mod traits;

#[cfg(test)]
mod tests;

pub(crate) use offline::{DisabledHttpClient, OfflineHttpClient};
pub use request_builder::{HttpClientExt, RequestBuilder, ResponseWrapper};
pub use reqwest_client::ReqwestClient;
pub use traits::{HttpClient, HttpRequest, HttpResponse};

pub static DEFAULT_HTTP_CLIENT: std::sync::LazyLock<std::sync::Arc<dyn HttpClient>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(ReqwestClient::new()));
