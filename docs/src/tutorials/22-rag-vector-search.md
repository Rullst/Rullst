# 22. RAG Systems & Vector Search

> [!IMPORTANT]
> Dependency examples pin the v13 prerelease `=13.0.0-alpha.4` exactly: Cargo
> never selects a prerelease on its own, and the v12 line reached end of life
> on 16 October 2026 (see the [release record](../v12.md)). Commit Cargo.lock
> for reproducible builds. Use path dependencies only when intentionally
> testing checkout-local changes.

Rullst provides two deliberately separate vector paths: a deterministic
in-memory index in `rullst-ai` and parameterized PostgreSQL `pgvector`
queries; a dedicated vector service such as Qdrant is application code (see
below). None silently invent tenant authorization, context
budgets, an embedding model, or a production RAG policy.

## In-memory retrieval

`VectorIndex` is useful for bounded local datasets and tests:

```toml
[dependencies]
rullst = { version = "=13.0.0-alpha.4", default-features = false, features = ["ai"] }
serde_json = "1.0"
```

```rust
use rullst::ai::VectorIndex;

let mut index = VectorIndex::new();
index.add(
    "rullst",
    vec![1.0, 0.0, 0.0],
    serde_json::json!({"text": "Rullst is a Rust framework suite."}),
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
rullst = { version = "=13.0.0-alpha.4", default-features = false, features = [
  "orm-pgvector",
  "strict-postgres",
  "ai",
] }
# `#[derive(Orm)]`, the model's `FromRow` included, expands to `rullst_orm::…`
# paths, so a crate that derives models depends on it directly.
rullst-orm = { version = "=13.0.0-alpha.4", default-features = false, features = ["strict-postgres"] }
```

Projects generated with `cargo rullst new --database postgres` already list
the `rullst-orm` line; add `orm-pgvector` and `ai` to their `rullst`
features.

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
use rullst::orm::{Orm, Vector};

#[derive(Clone, Debug, Orm)]
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
use rullst::orm::{Orm, Vector, with_tenant};

# #[derive(Clone, Debug, Orm)]
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

The two clauses use different metrics: `where_similar` keeps rows whose L2
distance (`<->`) is below the bound, while `order_by_cosine_distance` sorts by
cosine distance (`<=>`). `order_by_l2_distance` and `order_by_inner_product`
(`<#>`) are also available; choose the bound for the L2 metric.

Vector and distance values are SQL bindings, not interpolated literals. Column
names use the normal identifier validation, vectors must be finite/non-empty,
and the distance must be finite and non-negative. The `pgvector` feature also
supplies SQLx encode/decode for the typed field. Rullst's live contract creates
the extension, inserts typed vectors, and executes L2 and cosine queries against
a digest-pinned `pgvector/pgvector` container.

## Qdrant dense-vector store

Use Qdrant when the application deliberately chooses a specialized external
vector service rather than keeping vectors in PostgreSQL. Since
13.0.0-alpha.5 Rullst has no Qdrant adapter: the
[Qdrant recipe](../recipes/qdrant.md) uses the official `qdrant-client` crate
in the application, with one client created at start-up, an in-memory fake
for tests, every search filtered by tenant and capped, and tests against a
real server. Its `NoteIndex` trait is also a natural place to implement the
`RagRetriever` below.

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
