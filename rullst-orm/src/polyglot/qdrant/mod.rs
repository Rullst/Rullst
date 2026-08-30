//! Bounded Qdrant dense-vector adapter.

use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url, redirect::Policy};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{Backend, BackendCapabilities, Capability, PolyglotError};

mod config;
mod mock;
#[cfg(test)]
mod tests;
mod types;

pub use config::QdrantConfig;
use config::ValidatedQdrantConfig;
use mock::MockQdrant;
pub use types::{
    VectorCollectionName, VectorDimensions, VectorMatch, VectorPoint, VectorQueryLimit,
};
use types::{validate_payload, validate_vector};

const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

/// Static-dispatch-friendly Qdrant operations supported by this adapter.
#[async_trait]
pub trait VectorRepository: Send + Sync {
    /// Creates one dense-vector collection using cosine similarity.
    async fn create_collection(
        &self,
        collection: &VectorCollectionName,
        dimensions: VectorDimensions,
    ) -> Result<(), PolyglotError>;

    /// Inserts or replaces one point and waits for the write to apply.
    async fn upsert(
        &self,
        collection: &VectorCollectionName,
        point: VectorPoint,
    ) -> Result<(), PolyglotError>;

    /// Deletes one point and waits for the write to apply.
    async fn delete(&self, collection: &VectorCollectionName, id: u64)
    -> Result<(), PolyglotError>;

    /// Returns bounded nearest neighbors using cosine similarity.
    async fn search(
        &self,
        collection: &VectorCollectionName,
        query: &[f32],
        limit: VectorQueryLimit,
    ) -> Result<Vec<VectorMatch>, PolyglotError>;
}

struct LiveQdrant {
    client: Client,
    endpoint: Url,
    api_key: Option<String>,
    response_limit: usize,
}

enum QdrantInner {
    Live(LiveQdrant),
    Mock(MockQdrant),
}

/// Qdrant HTTP store with a deterministic offline fallback.
pub struct QdrantStore {
    inner: QdrantInner,
}

impl QdrantStore {
    /// Builds the HTTP adapter or selects the fallback for empty/`mock_*`
    /// endpoint or API-key values.
    pub fn connect_or_mock(config: QdrantConfig) -> Result<Self, PolyglotError> {
        if config.requests_mock() {
            return Ok(Self {
                inner: QdrantInner::Mock(MockQdrant::default()),
            });
        }
        let config = config.validate()?;
        Ok(Self {
            inner: QdrantInner::Live(LiveQdrant::new(config)?),
        })
    }

    /// Declares the adapter's bounded behavior.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(Backend::Qdrant, &[Capability::Vectors])
    }

    /// Returns whether this instance uses the deterministic offline store.
    pub const fn is_mock(&self) -> bool {
        matches!(&self.inner, QdrantInner::Mock(_))
    }
}

#[async_trait]
impl VectorRepository for QdrantStore {
    async fn create_collection(
        &self,
        collection: &VectorCollectionName,
        dimensions: VectorDimensions,
    ) -> Result<(), PolyglotError> {
        let QdrantInner::Live(live) = &self.inner else {
            return qdrant_mock(self)?
                .create_collection(collection, dimensions)
                .await;
        };
        let body = CreateCollection {
            vectors: VectorParameters {
                size: dimensions.get(),
                distance: "Cosine",
            },
        };
        live.send_json::<Value>(
            live.request(Method::PUT, &collection_route(live, collection)?),
            &body,
        )
        .await
        .map(|_| ())
    }

    async fn upsert(
        &self,
        collection: &VectorCollectionName,
        point: VectorPoint,
    ) -> Result<(), PolyglotError> {
        let QdrantInner::Live(live) = &self.inner else {
            return qdrant_mock(self)?.upsert(collection, point).await;
        };
        validate_vector(point.vector(), None)?;
        validate_payload(point.payload())?;
        let mut route = points_route(live, collection, &[])?;
        route.query_pairs_mut().append_pair("wait", "true");
        live.send_json::<Value>(
            live.request(Method::PUT, &route),
            &UpsertPoints {
                points: std::slice::from_ref(&point),
            },
        )
        .await
        .map(|_| ())
    }

    async fn delete(
        &self,
        collection: &VectorCollectionName,
        id: u64,
    ) -> Result<(), PolyglotError> {
        let QdrantInner::Live(live) = &self.inner else {
            return qdrant_mock(self)?.delete(collection, id).await;
        };
        let mut route = points_route(live, collection, &["delete"])?;
        route.query_pairs_mut().append_pair("wait", "true");
        live.send_json::<Value>(
            live.request(Method::POST, &route),
            &DeletePoints { points: &[id] },
        )
        .await
        .map(|_| ())
    }

    async fn search(
        &self,
        collection: &VectorCollectionName,
        query: &[f32],
        limit: VectorQueryLimit,
    ) -> Result<Vec<VectorMatch>, PolyglotError> {
        let QdrantInner::Live(live) = &self.inner else {
            return qdrant_mock(self)?.search(collection, query, limit).await;
        };
        validate_vector(query, None)?;
        let response = live
            .send_json::<QueryResult>(
                live.request(Method::POST, &points_route(live, collection, &["query"])?),
                &QueryPoints {
                    query,
                    limit: limit.get(),
                    with_payload: true,
                    with_vector: false,
                },
            )
            .await?;
        if response.points.len() > usize::from(limit.get()) {
            return Err(invalid_response("server exceeded the query result limit"));
        }
        for result in &response.points {
            if !result.score().is_finite() {
                return Err(invalid_response("server returned a non-finite score"));
            }
            validate_payload(result.payload())?;
        }
        Ok(response.points)
    }
}

impl LiveQdrant {
    fn new(config: ValidatedQdrantConfig) -> Result<Self, PolyglotError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| transport_error())?;
        Ok(Self {
            client,
            endpoint: config.endpoint,
            api_key: config.api_key,
            response_limit: config.response_limit,
        })
    }

    fn request(&self, method: Method, url: &Url) -> RequestBuilder {
        let request = self
            .client
            .request(method, url.clone())
            .header(reqwest::header::ACCEPT, "application/json");
        if let Some(api_key) = &self.api_key {
            request.header("api-key", api_key)
        } else {
            request
        }
    }

    async fn send_json<T>(
        &self,
        request: RequestBuilder,
        body: &impl Serialize,
    ) -> Result<T, PolyglotError>
    where
        T: DeserializeOwned,
    {
        let bytes = serde_json::to_vec(body).map_err(PolyglotError::serialization)?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Qdrant",
                reason: "request body must not exceed 4 MiB",
            });
        }
        let response = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes)
            .send()
            .await
            .map_err(|_| transport_error())?;
        let status = response.status();
        if status == StatusCode::CONFLICT {
            return Err(PolyglotError::Conflict);
        }
        if status == StatusCode::NOT_FOUND {
            return Err(PolyglotError::NotFound);
        }
        if !status.is_success() {
            return Err(PolyglotError::Driver {
                backend: "Qdrant",
                message: format!("HTTP status {status}"),
            });
        }
        let bytes = bounded_response(response, self.response_limit).await?;
        let envelope: Envelope<T> =
            serde_json::from_slice(&bytes).map_err(PolyglotError::serialization)?;
        if envelope.status != "ok" {
            return Err(invalid_response("server returned a non-ok status"));
        }
        Ok(envelope.result)
    }
}

async fn bounded_response(
    response: Response,
    response_limit: usize,
) -> Result<Vec<u8>, PolyglotError> {
    if response
        .content_length()
        .is_some_and(|length| length > response_limit as u64)
    {
        return Err(response_too_large(response_limit));
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| transport_error())?;
        if bytes.len().saturating_add(chunk.len()) > response_limit {
            return Err(response_too_large(response_limit));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn collection_route(
    live: &LiveQdrant,
    collection: &VectorCollectionName,
) -> Result<Url, PolyglotError> {
    route(live, &["collections", collection.as_str()])
}

fn points_route(
    live: &LiveQdrant,
    collection: &VectorCollectionName,
    suffix: &[&str],
) -> Result<Url, PolyglotError> {
    let mut segments = vec!["collections", collection.as_str(), "points"];
    segments.extend_from_slice(suffix);
    route(live, &segments)
}

fn route(live: &LiveQdrant, segments: &[&str]) -> Result<Url, PolyglotError> {
    let mut url = live.endpoint.clone();
    let mut path = url
        .path_segments_mut()
        .map_err(|_| invalid_response("endpoint cannot be used as an HTTP base URL"))?;
    path.pop_if_empty();
    for segment in segments {
        path.push(segment);
    }
    drop(path);
    Ok(url)
}

fn qdrant_mock(store: &QdrantStore) -> Result<&MockQdrant, PolyglotError> {
    match &store.inner {
        QdrantInner::Mock(mock) => Ok(mock),
        QdrantInner::Live(_) => Err(invalid_response("internal backend mismatch")),
    }
}

fn transport_error() -> PolyglotError {
    PolyglotError::Driver {
        backend: "Qdrant",
        message: "HTTP transport failed".to_owned(),
    }
}

fn invalid_response(reason: &'static str) -> PolyglotError {
    PolyglotError::Driver {
        backend: "Qdrant",
        message: reason.to_owned(),
    }
}

fn response_too_large(limit_bytes: usize) -> PolyglotError {
    PolyglotError::ResponseTooLarge {
        backend: "Qdrant",
        limit_bytes,
    }
}

#[derive(Serialize)]
struct CreateCollection {
    vectors: VectorParameters,
}

#[derive(Serialize)]
struct VectorParameters {
    size: u32,
    distance: &'static str,
}

#[derive(Serialize)]
struct UpsertPoints<'a> {
    points: &'a [VectorPoint],
}

#[derive(Serialize)]
struct DeletePoints<'a> {
    points: &'a [u64],
}

#[derive(Serialize)]
struct QueryPoints<'a> {
    query: &'a [f32],
    limit: u16,
    with_payload: bool,
    with_vector: bool,
}

#[derive(Deserialize)]
struct Envelope<T> {
    status: String,
    result: T,
}

#[derive(Deserialize)]
struct QueryResult {
    points: Vec<VectorMatch>,
}
