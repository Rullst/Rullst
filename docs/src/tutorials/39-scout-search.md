# 39. Scout Search Providers

Scout connects a `#[orm(searchable)]` SQLx model to one search backend. The
model database remains authoritative; the search index is a projection updated
only after a successful managed commit.

## Enable the provider adapters

With the umbrella crate:

```toml
[dependencies]
rullst = { version = "12.0.0", features = ["orm-scout"] }
```

Or enable `scout-http` directly on `rullst-orm`. The feature supplies bounded
HTTP adapters for Meilisearch, Elasticsearch and Algolia. The in-memory
`MockSearchEngine` is available without the HTTP feature.

## Configure one engine

Configuration is process-wide and fail-closed if code tries to replace an
already installed engine:

```rust,no_run
use rullst_orm::{MeilisearchEngine, set_search_engine};

# fn configure() -> Result<(), rullst_orm::Error> {
let engine = MeilisearchEngine::new(
    std::env::var("MEILI_URL").unwrap_or_default(),
    std::env::var("MEILI_API_KEY").unwrap_or_default(),
)?;
set_search_engine(engine)?;
# Ok(())
# }
```

Empty or `mock_*` credentials deliberately select the deterministic offline
store. That makes local/test behavior explicit and prevents an accidental live
call with missing secrets. A keyless Meilisearch or Elasticsearch development
server must be selected with `MeilisearchEngine::local(...)` or
`ElasticsearchEngine::local(...)`; those constructors accept only loopback
HTTP. Normal custom endpoints require HTTPS, contain no URL credentials/path,
and disable redirects.

Algolia normally derives its official API origin from the application ID:

```rust,no_run
use rullst_orm::AlgoliaEngine;

# fn engine() -> Result<AlgoliaEngine, rullst_orm::Error> {
AlgoliaEngine::new("APPLICATION_ID", "ADMIN_API_KEY")
# }
```

`AlgoliaEngine::with_endpoint` exists for an explicitly configured compatible
proxy and applies the same HTTPS/loopback URL policy. Never expose an indexing
or admin key to browser code.

## Mark and query the model

```rust,no_run
use rullst_orm::{FromRow, Orm};

#[derive(Clone, Debug, FromRow, Orm)]
#[orm(table = "articles", searchable)]
struct Article {
    id: i32,
    title: String,
    body: String,
}

# async fn find() -> Result<Vec<Article>, rullst_orm::Error> {
let results = Article::search("transactional outbox").await.get().await?;
# Ok(results)
# }
```

Generated save/update/delete operations project only after the relational
commit. Rollback produces no search write. Provider or search errors remain
typed errors; they are not silently converted into an empty result.

The shared adapter boundary enforces:

- lowercase index names of at most 128 bytes;
- positive `i32` document IDs;
- object-shaped JSON documents no larger than one MiB;
- queries no larger than 1,024 bytes and without control characters;
- at most 1,000 parsed hits and a four-MiB response body;
- five-second connect and twenty-second request deadlines;
- disabled redirects and no secret-bearing error bodies.

Meilisearch and Algolia asynchronous indexing tasks are awaited with a bounded
poll loop. Elasticsearch uses `refresh=wait_for` for the adapter operations.

## Choose the durability level

The generated Scout hook is process-local after commit. A provider outage is
reported as `PostCommit`: the model row is already durable and must not be
blindly inserted again. This path is suitable when the application can rebuild
the index or explicitly retry.

If every projection must survive a process crash, write a versioned search
event through `Outbox::enqueue` in the domain transaction and run the selected
engine from an idempotent outbox worker. Rullst does not invent a stable event
key or serialize every model hook automatically. See
[Transactional Outbox & Durable Effects](38-transactional-outbox.md).

## Evidence boundaries

The repository runs a real update/search/delete lifecycle against a
digest-pinned Meilisearch container. Elasticsearch and Algolia run deterministic
offline tests plus local HTTP protocol fixtures that verify paths, headers,
payloads, response bounds and ID parsing. Those fixtures do not prove a hosted
Algolia account, every Elasticsearch version, cluster failover, ranking quality
or production capacity.
