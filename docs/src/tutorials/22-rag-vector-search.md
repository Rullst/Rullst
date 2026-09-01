# 22. RAG Systems & Vector Search

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

Rullst provides three deliberately separate vector paths: a deterministic
in-memory index in `rullst-ai`, parameterized PostgreSQL `pgvector` queries,
and a bounded Qdrant HTTP adapter in `rullst-orm`. None silently invent tenant authorization, context
budgets, an embedding model, or a production RAG policy.

## In-memory retrieval

`VectorIndex` is useful for bounded local datasets and tests:

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", default-features = false, features = ["ai"] }
serde_json = "1.0"
```

```rust
use rullst::ai::VectorIndex;

let mut index = VectorIndex::new();
index.add(
    "rullst",
    vec![1.0, 0.0, 0.0],
    serde_json::json!({"text": "Rullst is a Rust web framework."}),
);
index.add(
    "other",
    vec![0.0, 1.0, 0.0],
    serde_json::json!({"text": "An unrelated document."}),
);

let matches = index.search(&[0.9, 0.1, 0.0], 3);
assert_eq!(matches[0].1.id, "rullst");
```

The caller supplies the embedding and the limit. This process-local index is
not durable, distributed, tenant-aware, or an approximate-nearest-neighbor
service.

## PostgreSQL + pgvector

Enable the typed vector and concrete PostgreSQL paths:

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", default-features = false, features = [
  "orm-pgvector",
  "strict-postgres",
  "ai",
] }
```

Install the extension through a reviewed PostgreSQL migration and choose the
dimension used by your embedding provider:

```sql
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE knowledge_chunks (
    id SERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536) NOT NULL
);
```

The model can use the re-exported typed `Vector`:

```rust,no_run
use rullst::orm::{FromRow, Orm, Vector};

#[derive(Clone, Debug, FromRow, Orm)]
#[orm(table = "knowledge_chunks", tenant_column = "tenant_id")]
struct KnowledgeChunk {
    id: i32,
    tenant_id: String,
    content: String,
    embedding: Vector,
}
```

Generate the query embedding through a configured guarded AI client, establish
the authenticated tenant scope, and then use the native operators:

```rust,no_run
use rullst::ai::AiClient;
use rullst::orm::{FromRow, Orm, Vector, with_tenant};

# #[derive(Clone, Debug, FromRow, Orm)]
# #[orm(table = "knowledge_chunks", tenant_column = "tenant_id")]
# struct KnowledgeChunk {
#     id: i32,
#     tenant_id: String,
#     content: String,
#     embedding: Vector,
# }

# async fn retrieve(
#     client: &AiClient,
#     question: &str,
# ) -> Result<Vec<KnowledgeChunk>, Box<dyn std::error::Error>> {
let embedding = client.embed(question).await?;
let query: Vec<f64> = embedding.iter().map(|value| f64::from(*value)).collect();

let chunks = with_tenant("tenant-42", async move {
    KnowledgeChunk::query()
        .where_similar("embedding", query.clone(), 0.8)
        .order_by_cosine_distance("embedding", query)
        .limit(5)
        .get()
        .await
})
.await?;
# Ok(chunks)
# }
```

Vector and distance values are SQL bindings, not interpolated literals. Column
names use the normal identifier validation, vectors must be finite/non-empty,
and the distance must be finite and non-negative. The `pgvector` feature also
supplies SQLx encode/decode for the typed field. Rullst's live contract creates
the extension, inserts typed vectors, and executes L2 and cosine queries against
a digest-pinned `pgvector/pgvector` container.

## Qdrant dense-vector store

Use Qdrant when the application deliberately chooses a specialized external
vector service rather than keeping vectors in PostgreSQL:

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", default-features = false, features = [
  "orm-qdrant",
  "ai",
] }
serde_json = "1.0"
```

```rust,no_run
use rullst::orm::{
    QdrantConfig, QdrantStore, VectorCollectionName, VectorDimensions,
    VectorPoint, VectorQueryLimit, VectorRepository,
};
use serde_json::{Map, Value};

# async fn qdrant_example() -> Result<(), Box<dyn std::error::Error>> {
let config = QdrantConfig::new(
    std::env::var("QDRANT_URL").unwrap_or_default(),
    std::env::var("QDRANT_API_KEY").unwrap_or_default(),
);
let vectors = QdrantStore::connect_or_mock(config)?;
let collection = VectorCollectionName::new("knowledge-v1")?;
vectors
    .create_collection(&collection, VectorDimensions::new(3)?)
    .await?;

let mut payload = Map::new();
payload.insert("chunk_id".into(), Value::String("chunk-42".into()));
vectors
    .upsert(
        &collection,
        VectorPoint::new(42, vec![1.0, 0.0, 0.0], payload)?,
    )
    .await?;
let matches = vectors
    .search(
        &collection,
        &[0.9, 0.1, 0.0],
        VectorQueryLimit::new(5)?,
    )
    .await?;
# let _ = matches;
# Ok(())
# }
```

Empty or `mock_*` endpoint/API-key values select the deterministic in-process
backend. An explicit `QdrantConfig::unauthenticated_local` is available only
for loopback self-hosting. The live contract is deliberately limited to one
unnamed dense cosine vector per numeric point. It bounds identifiers,
dimensions, finite/non-zero-norm vectors, 1 MiB object payloads, `top-k`, request
and response memory, deadlines and redirects. It does not claim named, sparse
or multivectors, arbitrary filters, hosted availability, ANN index tuning or
tenant authorization. A digest-pinned Qdrant lifecycle proves the supported
operations.

## Orchestrate one bounded RAG operation

The compatibility `build_rag_prompt` helper only formats already-authorized
text. Prefer `RagPipeline` when the application needs one typed operation for
embedding, retrieval, context budgets, guarded generation, source metadata, and
mandatory auditing:

```rust,no_run
use rullst::ai::rag::{RagPipeline, RagRetriever};

# fn compose<R, A>(client: rullst::ai::AiClient, retriever: R, audit: A)
# where
#     R: RagRetriever,
#     A: rullst::ai::rag::RagAuditSink,
# {
let pipeline = RagPipeline::new(client, retriever, audit);
# let _ = pipeline;
# }
```

The retriever receives a trusted tenant context and must enforce authoritative
tenant and ownership predicates in its datastore query. The pipeline also
rejects differently tagged documents and refuses ungrounded generation when no
safe context remains. Follow the complete
[Tenant-Bound RAG tutorial](41-tenant-bound-rag.md) for the offline index,
production adapter boundary, and secret-minimized audit contract.

The application still owns embedding dimension/model compatibility, durable
ingestion and deletion, citation evaluation, index tuning, authorization,
output policy, observability, and recovery.
