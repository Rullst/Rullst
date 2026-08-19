use serde_json::Value;

use super::traits::{HttpClient, HttpRequest, HttpResponse};

/// A fluent builder for HTTP requests, matching the subset of reqwest used by providers.
pub struct RequestBuilder<'a> {
    client: &'a dyn HttpClient,
    req: HttpRequest,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(
        client: &'a dyn HttpClient,
        method: impl Into<std::borrow::Cow<'static, str>>,
        url: String,
    ) -> Self {
        Self {
            client,
            req: HttpRequest {
                method: method.into(),
                url,
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
    pub fn error_for_status(self) -> Result<Self, crate::error::ConnectError> {
        if self.res.status >= 400 {
            tracing::error!("HTTP status {} received", self.res.status);
            let mut code = format!("HTTP_{}", self.res.status);
            let mut message_opt: Option<String> = None;

            if let Some(obj) = self.res.body.as_object() {
                if let Some(err) = obj.get("error").and_then(|v| v.as_str()) {
                    code = err.to_string();
                }
                if let Some(desc) = obj.get("error_description").and_then(|v| v.as_str()) {
                    message_opt = Some(desc.to_string());
                } else if let Some(msg) = obj.get("message").and_then(|v| v.as_str()) {
                    message_opt = Some(msg.to_string());
                } else {
                    message_opt = Some(self.res.body.to_string());
                }
            } else if let Some(s) = self.res.body.as_str() {
                message_opt = Some(s.to_string());
            }

            let mut message = message_opt.unwrap_or_else(|| "Unknown error".to_string());

            // Prevent sensitive information exposure or massive log spam
            if message.len() > 512 {
                message.truncate(512);
                message.push_str("... (truncated)");
            }

            Err(crate::error::ConnectError::ProviderApiError { code, message })
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
