# Qdrant vector search with the official client

This recipe uses the official [`qdrant-client`](https://crates.io/crates/qdrant-client)
crate in a Rullst application: one client created at start-up, a small
application trait that handlers depend on, an in-memory fake for tests and
local work, bounded tenant-scoped queries, and integration tests against a
real server. It replaces the `QdrantStore` adapter that 13.0.0-alpha.5
removed; see [Migrating from `QdrantStore`](#migrating-from-qdrantstore). The
code is yours (see [what a recipe is](index.md#what-a-recipe-is)).

The blocks that use Rullst types (the trait, the fake, the handler and its
test) are compiled and run as doctests in Rullst's CI. The blocks marked as
client code are not compiled there, because the Rullst workspace deliberately
does not depend on `qdrant-client`; they are written against its 1.x builder
API (1.10 or later; the current release is 1.20).

## When to use it

Use Qdrant when the application already runs it, or when its vector set
outgrows the primary database. Vectors that live next to the rows they
describe are usually simpler in PostgreSQL with pgvector: `rullst-orm`'s
`pgvector` feature keeps them in the same transaction, backup and tenant
filter as the rest of the data (see
[the RAG and vector search tutorial](../tutorials/22-rag-vector-search.md)).

## Dependency

```toml
[dependencies]
qdrant-client = { version = "1", default-features = false, features = ["serde"] }
```

`default-features = false` leaves out `download_snapshots`, which adds a
`reqwest` client only for downloading snapshots. The client talks gRPC
(port 6334, not the REST port 6333) through `tonic`, whose TLS uses ring as
the rustls provider; Rullst's own TLS connections use aws-lc-rs
([cryptography providers](../security-architecture.md#cryptography-providers)),
and ring is already in the graph of every Rullst application as a primitives
library, so this adds a TLS provider, not a new crate.

## Configuration

Read the settings with `rullst::config::project_setting`, which checks the
process environment first and then the project's `.env`. An empty or absent
`QDRANT_URL`, or one that starts with `mock_`, selects the in-memory index, so
a fresh checkout runs without a server:

```rust,no_run
use rullst::config::project_setting;

/// Where the vector index lives.
pub enum VectorSettings {
    /// No server configured: use the in-memory index.
    Offline,
    /// A Qdrant server (its gRPC URL) and its API key, if it needs one.
    Qdrant { url: String, api_key: Option<String> },
}

/// Reads `QDRANT_URL` and `QDRANT_API_KEY`.
pub fn vector_settings() -> Result<VectorSettings, Box<dyn std::error::Error + Send + Sync>> {
    let url = project_setting("QDRANT_URL")?.unwrap_or_default();
    if url.is_empty() || url.starts_with("mock_") {
        return Ok(VectorSettings::Offline);
    }
    let api_key = project_setting("QDRANT_API_KEY")?.filter(|key| !key.is_empty());
    if api_key.is_none() && !url.starts_with("http://127.0.0.1") && !url.starts_with("http://[::1]") {
        return Err("QDRANT_API_KEY is required for a Qdrant server outside this host".into());
    }
    Ok(VectorSettings::Qdrant { url, api_key })
}
```

```text
# .env (development; never commit production credentials)
QDRANT_URL=mock_local
# QDRANT_URL=http://127.0.0.1:6334
# QDRANT_API_KEY=...
```

In production, inject `QDRANT_API_KEY` from the platform's secret store and
use an `https://` URL; never log either.

## The application boundary

Handlers depend on a small trait that names what the application needs: here,
the notes most similar to a query vector, within the signed-in user's tenant.
The vectors come from an embedding model, for example
`rullst::ai::AiClient::embed` with the same model for stored notes and queries
(vectors of different models are not comparable). The trait's futures are
`Send`, and handlers are generic over the index (static dispatch). The
in-memory fake implements the same trait with an exact cosine search:

```rust
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use rullst::Router;
use rullst::server::{Extension, IntoResponse, Json, Response, StatusCode, post};
use serde::{Deserialize, Serialize};

/// The embedding size of the application's model (768 for `nomic-embed-text`).
pub const DIMENSIONS: usize = 768;

/// Bounds every search, whatever the client asks for.
pub const MAX_MATCHES: u64 = 20;

/// One similar note, best first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteMatch {
    pub note_id: u64,
    pub score: f32,
}

/// Failures of the vector index. Client details stay in the server log.
#[derive(Debug)]
pub enum IndexError {
    /// The vector has the wrong size or a value that is not finite.
    InvalidVector,
    /// The index could not complete the operation.
    Unavailable(String),
}

impl fmt::Display for IndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVector => {
                write!(formatter, "a vector needs {DIMENSIONS} finite values")
            }
            Self::Unavailable(detail) => write!(formatter, "vector index unavailable: {detail}"),
        }
    }
}

impl std::error::Error for IndexError {}

impl IntoResponse for IndexError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidVector => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
            Self::Unavailable(detail) => {
                tracing::error!(%detail, "vector index unavailable");
                StatusCode::SERVICE_UNAVAILABLE.into_response()
            }
        }
    }
}

/// Rejects a vector of the wrong size or with a NaN or infinite value before
/// it reaches the index.
pub fn check_vector(vector: &[f32]) -> Result<(), IndexError> {
    if vector.len() == DIMENSIONS && vector.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(IndexError::InvalidVector)
    }
}

/// What the application needs from its vector index. Every call names the
/// tenant, so no query can reach another tenant's notes.
pub trait NoteIndex: Clone + Send + Sync + 'static {
    /// Stores or replaces the vector of one note.
    fn upsert(
        &self,
        tenant_id: &str,
        note_id: u64,
        vector: Vec<f32>,
    ) -> impl Future<Output = Result<(), IndexError>> + Send;

    /// The tenant's notes most similar to `vector`, at most `limit` of them.
    fn similar(
        &self,
        tenant_id: &str,
        vector: &[f32],
        limit: u64,
    ) -> impl Future<Output = Result<Vec<NoteMatch>, IndexError>> + Send;

    /// Removes the vector of one of the tenant's notes.
    fn remove(
        &self,
        tenant_id: &str,
        note_id: u64,
    ) -> impl Future<Output = Result<(), IndexError>> + Send;
}

/// In-process index for tests and for an offline `QDRANT_URL`. Clones share
/// the same vectors, which live as long as the process.
#[derive(Clone, Default)]
pub struct MemoryNoteIndex {
    vectors: Arc<Mutex<BTreeMap<(String, u64), Vec<f32>>>>,
}

impl MemoryNoteIndex {
    fn vectors(&self) -> Result<MutexGuard<'_, BTreeMap<(String, u64), Vec<f32>>>, IndexError> {
        self.vectors
            .lock()
            .map_err(|_| IndexError::Unavailable("the in-memory index lock is poisoned".to_owned()))
    }
}

/// Cosine similarity, 0 for a zero vector (as a normalizing index treats it).
fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let norm = |values: &[f32]| values.iter().map(|value| value * value).sum::<f32>().sqrt();
    let norms = norm(left) * norm(right);
    if norms == 0.0 { 0.0 } else { dot / norms }
}

impl NoteIndex for MemoryNoteIndex {
    async fn upsert(&self, tenant_id: &str, note_id: u64, vector: Vec<f32>) -> Result<(), IndexError> {
        check_vector(&vector)?;
        self.vectors()?.insert((tenant_id.to_owned(), note_id), vector);
        Ok(())
    }

    async fn similar(
        &self,
        tenant_id: &str,
        vector: &[f32],
        limit: u64,
    ) -> Result<Vec<NoteMatch>, IndexError> {
        check_vector(vector)?;
        let vectors = self.vectors()?;
        let mut matches: Vec<NoteMatch> = vectors
            .iter()
            .filter(|((tenant, _), _)| tenant == tenant_id)
            .map(|((_, note_id), stored)| NoteMatch { note_id: *note_id, score: cosine(vector, stored) })
            .collect();
        matches.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.note_id.cmp(&b.note_id)));
        matches.truncate(usize::try_from(limit.min(MAX_MATCHES)).unwrap_or(0));
        Ok(matches)
    }

    async fn remove(&self, tenant_id: &str, note_id: u64) -> Result<(), IndexError> {
        self.vectors()?.remove(&(tenant_id.to_owned(), note_id));
        Ok(())
    }
}

/// The signed-in user, inserted by the application's authentication layer.
#[derive(Clone)]
pub struct CurrentUser {
    pub id: String,
    pub tenant_id: String,
}

/// The body of `POST /notes/similar`: the query's embedding.
#[derive(Deserialize)]
pub struct SimilarRequest {
    pub vector: Vec<f32>,
}

/// `POST /notes/similar`: the signed-in tenant's most similar notes.
async fn similar_notes<I: NoteIndex>(
    Extension(index): Extension<I>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<SimilarRequest>,
) -> Result<Json<Vec<NoteMatch>>, IndexError> {
    check_vector(&request.vector)?;
    let matches = index.similar(&user.tenant_id, &request.vector, MAX_MATCHES).await?;
    Ok(Json(matches))
}

/// The note-search routes with the index selected at start-up.
pub fn note_search_routes<I: NoteIndex>(index: I) -> Router {
    Router::new()
        .route("/notes/similar", post(similar_notes::<I>))
        .layer(Extension(index))
}

/// A unit vector along one axis, for tests.
fn axis(index: usize) -> Vec<f32> {
    let mut vector = vec![0.0; DIMENSIONS];
    vector[index] = 1.0;
    vector
}

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
// A unit test of the application: the fake, the real routes, no server.
let index = MemoryNoteIndex::default();
index.upsert("tenant-a", 1, axis(0)).await?;
index.upsert("tenant-a", 2, axis(1)).await?;
index.upsert("tenant-b", 3, axis(0)).await?;
let app = rullst::testing::TestApp::new(
    note_search_routes(index)
        .layer(Extension(CurrentUser { id: "user-1".into(), tenant_id: "tenant-a".into() }))
        .into_axum(),
);
let response = app
    .post("/notes/similar")
    .json(&serde_json::json!({ "vector": axis(0) }))
    .send()
    .await;
response.assert_status(200);
let matches: Vec<NoteMatch> = response.json();
// Tenant B's identical vector is not visible; the closest note comes first.
let ids: Vec<u64> = matches.iter().map(|found| found.note_id).collect();
assert_eq!(ids, [1, 2]);
assert!(matches[0].score > matches[1].score);

let wrong_size = app
    .post("/notes/similar")
    .json(&serde_json::json!({ "vector": [1.0, 0.0] }))
    .send()
    .await;
wrong_size.assert_status(422);
# Ok(())
# }
```

`note_search_routes` returns a `rullst::Router`; mount it with the
application's other routes, behind the authentication layer that inserts
`CurrentUser` and behind the production security layers (CSRF for
state-changing routes, secure headers, rate limits) like any other route.

## The Qdrant implementation

Client code (not compiled by Rullst's CI). All tenants share one collection;
each point carries its tenant in the payload, a keyword index makes that
filter fast, and every search and delete filters on it. The point identifier
is the note's database primary key, unique across tenants:

```rust,ignore
use std::time::Duration;

use qdrant_client::qdrant::point_id::PointIdOptions;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    DeletePointsBuilder, Distance, FieldType, Filter, PointStruct, QueryPointsBuilder,
    UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant, QdrantError};

const COLLECTION: &str = "notes";

/// Qdrant-backed index. Cloning shares the client's gRPC channel.
#[derive(Clone)]
pub struct QdrantNoteIndex {
    client: std::sync::Arc<Qdrant>,
}

fn unavailable(error: QdrantError) -> IndexError {
    IndexError::Unavailable(error.to_string())
}

fn tenant_filter(tenant_id: &str) -> Filter {
    Filter::must([Condition::matches("tenant_id", tenant_id.to_owned())])
}

impl QdrantNoteIndex {
    /// Builds the client once, at start-up, with bounded timeouts.
    pub fn connect(url: &str, api_key: Option<String>) -> Result<Self, IndexError> {
        let client = Qdrant::from_url(url)
            .api_key(api_key)
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .build()
            // The error can name the URL: report a fixed text.
            .map_err(|_| IndexError::Unavailable("QDRANT_URL could not be used".to_owned()))?;
        Ok(Self { client: std::sync::Arc::new(client) })
    }

    /// Creates the collection and the tenant index. Run it at start-up or in
    /// a deployment step; it is idempotent.
    pub async fn ensure_collection(&self) -> Result<(), IndexError> {
        if !self.client.collection_exists(COLLECTION).await.map_err(unavailable)? {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(COLLECTION)
                        .vectors_config(VectorParamsBuilder::new(DIMENSIONS as u64, Distance::Cosine)),
                )
                .await
                .map_err(unavailable)?;
        }
        self.client
            .create_field_index(
                CreateFieldIndexCollectionBuilder::new(COLLECTION, "tenant_id", FieldType::Keyword)
                    .wait(true),
            )
            .await
            .map_err(unavailable)?;
        Ok(())
    }
}

impl NoteIndex for QdrantNoteIndex {
    async fn upsert(&self, tenant_id: &str, note_id: u64, vector: Vec<f32>) -> Result<(), IndexError> {
        check_vector(&vector)?;
        let payload = Payload::try_from(serde_json::json!({ "tenant_id": tenant_id }))
            .map_err(|error| IndexError::Unavailable(error.to_string()))?;
        self.client
            .upsert_points(
                UpsertPointsBuilder::new(COLLECTION, vec![PointStruct::new(note_id, vector, payload)])
                    .wait(true),
            )
            .await
            .map(|_| ())
            .map_err(unavailable)
    }

    async fn similar(
        &self,
        tenant_id: &str,
        vector: &[f32],
        limit: u64,
    ) -> Result<Vec<NoteMatch>, IndexError> {
        check_vector(vector)?;
        let response = self
            .client
            .query(
                QueryPointsBuilder::new(COLLECTION)
                    .query(vector.to_vec())
                    .filter(tenant_filter(tenant_id))
                    .limit(limit.min(MAX_MATCHES))
                    .with_payload(false),
            )
            .await
            .map_err(unavailable)?;
        Ok(response
            .result
            .into_iter()
            .filter_map(|point| match point.id?.point_id_options? {
                PointIdOptions::Num(note_id) => Some(NoteMatch { note_id, score: point.score }),
                PointIdOptions::Uuid(_) => None,
            })
            .collect())
    }

    async fn remove(&self, tenant_id: &str, note_id: u64) -> Result<(), IndexError> {
        // The tenant condition keeps a guessed identifier from deleting another
        // tenant's point.
        let filter = Filter::must([
            Condition::matches("tenant_id", tenant_id.to_owned()),
            Condition::has_id([note_id]),
        ]);
        self.client
            .delete_points(DeletePointsBuilder::new(COLLECTION).points(filter).wait(true))
            .await
            .map(|_| ())
            .map_err(unavailable)
    }
}
```

`Qdrant` holds one gRPC channel: create it once and share it (here behind an
`Arc`). Creating a client per request opens a new connection every time.

## Start-up

Client code. The application chooses its index once, from the settings above.
An enum keeps static dispatch while either index can serve:

```rust,ignore
/// The index the application runs with, chosen at start-up.
#[derive(Clone)]
pub enum AppNoteIndex {
    Qdrant(QdrantNoteIndex),
    Memory(MemoryNoteIndex),
}

impl NoteIndex for AppNoteIndex {
    async fn upsert(&self, tenant_id: &str, note_id: u64, vector: Vec<f32>) -> Result<(), IndexError> {
        match self {
            Self::Qdrant(index) => index.upsert(tenant_id, note_id, vector).await,
            Self::Memory(index) => index.upsert(tenant_id, note_id, vector).await,
        }
    }

    async fn similar(&self, tenant_id: &str, vector: &[f32], limit: u64) -> Result<Vec<NoteMatch>, IndexError> {
        match self {
            Self::Qdrant(index) => index.similar(tenant_id, vector, limit).await,
            Self::Memory(index) => index.similar(tenant_id, vector, limit).await,
        }
    }

    async fn remove(&self, tenant_id: &str, note_id: u64) -> Result<(), IndexError> {
        match self {
            Self::Qdrant(index) => index.remove(tenant_id, note_id).await,
            Self::Memory(index) => index.remove(tenant_id, note_id).await,
        }
    }
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let index = match vector_settings()? {
        VectorSettings::Offline => AppNoteIndex::Memory(MemoryNoteIndex::default()),
        VectorSettings::Qdrant { url, api_key } => {
            let index = QdrantNoteIndex::connect(&url, api_key)?;
            index.ensure_collection().await?;
            AppNoteIndex::Qdrant(index)
        }
    };
    let app = rullst::Router::new().merge_axum(note_search_routes(index).into_axum());
    rullst::Server::new(app).run(3000).await?;
    Ok(())
}
```

A production deployment should refuse to start with the in-memory index: make
`Offline` an error there (for example when `RULLST_ENV` is `production`), so a
missing secret is not silently replaced by an empty, process-local index.

## Tenant isolation and query bounds

- **Filter every query by tenant.** `similar` and `remove` add the tenant
  condition themselves; never let a request choose the filter. Qdrant's
  payload index on `tenant_id` keeps the filtered search fast. When tenants
  need hard isolation (separate keys, backups or deletion), give each one its
  own collection instead and derive the collection name from the authenticated
  tenant, never from input.
- **Check vectors before sending them.** `check_vector` rejects a wrong size
  or a NaN before the request leaves the application.
- **Bound every search.** Cap `limit` on the server side, as `MAX_MATCHES`
  does, and keep the client's request timeout. Return identifiers and scores;
  load the notes from the database with the same ownership check as any other
  read, so the index is never the authority for what a user may see.

## Tests against a real server

Unit tests use `MemoryNoteIndex`, as in the doctest above. Test
`QdrantNoteIndex` itself against a real server, opt-in so `cargo test` stays
offline by default. `testcontainers-modules` has no Qdrant module, so start
the official image with `GenericImage`:

```toml
[dev-dependencies]
testcontainers = { version = "0.27.3", default-features = false, features = ["aws-lc-rs"] }
```

Client code, in a `#[cfg(test)]` module next to `QdrantNoteIndex`:

```rust,ignore
#[cfg(test)]
mod qdrant_tests {
    use super::*;
    use testcontainers::GenericImage;
    use testcontainers::core::{IntoContainerPort, WaitFor};
    use testcontainers::runners::AsyncRunner;

    fn axis(index: usize) -> Vec<f32> {
        let mut vector = vec![0.0; DIMENSIONS];
        vector[index] = 1.0;
        vector
    }

    #[tokio::test]
    async fn qdrant_index_is_tenant_scoped_and_bounded() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var_os("RUN_QDRANT_TESTS").is_none() {
            eprintln!("skipped: set RUN_QDRANT_TESTS=1 with Docker running");
            return Ok(());
        }
        // Pin the server version you deploy.
        let container = GenericImage::new("qdrant/qdrant", "v1.19.0")
            .with_wait_for(WaitFor::message_on_stdout("Qdrant gRPC listening on 6334"))
            .with_exposed_port(6334.tcp())
            .start()
            .await?;
        let port = container.get_host_port_ipv4(6334).await?;
        let index = QdrantNoteIndex::connect(&format!("http://127.0.0.1:{port}"), None)?;
        index.ensure_collection().await?;
        index.ensure_collection().await?; // idempotent

        index.upsert("tenant-a", 1, axis(0)).await?;
        index.upsert("tenant-a", 2, axis(1)).await?;
        index.upsert("tenant-b", 3, axis(0)).await?;
        let found = index.similar("tenant-a", &axis(0), 10).await?;
        assert_eq!(found.iter().map(|m| m.note_id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(index.similar("tenant-a", &axis(0), 1).await?.len(), 1);

        index.remove("tenant-b", 1).await?; // another tenant's id: no effect
        assert_eq!(index.similar("tenant-a", &axis(0), 10).await?.len(), 2);
        index.remove("tenant-a", 1).await?;
        assert_eq!(index.similar("tenant-a", &axis(0), 10).await?.len(), 1);
        Ok(())
    }
}
```

Run the same assertions against `MemoryNoteIndex` too, so the fake keeps the
real index's behaviour (the tenant filter, the ordering, the limit).

Without Testcontainers, start a disposable server bound to loopback, with a
memory limit, and have the test read its URL from an environment variable of
your own instead of starting a container:

```text
docker run --rm -d --name app-qdrant-test --memory 512m \
  -p 127.0.0.1:6334:6334 qdrant/qdrant:v1.19.0
RUN_QDRANT_TESTS=1 cargo test
docker rm -f app-qdrant-test
```

## Security

- Give every server outside a disposable test an API key
  (`QDRANT__SERVICE__API_KEY` on the server, `QDRANT_API_KEY` in the
  application) and TLS; a Qdrant server without a key accepts any request
  that reaches its port.
- Bind a development server to loopback (`-p 127.0.0.1:6334:6334`), never to
  every interface.
- Prefer a read-only key (`QDRANT__SERVICE__READ_ONLY_API_KEY`) for processes
  that only search.

## Migrating from `QdrantStore`

Up to 13.0.0-alpha.4, `rullst-orm`'s `qdrant` feature (`orm-qdrant` on
`rullst`) provided `QdrantStore`, `QdrantConfig`, the `VectorRepository` trait
and the `VectorCollectionName`, `VectorDimensions`, `VectorPoint`,
`VectorMatch` and `VectorQueryLimit` types over Qdrant's REST API.
13.0.0-alpha.5 removed the feature and the types. To move:

1. Remove `orm-qdrant` from the `rullst` features (or `qdrant` from the
   `rullst-orm` features) and add the `qdrant-client` dependency shown above.
   `cargo rullst upgrade` reports both the feature and the code that names
   the removed types.
2. Replace `QdrantStore::connect_or_mock(QdrantConfig::new(url, key))` with
   the start-up code above: `QdrantNoteIndex::connect` for a real server, and
   `MemoryNoteIndex` where the adapter selected its offline store (an empty or
   `mock_*` URL or key). The adapter used the REST port (6333); the official
   client uses gRPC (6334), so change the port in `QDRANT_URL`.
3. Replace `create_collection`, `upsert`, `delete` and `search` with the
   operations the application needs, as `NoteIndex` does, and add the tenant
   filter the adapter left to the caller.

Existing collections need no migration: the adapter created cosine
collections with one unnamed dense vector and stored `u64` point identifiers
with the payload you gave it, which the client above reads unchanged. The
adapter's source, which you may copy under its MIT licence, is
[`rullst-orm/src/polyglot/qdrant` at v13.0.0-alpha.4](https://github.com/Rullst/Rullst/tree/v13.0.0-alpha.4/rullst-orm/src/polyglot/qdrant).
