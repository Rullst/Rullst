# Rullst AI

> **Vision preserved:** capability-typed schema support, local-model boundaries,
> and autonomous-agent ideas remain itemized with an implementation opinion in the
> [capability ledger](../capability-ledger.md#ai-and-mail).

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

Unsupported capabilities return `AiError::UnsupportedCapability`; the client does not silently
switch to an unrelated endpoint or represent a fixture as a live-provider result.

`OpenAiCompatibleProvider` covers servers implementing the named OpenAI
`/chat/completions` and optional `/embeddings` shapes. It defaults to chat-only;
vision, embeddings, JSON mode, and JSON Schema must be declared for the exact
endpoint/model pair. `try_local` permits unauthenticated HTTP only on a literal
loopback IP, `try_local_with_bearer` adds explicit local authentication, and
`try_cloud` requires HTTPS plus a Bearer credential. All three
disable redirects and environment proxies and bound response bodies. Different
protocols use a custom public `AiProvider`, not an arbitrary-HTTP mode.

## Guarded client

```rust,no_run
use rullst_ai::{AiClient, AiError, providers::openai::OpenAiProvider};

async fn answer(api_key: String, user_text: &str) -> Result<String, AiError> {
    let client = AiClient::new(OpenAiProvider::new(api_key));
    client
        .chat()
        .system("Answer concisely.")
        .user(user_text)
        .send()
        .await
}
```

`AiClient::prompt`, chat, vision, embedding, JSON, and structured-output calls run the same
guardrail stage before provider dispatch. Built-in providers repeat the check on direct trait calls.
Call custom `AiProvider` implementations through `AiClient` when the application needs the same
mandatory boundary.

The current guardrail blocks deterministic injection patterns, provider delimiter tokens, external
Markdown beacons, and selected invisible Unicode controls. Supported PII classes are masked before
outbound transmission. This is a bounded heuristic control, not proof that arbitrary input or model
output is safe; authorization, tool permissions, output encoding, and domain validation remain
application responsibilities.

## Tenant-aware chat memory

`StatefulChat<M>` is a static-dispatch orchestration boundary over
`ChatMemory`. It binds every conversation to trusted `TenantContext`, loads a
bounded even history, calls the guarded client, and atomically appends the user
and assistant halves after successful generation. `InMemoryChatMemory` is a
bounded deterministic offline store.

With the opt-in umbrella `ai-sql-memory` feature, `SqlChatMemory` supplies a
dedicated SQLx Any pool and fixed schema for SQLite, PostgreSQL, MySQL, and
MariaDB. Its revision compare-and-swap rejects stale cross-process writers. It
does not retry the provider call, because doing so could duplicate cost or side
effects. The [AI integration tutorial](../6-ai-integration-tutorial.md#8-durable-chat-memory)
shows the complete setup and application-owned security/retention boundary.

## Tenant-aware RAG pipeline

`RagPipeline::answer` performs guarded embedding, calls a static-dispatch `RagRetriever`, applies
per-document and total Unicode-safe budgets, guards and masks every selected passage, generates a
grounded response, returns source metadata, and records one terminal audit event. It requires a
trusted `TenantContext`, rejects differently tagged documents, and fails with `RagError::NoContext`
instead of asking the model to answer without retrieved evidence.

The bundled `InMemoryRagRetriever` is a bounded tenant-partitioned cosine index for offline tests,
development, and small ephemeral datasets. It is neither durable nor distributed. A production
application can implement `RagRetriever` over ORM pgvector or Qdrant, but that adapter must bind the
trusted tenant and ownership predicates in the authoritative datastore. The pipeline's tag check is
defense in depth, not a replacement for datastore authorization.

The mandatory audit event stores the tenant, a SHA-256 correlation digest, counts, character budget,
and outcome. It deliberately omits raw questions, documents, embeddings, provider bodies, and model
answers. The digest is not encryption and can be guessed for low-entropy questions. Use a durable
append-only `RagAuditSink` in production; the included sink is process-local.

Follow the [tenant-bound RAG tutorial](../tutorials/41-tenant-bound-rag.md) for the complete offline
flow and production integration boundary.

## Versioned offline evals

The packaged `evals/guardrails-v1.json` corpus freezes deterministic injection,
jailbreak, and PII regressions. The repository gate validates unique IDs and
required categories, then runs every case across all six built-in transports in
offline mode. It is deliberately not presented as a safety benchmark:
adaptive attacks, tool selection, hallucination, and live provider/model
versions require separate eval suites.

## Strict egress policy

`EgressPolicy::strict()` starts with no permitted destination. After an exact
host allowlist is configured, `EgressFetcher` permits HTTPS and explicit ports,
blocks credentials/local/private/metadata/reserved addresses, validates every
DNS answer, pins those answers in a proxy-free reqwest client, verifies the
connected peer, follows redirects only after repeating policy, and bounds time
and streamed bytes. The fetcher is opt-in: it cannot protect arbitrary
application/provider HTTP clients, and tenant authorization, content schema and
data minimization remain caller contracts.

## Offline mode

OpenAI, Gemini, Anthropic, DeepSeek, and compatible cloud endpoints use
deterministic offline mode when their API key is empty or begins with `mock_`.
Ollama uses an empty or `mock_*` host; the compatible adapter also exposes an
explicit `mock` constructor. Plain `try_local` is deliberately live because no
credential is its valid loopback configuration. Offline branches return before
HTTP dispatch and cover each capability the provider declares. Unsupported
capabilities remain typed errors in offline mode.

`AiClient::auto()` checks `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`,
`DEEPSEEK_API_KEY`, and `OLLAMA_HOST`. If none is configured, it selects an offline OpenAI fixture;
it does not probe localhost implicitly.

## JSON mode and structured output

JSON mode requests a parseable JSON value and deserializes it in Rust:

```rust,no_run
# use rullst_ai::{AiClient, AiError, providers::openai::OpenAiProvider};
# async fn example() -> Result<(), AiError> {
let client = AiClient::new(OpenAiProvider::new("mock_local"));
let value: serde_json::Value = client.json_prompt("Summarize this record").await?;
# let _ = value;
# Ok(())
# }
```

Native structured output requires an explicit schema and fails when the provider cannot enforce it:

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

Provider-side schema enforcement and Rust deserialization do not replace application-specific
semantic validation.

## Current boundaries

Streaming responses, provider-native tool execution loops, first-party external
vector-store `RagRetriever` adapters, and compile-time schema derivation remain roadmap work. The
SQL memory does not supply raw-text encryption, ownership within a tenant,
retention or provider auditing; the in-memory vector utilities and tool registry
do not create an authorization boundary by themselves.
