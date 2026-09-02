# Rullst AI

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io.

`rullst-ai` is a provider-agnostic LLM client with mandatory outbound prompt-injection checks,
PII masking, deterministic offline fixtures, JSON mode, explicit JSON Schema output, and a bounded
tenant-aware RAG pipeline.

## Provider capabilities

| Provider | Chat | Vision | Embeddings | JSON mode | Native JSON Schema |
|---|---:|---:|---:|---:|---:|
| OpenAI | yes | yes | yes | yes | yes |
| Gemini | yes | yes | yes | yes | yes |
| Anthropic | yes | yes | no | prompt-constrained | no |
| DeepSeek | yes | no | no | yes | `deepseek-v4-flash` |
| Ollama | yes | model-dependent | yes | yes | local API only |
| OpenAI-compatible local/cloud | yes | declared | declared | declared | declared |

Unsupported capabilities return `AiError::UnsupportedCapability`; they do not silently switch to
an unrelated endpoint or fabricate a live-provider result.

`OpenAiCompatibleProvider` covers servers implementing the named OpenAI
`/chat/completions` and optional `/embeddings` shapes. It defaults to chat-only;
vision, embeddings, JSON mode, and JSON Schema must be declared for the exact
endpoint/model pair. `try_local` permits unauthenticated HTTP only on a literal
loopback IP, `try_local_with_bearer` adds explicit local authentication, and
`try_cloud` requires HTTPS plus a Bearer credential. All three
disable redirects and environment proxies and bound response bodies. Different
protocols use a custom public `AiProvider`, not an arbitrary-HTTP mode.
Local runtimes such as llama.cpp server, LocalAI, LM Studio, and vLLM can use
this path only when their installed configuration exposes the declared shapes;
the product name alone is not treated as compatibility evidence.

## Guarded client

```rust,no_run
use rullst_ai::{AiClient, AiError, providers::openai::OpenAiProvider};

async fn answer(api_key: String, user_text: &str) -> Result<String, AiError> {
    let client = AiClient::new(OpenAiProvider::new(api_key));
    client.chat()
        .system("Answer concisely.")
        .user(user_text)
        .send()
        .await
}
```

`AiClient::prompt`, chat, vision, embedding, JSON, and structured-output calls all pass through the
same guardrail stage before provider dispatch. Built-in providers repeat that check on direct trait
calls. Custom `AiProvider` implementations should be called through `AiClient` in application code.

The guardrail blocks deterministic injection patterns and invisible Unicode controls. Supported PII
classes are masked before outbound transmission. Like all heuristic filters, this is one boundary in
a defense-in-depth design; it is not a proof that arbitrary model output is safe.

## Tenant-aware durable chat memory

`StatefulChat<M>` uses static dispatch over the `ChatMemory` contract. It loads
bounded recent history, performs the normal guarded provider call, then appends
the user and assistant messages atomically. Every conversation is selected by a
trusted `TenantContext` plus a validated `ConversationId`.

`InMemoryChatMemory` is a deterministic bounded offline implementation. The
opt-in `sql-memory` feature adds `SqlChatMemory` for SQLite, PostgreSQL, MySQL,
and MariaDB. Its fixed schema uses a monotonically increasing even revision;
the update and both message inserts share one transaction. A stale concurrent
writer receives `ChatMemoryError::RevisionConflict` instead of silently
reordering history or automatically repeating a billable provider call.

```rust,no_run
# use rullst_ai::{AiClient, ChatMemoryConfig, ConversationId, SqlChatMemory, StatefulChat, providers::openai::OpenAiProvider};
# use rullst_core::security::TenantContext;
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let memory = SqlChatMemory::connect(
    "sqlite://chat.db?mode=rwc",
    ChatMemoryConfig::default(),
).await?;
memory.prepare_schema().await?;
let chat = StatefulChat::new(
    AiClient::new(OpenAiProvider::new("mock_local")),
    memory,
);
let tenant = TenantContext::try_new("tenant-42")?;
let conversation = ConversationId::try_new("support:case-7")?;
chat.ensure_conversation(&tenant, &conversation).await?;
let turn = chat.send(&tenant, &conversation, "What changed?").await?;
# let _ = turn;
# Ok(())
# }
```

The SQL adapter stores message text as supplied by the application. Encryption,
retention, erasure policy, authenticated conversation ownership inside a
tenant, provider-call audit, backups, and conflict retry UX remain host
responsibilities. The generated `make:chat-session` scaffold remains useful
when the application wants to own or customize its models, migrations, or the
Turso-primary implementation.

## Tenant-aware RAG pipeline

`RagPipeline::answer` composes guarded embedding, application-provided retrieval, Unicode-safe
context budgets, guarded generation, source metadata, and mandatory secret-minimized auditing in one
typed operation. A trusted `TenantContext` is required and every returned document must carry the
same tenant tag. Empty retrieval fails with `RagError::NoContext` instead of generating an
ungrounded answer.

`InMemoryRagRetriever` supplies bounded tenant-partitioned cosine retrieval for tests, local
development, and small ephemeral datasets. It is not durable or distributed. Production
applications implement the static-dispatch `RagRetriever` boundary over a store such as Rullst ORM
pgvector or Qdrant and must enforce authoritative tenant/ownership predicates in that store. The
pipeline's tenant-tag check is an additional invariant, not datastore authorization.

See the [tenant-bound RAG tutorial](../docs/src/tutorials/41-tenant-bound-rag.md) for a complete
offline example, audit behavior, and the production adapter boundary.

## Versioned offline evals

`evals/guardrails-v1.json` is the machine-readable deterministic regression
corpus for the implemented injection, jailbreak, and PII behaviors. Run
`bash .github/check-ai-evals.sh` from the workspace root to validate the corpus
and exercise every built-in provider in explicit offline mode. This corpus is
not a safety benchmark; adaptive attacks, tool selection, hallucination, and
live model/version evaluations remain separate work.

## Strict egress policy

`EgressPolicy::strict()` is deny-by-default until configured with an exact host
allowlist. `EgressFetcher` accepts HTTPS only, blocks URL credentials and
local/private/metadata/reserved literal or DNS answers, pins every validated
answer into a proxy-free client, checks the connected peer, validates redirects
manually, and enforces request-time and streaming-byte budgets.

```rust,no_run
# use rullst_ai::{EgressFetcher, EgressPolicy};
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let policy = EgressPolicy::strict()
    .with_allowed_hosts(["docs.example.com"])?;
let resource = EgressFetcher::new(policy)
    .fetch_bytes("https://docs.example.com/guide.json")
    .await?;
# let _ = resource;
# Ok(())
# }
```

This fetcher is not automatically mounted around unrelated HTTP clients or
provider transports. Applications still own tenant authorization, destination
selection, content-type/schema validation and data minimization.

## Offline mode

OpenAI, Gemini, Anthropic, DeepSeek, and compatible cloud endpoints select
deterministic offline mode when their API key is empty or starts with `mock_`.
Ollama uses an empty or `mock_*` host; the compatible adapter also exposes an
explicit `mock` constructor. Plain `try_local` is deliberately live because no
credential is its valid loopback configuration. Offline branches return before
HTTP dispatch and cover every capability the provider declares. Unsupported
live capabilities remain typed errors in offline mode.

`AiClient::auto()` checks `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`,
`DEEPSEEK_API_KEY`, and `OLLAMA_HOST`. If none is configured, it selects an offline OpenAI fixture;
it never probes localhost implicitly.

## JSON mode versus structured output

JSON mode only guarantees that a response can be parsed as JSON:

```rust,no_run
# use rullst_ai::{AiClient, AiError, providers::openai::OpenAiProvider};
# async fn example() -> Result<(), AiError> {
let client = AiClient::new(OpenAiProvider::new("mock_local"));
let value: serde_json::Value = client.json_prompt("Summarize this record").await?;
# let _ = value;
# Ok(())
# }
```

Native structured output requires an explicit schema and fails when the selected provider cannot
enforce it:

```rust,no_run
# use rullst_ai::{AiClient, AiError, StructuredOutputSchema, providers::openai::OpenAiProvider};
# async fn example() -> Result<(), AiError> {
let client = AiClient::new(OpenAiProvider::new("mock_local"));
let schema = StructuredOutputSchema::new("answer", serde_json::json!({
    "type": "object",
    "properties": {"ok": {"type": "boolean"}},
    "required": ["ok"],
    "additionalProperties": false
}))?;
let value: serde_json::Value = client
    .structured_prompt_with_schema("Evaluate the input", &schema)
    .await?;
# let _ = value;
# Ok(())
# }
```

The schema is enforced by the provider API and the returned value is deserialized again in Rust.
Application-specific semantic validation is still required.
