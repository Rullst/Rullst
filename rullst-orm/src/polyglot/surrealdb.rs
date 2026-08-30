//! SurrealDB HTTP adapter.
//!
//! The adapter intentionally uses the public HTTP protocol instead of
//! embedding the SurrealDB SDK. This keeps the MIT-licensed framework from
//! implicitly redistributing the database engine's BSL dependency.

use std::{marker::PhantomData, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Client, Method, RequestBuilder, Url, redirect::Policy};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{
    Backend, BackendCapabilities, Capability, CollectionName, DocumentId, DocumentPage,
    DocumentRepository, MockDocumentStore, PolyglotError,
};

mod config;
mod graph;
#[cfg(test)]
mod tests;

pub use config::{SurrealAuth, SurrealConfig};
use config::{is_mock_credential, validate_endpoint};
pub use graph::{GraphQuery, GraphRepository};

struct LiveSurreal {
    client: Client,
    endpoint: Url,
    namespace: CollectionName,
    database: CollectionName,
    auth: SurrealAuth,
    response_limit: usize,
}

enum SurrealDbInner<T> {
    Live {
        live: LiveSurreal,
        marker: PhantomData<fn() -> T>,
    },
    Mock(MockDocumentStore<T>),
}

/// SurrealDB multi-model adapter with deterministic mock fallback.
pub struct SurrealDbStore<T> {
    inner: SurrealDbInner<T>,
}

impl<T> SurrealDbStore<T> {
    /// Builds the HTTP adapter or selects the deterministic fallback.
    pub fn connect_or_mock(config: SurrealConfig) -> Result<Self, PolyglotError> {
        let namespace = CollectionName::new(config.namespace)?;
        let database = CollectionName::new(config.database)?;
        if is_mock_credential(&config.endpoint) || config.auth.requests_mock() {
            return Ok(Self {
                inner: SurrealDbInner::Mock(MockDocumentStore::new()),
            });
        }
        let endpoint = validate_endpoint(&config.endpoint, config.allow_insecure_http)?;
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| PolyglotError::driver("SurrealDB", error))?;
        Ok(Self {
            inner: SurrealDbInner::Live {
                live: LiveSurreal {
                    client,
                    endpoint,
                    namespace,
                    database,
                    auth: config.auth,
                    response_limit: config.response_limit,
                },
                marker: PhantomData,
            },
        })
    }

    /// Declares the adapter's bounded behavior.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(
            Backend::SurrealDb,
            &[Capability::Documents, Capability::Graph],
        )
    }

    /// Returns whether this instance is using the deterministic offline store.
    pub const fn is_mock(&self) -> bool {
        matches!(&self.inner, SurrealDbInner::Mock(_))
    }
}

#[async_trait]
impl<T> DocumentRepository<T> for SurrealDbStore<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    async fn create(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let SurrealDbInner::Live { live, .. } = &self.inner else {
            return surreal_mock(self)?.create(collection, id, entity).await;
        };
        let body = encode_document(entity)?;
        let envelopes = live
            .send(
                Method::POST,
                &record_route(live, collection, id)?,
                Some(body),
            )
            .await?;
        statement_result(envelopes, true).map(|_| ())
    }

    async fn find(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<Option<T>, PolyglotError> {
        let SurrealDbInner::Live { live, .. } = &self.inner else {
            return surreal_mock(self)?.find(collection, id).await;
        };
        let envelopes = live
            .send(Method::GET, &record_route(live, collection, id)?, None)
            .await?;
        let mut values = result_values(statement_result(envelopes, false)?)?;
        if values.len() > 1 {
            return Err(invalid_surreal_response(
                "single-record lookup returned multiple rows",
            ));
        }
        values.pop().map(decode_document).transpose()
    }

    async fn replace(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let SurrealDbInner::Live { live, .. } = &self.inner else {
            return surreal_mock(self)?.replace(collection, id, entity).await;
        };
        let body = encode_document(entity)?;
        let envelopes = live
            .send(
                Method::PUT,
                &record_route(live, collection, id)?,
                Some(body),
            )
            .await?;
        if result_values(statement_result(envelopes, false)?)?.is_empty() {
            return Err(PolyglotError::NotFound);
        }
        Ok(())
    }

    async fn delete(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<bool, PolyglotError> {
        let SurrealDbInner::Live { live, .. } = &self.inner else {
            return surreal_mock(self)?.delete(collection, id).await;
        };
        let envelopes = live
            .send(Method::DELETE, &record_route(live, collection, id)?, None)
            .await?;
        Ok(!result_values(statement_result(envelopes, false)?)?.is_empty())
    }

    async fn list(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<T>, PolyglotError> {
        let SurrealDbInner::Live { live, .. } = &self.inner else {
            return surreal_mock(self)?.list(collection, page).await;
        };
        let mut url = live.route(&["sql"])?;
        url.query_pairs_mut()
            .append_pair("table", collection.as_str())
            .append_pair("start", &page.offset().to_string())
            .append_pair("limit", &page.limit().to_string());
        let envelopes = live
            .send_text(
                Method::POST,
                &url,
                "SELECT * FROM type::table($table) ORDER BY id START type::int($start) LIMIT type::int($limit)",
            )
            .await?;
        let values = result_values(statement_result(envelopes, false)?)?;
        if values.len() > page.limit() as usize {
            return Err(invalid_surreal_response(
                "server exceeded the requested page limit",
            ));
        }
        values.into_iter().map(decode_document).collect()
    }
}

#[async_trait]
impl<T> GraphRepository for SurrealDbStore<T>
where
    T: Send + Sync,
{
    async fn query_graph(&self, query: &GraphQuery) -> Result<Vec<Value>, PolyglotError> {
        let SurrealDbInner::Live { live, .. } = &self.inner else {
            return Ok(Vec::new());
        };
        let envelopes = live
            .send_text(Method::POST, &live.route(&["gql"])?, &query.bounded_query())
            .await?;
        let values = result_values(statement_result(envelopes, false)?)?;
        if values.len() > query.limit as usize {
            return Err(invalid_surreal_response(
                "server exceeded the graph row limit",
            ));
        }
        Ok(values)
    }
}

impl LiveSurreal {
    fn route(&self, segments: &[&str]) -> Result<Url, PolyglotError> {
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

    async fn send(
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

    async fn send_text(
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

#[derive(Debug, Deserialize)]
struct StatementEnvelope {
    status: String,
    #[serde(default)]
    result: Value,
}

fn statement_result(
    mut envelopes: Vec<StatementEnvelope>,
    conflict_on_duplicate: bool,
) -> Result<Value, PolyglotError> {
    if envelopes.len() != 1 {
        return Err(invalid_surreal_response(
            "expected exactly one statement result",
        ));
    }
    let envelope = envelopes.remove(0);
    if envelope.status != "OK" {
        let detail = envelope
            .result
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if conflict_on_duplicate && detail.contains("already exists") {
            return Err(PolyglotError::Conflict);
        }
        return Err(PolyglotError::Driver {
            backend: "SurrealDB",
            message: format!("statement returned status {}", envelope.status),
        });
    }
    Ok(envelope.result)
}

fn result_values(result: Value) -> Result<Vec<Value>, PolyglotError> {
    match result {
        Value::Array(values) => Ok(values),
        Value::Null => Ok(Vec::new()),
        _ => Err(invalid_surreal_response(
            "statement result was not an array",
        )),
    }
}

fn encode_document<T: Serialize>(entity: &T) -> Result<Value, PolyglotError> {
    let value = serde_json::to_value(entity).map_err(PolyglotError::serialization)?;
    let Some(object) = value.as_object() else {
        return Err(invalid_surreal_response(
            "portable models must serialize as JSON objects",
        ));
    };
    if object.contains_key("id") {
        return Err(PolyglotError::InvalidIdentifier {
            kind: "SurrealDB model",
            reason: "the portable model must not define an id field",
        });
    }
    Ok(value)
}

fn decode_document<T: DeserializeOwned>(mut value: Value) -> Result<T, PolyglotError> {
    let Some(object) = value.as_object_mut() else {
        return Err(invalid_surreal_response(
            "document result was not a JSON object",
        ));
    };
    object.remove("id");
    serde_json::from_value(value).map_err(PolyglotError::serialization)
}

fn record_route(
    live: &LiveSurreal,
    collection: &CollectionName,
    id: &DocumentId,
) -> Result<Url, PolyglotError> {
    live.route(&["key", collection.as_str(), id.as_str()])
}

fn surreal_mock<T>(store: &SurrealDbStore<T>) -> Result<&MockDocumentStore<T>, PolyglotError> {
    match store {
        SurrealDbStore {
            inner: SurrealDbInner::Mock(store),
        } => Ok(store),
        SurrealDbStore {
            inner: SurrealDbInner::Live { .. },
        } => Err(invalid_surreal_response(
            "adapter state changed during operation",
        )),
    }
}

fn invalid_surreal_response(message: &'static str) -> PolyglotError {
    PolyglotError::Driver {
        backend: "SurrealDB",
        message: message.to_owned(),
    }
}

fn response_too_large(limit_bytes: usize) -> PolyglotError {
    PolyglotError::ResponseTooLarge {
        backend: "SurrealDB",
        limit_bytes,
    }
}
