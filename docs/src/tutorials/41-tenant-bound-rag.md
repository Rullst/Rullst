# 41. Tenant-Bound RAG in One Typed Operation

Rullst's bounded RAG pipeline turns one authenticated question into a guarded
embedding, authorized retrieval request, budgeted context, grounded model call,
source metadata, and a terminal audit event.

It deliberately does not hide the datastore or invent authorization. The host
application still decides who may ask, which records that identity may read,
how documents are ingested and deleted, and which embedding model and vector
dimensions define an index.

## Add the feature

```toml
[dependencies]
rullst = { version = "12.0.0", default-features = false, features = ["ai"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Run the complete offline contract

An empty or `mock_*` OpenAI key selects Rullst's deterministic offline provider.
Its fixture embeddings have 16 dimensions, so the local index below uses the
same exact dimension.

```rust,no_run
use rullst::ai::rag::{
    InMemoryRagAuditTrail, InMemoryRagRetriever, RagDocument, RagPipeline,
};
use rullst::ai::{providers::openai::OpenAiProvider, AiClient};
use rullst::security::TenantContext;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build this only from verified authentication and membership claims.
    let tenant = TenantContext::try_new("tenant:acme")?;
    let client = AiClient::new(OpenAiProvider::new("mock_rag"));
    let retriever = InMemoryRagRetriever::try_new(1_000, 16)?;

    let content = "Rullst upgrades start with cargo rullst upgrade --dry-run.";
    let vector = client.embed(content).await?;
    let document = RagDocument::try_new(
        &tenant,
        "upgrade-guide",
        content,
        0.0, // The retriever replaces this with cosine similarity.
    )?;
    retriever.upsert(&tenant, document, vector)?;

    let audit = Arc::new(InMemoryRagAuditTrail::new(1_000)?);
    let pipeline = RagPipeline::new(client, retriever, Arc::clone(&audit));
    let response = pipeline
        .answer(&tenant, "How do I preview a framework upgrade?")
        .await?;

    println!("{}", response.answer());
    for source in response.sources() {
        println!(
            "source={} score={} chars={} truncated={}",
            source.document_id(),
            source.score(),
            source.included_chars(),
            source.truncated()
        );
    }

    let events = audit.entries()?;
    println!("recorded {} terminal RAG event(s)", events.len());
    Ok(())
}
```

The offline provider is a deterministic test fixture, not evidence of live
model quality. In a real application, select a live provider explicitly and
evaluate its configured model against a versioned domain corpus.

## What the pipeline enforces

For each call, `RagPipeline`:

1. validates the bounded question and runs guarded embedding;
2. passes the trusted `TenantContext`, query vector, and bounded limit to the
   application retriever;
3. rejects over-returned or differently tagged documents;
4. runs prompt-injection checks and outbound PII masking on every passage;
5. truncates by per-document and total Unicode-scalar budgets;
6. refuses to generate when no safe context remains;
7. returns only source identifiers, scores, included character counts, and
   truncation state alongside the answer;
8. records exactly one terminal, secret-minimized audit event, or fails the
   operation if the required audit sink is unavailable.

Tune budgets with `RagConfig::try_new(max_documents,
max_document_chars, max_context_chars)`. The constructor rejects zero or
unbounded values; `RagPipeline::with_config` accepts only an already validated
configuration.

## Use an authoritative production retriever

`InMemoryRagRetriever` is bounded and tenant-partitioned, but it is process
local, nondurable, and uses an exact linear cosine scan. It is appropriate for
tests, development, and small ephemeral datasets.

Production applications implement the public async `RagRetriever` trait over
their chosen store. A PostgreSQL implementation should put the tenant predicate
inside the same parameterized pgvector query that performs similarity ordering.
A Qdrant implementation should apply the deployment's reviewed tenant filter
or separate-collection policy. It then creates each `RagDocument` from the same
trusted context passed to `retrieve`.

The essential shape is:

```rust,ignore
async fn retrieve(
    &self,
    tenant: &TenantContext,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<RagDocument>, RagRetrievalError> {
    let rows = self.store
        .similar_chunks_for_tenant(&tenant.tenant_id, query_embedding, limit)
        .await
        .map_err(redacted_retrieval_error)?;

    rows.into_iter()
        .map(|row| RagDocument::try_new(tenant, row.id, row.content, row.score))
        .collect()
}
```

The pipeline verifies the returned tenant tag as defense in depth. That check
cannot detect a datastore adapter that incorrectly read another tenant's row and
then falsely relabeled it. Authoritative tenant and ownership filtering must
therefore happen in the datastore query itself.

## Supply durable, minimized audit evidence

`InMemoryRagAuditTrail` is useful for local assertions. Multi-instance
production deployments should implement `RagAuditSink` over an append-only,
access-controlled destination with an explicit retention policy.

The built-in event contains the tenant ID, a SHA-256 digest of the original
question, document counts, included context characters, and the terminal
outcome. It omits raw questions, passages, vectors, model answers, and provider
error bodies. A digest is not encryption and low-entropy questions may be
guessable, so do not expose the audit stream as public data.

## Security and quality boundaries

- Prompt-injection filtering is heuristic defense in depth, not proof that a
  passage or answer is safe.
- Source metadata proves which identifiers entered the prompt; it does not prove
  that the model cited them faithfully or that their contents were true.
- Apply normal output encoding, domain validation, and tool authorization after
  generation. Never treat model text as an authorized command.
- Keep ingestion, updates, deletions, retention, embedding-model migrations,
  ANN tuning, backups, and disaster recovery explicit.
- Test cross-tenant denial, empty retrieval, hostile passages, provider failure,
  datastore failure, audit failure, and context truncation in the host
  application.

For the lower-level vector-store contracts, see
[RAG Systems & Vector Search](22-rag-vector-search.md). For the framework's
broader claims and limits, see the [capability ledger](../capability-ledger.md).
