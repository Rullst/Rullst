use serde_json::Value;

use super::traits::{HttpClient, HttpRequest, HttpResponse};
use crate::error::{
    MAX_PROVIDER_ERROR_CODE_BYTES, MAX_PROVIDER_ERROR_MESSAGE_BYTES, bounded_provider_text,
};

/// A fluent builder for HTTP requests, matching the subset of reqwest used by providers.
pub struct RequestBuilder<'a> {
    client: &'a dyn HttpClient,
    req: HttpRequest,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(
        client: &'a dyn HttpClient,
        method: impl Into<std::borrow::Cow<'static, str>>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            client,
            req: HttpRequest {
                method: method.into(),
                url: url.into(),
                headers: reqwest::header::HeaderMap::new(),
                form: None,
                json: None,
                basic_auth: None,
                bearer_auth: None,
            },
        }
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::try_from(key),
            reqwest::header::HeaderValue::try_from(value),
        ) {
            self.req.headers.insert(name, val);
        }
        self
    }

    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.req.bearer_auth = Some(token.to_owned());
        self
    }

    pub fn basic_auth(
        mut self,
        username: impl Into<String>,
        password: Option<impl Into<String>>,
    ) -> Self {
        self.req.basic_auth = Some((username.into(), password.map(Into::into)));
        self
    }

    pub fn json(mut self, value: Value) -> Self {
        self.req.json = Some(value);
        self
    }

    pub fn form<T: serde::Serialize + ?Sized>(mut self, form: &T) -> Self {
        self.req.form = serde_urlencoded::to_string(form).ok();
        self
    }

    pub async fn send(self) -> Result<ResponseWrapper, crate::error::ConnectError> {
        let res = self.client.execute(self.req).await?;
        Ok(ResponseWrapper { res })
    }
}

#[derive(Debug)]
pub struct ResponseWrapper {
    pub(crate) res: HttpResponse,
}

impl ResponseWrapper {
    /// Returns [`crate::error::ConnectError::ProviderApiError`] for a status of 400 or above.
    ///
    /// `code` is the provider's OAuth `error` value (at most 128 bytes) or
    /// `HTTP_<status>`. `message` is the `error_description`, `message`, JSON
    /// body or text body, at most 512 bytes. Longer provider text is cut at the
    /// last UTF-8 character boundary within the limit and ends with
    /// `... (truncated)`.
    pub fn error_for_status(self) -> Result<Self, crate::error::ConnectError> {
        if self.res.status >= 400 {
            tracing::error!("HTTP status {} received", self.res.status);
            let mut code = format!("HTTP_{}", self.res.status);
            let mut message_opt: Option<String> = None;

            // Provider text is untrusted and may be localized: bound it by bytes
            // without splitting a UTF-8 character, which would panic.
            if let Some(obj) = self.res.body.as_object() {
                if let Some(err) = obj.get("error").and_then(|v| v.as_str()) {
                    code = bounded_provider_text(err, MAX_PROVIDER_ERROR_CODE_BYTES);
                }
                if let Some(desc) = obj.get("error_description").and_then(|v| v.as_str()) {
                    message_opt = Some(bounded_provider_text(
                        desc,
                        MAX_PROVIDER_ERROR_MESSAGE_BYTES,
                    ));
                } else if let Some(msg) = obj.get("message").and_then(|v| v.as_str()) {
                    message_opt =
                        Some(bounded_provider_text(msg, MAX_PROVIDER_ERROR_MESSAGE_BYTES));
                } else {
                    message_opt = Some(bounded_provider_text(
                        &self.res.body.to_string(),
                        MAX_PROVIDER_ERROR_MESSAGE_BYTES,
                    ));
                }
            } else if let Some(s) = self.res.body.as_str() {
                message_opt = Some(bounded_provider_text(s, MAX_PROVIDER_ERROR_MESSAGE_BYTES));
            }

            let message = message_opt.unwrap_or_else(|| "Unknown error".to_string());

            Err(crate::error::ConnectError::ProviderApiError { code, message })
        } else {
            Ok(self)
        }
    }

    pub(crate) fn error_for_status_redacted(
        self,
        operation: &'static str,
    ) -> Result<Self, crate::error::ConnectError> {
        if !(200..300).contains(&self.res.status) {
            Err(crate::error::ConnectError::ProviderApiError {
                code: format!("HTTP_{}", self.res.status),
                message: format!("provider rejected the {operation} request"),
            })
        } else {
            Ok(self)
        }
    }

    pub async fn json<T>(self) -> Result<T, crate::error::ConnectError>
    where
        T: serde::de::DeserializeOwned,
    {
        let t = serde_json::from_value(self.res.body)?;
        Ok(t)
    }
}

/// Extension trait to provide the fluent builder API (like reqwest).
pub trait HttpClientExt {
    fn get(&self, url: impl Into<String>) -> RequestBuilder<'_>;
    fn post(&self, url: impl Into<String>) -> RequestBuilder<'_>;
    fn delete(&self, url: impl Into<String>) -> RequestBuilder<'_>;
}

impl HttpClientExt for dyn HttpClient + '_ {
    fn get(&self, url: impl Into<String>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "GET", url.into())
    }
    fn post(&self, url: impl Into<String>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "POST", url.into())
    }

    fn delete(&self, url: impl Into<String>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "DELETE", url.into())
    }
}
