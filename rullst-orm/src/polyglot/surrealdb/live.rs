//! HTTP transport of the live SurrealDB adapter.

use futures::StreamExt;
use reqwest::{Method, RequestBuilder, Url};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

use super::{
    LiveSurreal, PolyglotError, StatementEnvelope, SurrealAuth, response_too_large, server_detail,
    with_detail,
};

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

    /// Runs one SurrealQL statement through the JSON-RPC `query` method.
    /// Unlike `/sql` URL parameters, which always arrive as strings, its
    /// variables keep their JSON types (a document stays an object), and no
    /// value is spliced into the statement text.
    pub(super) async fn rpc_query(
        &self,
        statement: &'static str,
        variables: Value,
    ) -> Result<Vec<StatementEnvelope>, PolyglotError> {
        let body = serde_json::json!({
            "id": 1,
            "method": "query",
            "params": [statement, variables],
        });
        let request = self
            .request(Method::POST, &self.route(&["rpc"])?)
            .json(&body);
        let response: RpcResponse = self.execute(request).await?;
        match response {
            RpcResponse {
                result: Some(envelopes),
                error: None,
            } => Ok(envelopes),
            RpcResponse { error, .. } => {
                let reason = error
                    .as_ref()
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .and_then(|message| self.detail(message));
                Err(PolyglotError::Driver {
                    backend: "SurrealDB",
                    message: with_detail("RPC query returned an error".to_owned(), reason),
                })
            }
        }
    }

    /// The server's reason for a failure, without the configured password or
    /// token should the server ever echo it.
    fn detail(&self, text: &str) -> Option<String> {
        let secret: &str = match &self.auth {
            SurrealAuth::None => "",
            SurrealAuth::Basic { password, .. } => password,
            SurrealAuth::Bearer(token) => token,
        };
        if secret.is_empty() {
            server_detail(text)
        } else {
            server_detail(&text.replace(secret, "[redacted]"))
        }
    }

    /// The reason in a failed HTTP response: the `information` of
    /// SurrealDB's JSON error, otherwise the body text.
    fn response_detail(&self, body: &[u8]) -> Option<String> {
        let text = String::from_utf8_lossy(body);
        let information = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|error| error.get("information")?.as_str().map(str::to_owned));
        self.detail(information.as_deref().unwrap_or(&text))
    }

    async fn execute<R: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<R, PolyglotError> {
        let response = request
            .send()
            .await
            .map_err(|error| PolyglotError::driver("SurrealDB", error))?;
        let status = response.status();
        if !status.is_success() {
            // The reason is read under the same bound; a body over it is
            // reported without one.
            let body = self.read_body(response).await.unwrap_or_default();
            return Err(PolyglotError::Driver {
                backend: "SurrealDB",
                message: with_detail(format!("HTTP status {status}"), self.response_detail(&body)),
            });
        }
        let bytes = self.read_body(response).await?;
        serde_json::from_slice(&bytes).map_err(PolyglotError::serialization)
    }

    async fn read_body(&self, response: reqwest::Response) -> Result<Vec<u8>, PolyglotError> {
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
        Ok(bytes)
    }
}

/// JSON-RPC response envelope: statement results or an error object.
#[derive(Deserialize)]
struct RpcResponse {
    #[serde(default)]
    result: Option<Vec<StatementEnvelope>>,
    #[serde(default)]
    error: Option<Value>,
}
