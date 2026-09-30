//! HTTP transport of the live SurrealDB adapter.

use futures::StreamExt;
use reqwest::{Method, RequestBuilder, Url};
use serde_json::Value;

use super::{LiveSurreal, PolyglotError, StatementEnvelope, SurrealAuth, response_too_large};

impl LiveSurreal {
    pub(super) fn route(&self, segments: &[&str]) -> Result<Url, PolyglotError> {
        let mut url = self.endpoint.clone();
        let mut path =
            url.path_segments_mut()
                .map_err(|_| PolyglotError::InvalidConfiguration {
                    backend: "SurrealDB",
                    reason: "endpoint cannot be used as an HTTP base URL",
                })?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
        drop(path);
        Ok(url)
    }

    pub(super) async fn send(
        &self,
        method: Method,
        url: &Url,
        body: Option<Value>,
    ) -> Result<Vec<StatementEnvelope>, PolyglotError> {
        let request = self.request(method, url);
        let request = if let Some(body) = body {
            request.json(&body)
        } else {
            request
        };
        self.execute(request).await
    }

    pub(super) async fn send_text(
        &self,
        method: Method,
        url: &Url,
        body: &str,
    ) -> Result<Vec<StatementEnvelope>, PolyglotError> {
        self.execute(
            self.request(method, url)
                .header(reqwest::header::CONTENT_TYPE, "text/plain")
                .body(body.to_owned()),
        )
        .await
    }

    fn request(&self, method: Method, url: &Url) -> RequestBuilder {
        let request = self
            .client
            .request(method, url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header("Surreal-NS", self.namespace.as_str())
            .header("Surreal-DB", self.database.as_str());
        match &self.auth {
            SurrealAuth::None => request,
            SurrealAuth::Basic { username, password } => {
                request.basic_auth(username, Some(password))
            }
            SurrealAuth::Bearer(token) => request.bearer_auth(token),
        }
    }

    async fn execute(
        &self,
        request: RequestBuilder,
    ) -> Result<Vec<StatementEnvelope>, PolyglotError> {
        let response = request
            .send()
            .await
            .map_err(|error| PolyglotError::driver("SurrealDB", error))?;
        if !response.status().is_success() {
            return Err(PolyglotError::Driver {
                backend: "SurrealDB",
                message: format!("HTTP status {}", response.status()),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.response_limit as u64)
        {
            return Err(response_too_large(self.response_limit));
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| PolyglotError::driver("SurrealDB", error))?;
            if bytes.len().saturating_add(chunk.len()) > self.response_limit {
                return Err(response_too_large(self.response_limit));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(PolyglotError::serialization)
    }
}
