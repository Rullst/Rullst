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

The native OpenAI, Gemini, Anthropic, DeepSeek and Ollama adapters also disable
HTTP redirects and ambient proxy settings. Their pooled HTTP transport has a
five-second connection deadline, the existing configurable whole-request
deadline (30 seconds by default), and a two-MiB JSON response ceiling enforced
while reading even when Content-Length is absent. Configure the final trusted
endpoint directly; redirected prompts, credentials and bodies are not replayed.
These are local transport invariants, not proof of provider availability,
prompt-injection immunity or upstream request cancellation.

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

## Bounded streaming and explicit cancellation

`StreamingAiClient<P>` is a separate static-dispatch extension so the portable
`AiProvider` trait remains object-safe. v12 implements genuine incremental
`text/event-stream` parsing for an exact OpenAI-compatible endpoint only after
that configuration calls `with_streaming()`. It requires the `[DONE]` sentinel,
accepts at most 4,096 non-empty chunks, 64 KiB per chunk and 2 MiB of raw SSE
and delivered UTF-8 output, and rejects malformed, truncated or incorrectly
typed responses.

```rust,no_run
# use rullst_ai::{AiCancellation, AiError, StreamingAiClient, providers::openai_compatible::{OpenAiCompatibleCapabilities, OpenAiCompatibleProvider}};
# async fn example() -> Result<(), AiError> {
let provider = OpenAiCompatibleProvider::try_local(
    "http://127.0.0.1:11434/v1",
    "local-model",
)?.with_capabilities(
    OpenAiCompatibleCapabilities::chat_only().with_streaming(),
);
let client = StreamingAiClient::new(provider);
let cancellation = AiCancellation::new();
let mut output = String::new();
let summary = client
    .stream_prompt("Answer concisely", &cancellation, &mut |chunk: &str| {
        output.push_str(chunk);
        Ok(())
    })
    .await?;
# let _ = (output, summary);
# Ok(())
# }
```

Cancelling the cloneable signal races the request and every body read, dropping
the local transport future. It does not prove that an upstream server stopped
generation or billing. Other built-in providers and ordinary non-streaming
`AiProvider` calls retain deadline/drop semantics until their exact streaming
protocols receive equivalent evidence.

## Policy-bound vision sources

Vision accepts three explicit source forms. `prompt_with_image` consumes bytes
already admitted by the application. `prompt_with_image_file` canonicalizes a
file and requires it to remain under an exact `LocalImagePolicy` root.
`prompt_with_image_url` requires an `EgressFetcher` whose exact HTTPS host
allowlist, DNS pinning, redirect checks, peer verification, timeout, and stream
budget apply before provider dispatch.

```rust,no_run
# use rullst_ai::{AiClient, EgressFetcher, EgressPolicy, LocalImagePolicy, providers::openai::OpenAiProvider};
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = AiClient::new(OpenAiProvider::new("mock_local"));

let local = LocalImagePolicy::with_max_bytes(["./private-uploads"], 2 * 1_024 * 1_024)?;
client
    .prompt_with_image_file("Describe this image", "./private-uploads/photo.png", &local)
    .await?;

let remote_policy = EgressPolicy::strict()
    .with_allowed_hosts(["images.example.com"])?
    .with_max_response_bytes(2 * 1_024 * 1_024)?;
let fetcher = EgressFetcher::new(remote_policy);
client
    .prompt_with_image_url(
        "Describe this image",
        "https://images.example.com/photo.png",
        &fetcher,
    )
    .await?;
# Ok(())
# }
```

The high-level file and URL paths accept at most 10 MiB and recognize bounded
JPEG, full PNG signature, RIFF/WebP, and GIF87a/GIF89a signatures. A supplied
remote `Content-Type` must match those bytes. Provider capability is checked
and text guardrails run before local or network I/O. Exact model support is
still provider/configuration dependent, and an allowlisted local directory
must be protected by the host against adversarial rename races.

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

`DurableRagAuditTrail` and `DurableToolAuditTrail` provide bounded synchronous
local audit files with distinct version headers, SHA-256 frame integrity,
restart validation and fail-closed quota/corruption handling. They are
single-process writers, not authenticated or distributed audit services; the
host owns directory permissions, rotation, retention and backup.

`AuditDeliveryClient` is the separate opt-in export boundary for minimized
RAG, tool or provider observations. A cloud endpoint must use HTTPS; a local
development endpoint must use a literal loopback IP. The client signs the exact
bounded JSON envelope with HMAC-SHA256, sends a key ID and timestamp, preserves
one caller-supplied event ID across bounded retries, and accepts only a small
closed JSON acknowledgement bound to that ID. Empty or `mock_*` signing keys
select deterministic offline behavior.

```rust,no_run
# use rullst_ai::{AiCancellation, AuditDeliveryClient, AuditDeliveryError};
# async fn example(signing_key: String) -> Result<(), AuditDeliveryError> {
let delivery = AuditDeliveryClient::try_cloud(
    "https://audit.example.com/v1/events",
    "academy-api",
    "audit-key-2026",
    signing_key,
)?;
let cancellation = AiCancellation::new();
let receipt = delivery
    .publish(
        "evt-0189f6b8",
        1_788_000_000_000,
        &serde_json::json!({"kind": "rag.completed", "outcome": "allowed"}),
        &cancellation,
    )
    .await?;
# let _ = receipt;
# Ok(())
# }
```

The receiver must independently verify freshness/signature, deduplicate the
event ID and operate its own authorization, persistence, retention and key
rotation. Rullst does not inspect an arbitrary event value for secrets, run a
durable delivery queue or claim SIEM availability; callers must export only
the minimized records their policy permits.

See the [tenant-bound RAG tutorial](../docs/src/tutorials/41-tenant-bound-rag.md) for a complete
offline example, audit behavior, and the production adapter boundary.

## Versioned and adaptive evaluations

`evals/guardrails-v1.json` is the machine-readable deterministic regression
corpus for the implemented injection, jailbreak, and PII behaviors. Run
`bash .github/check-ai-evals.sh` from the workspace root to validate the corpus
and exercise every built-in provider in explicit offline mode.

`AdaptiveAiEvaluator<P>` is the separate static-dispatch runner for
application-defined multi-turn scenarios. It reapplies the mandatory input
guardrail, caps turn count and prompt/response bytes, sets an independent
per-turn deadline, supports `AiCancellation`, and preserves only sizes and
low-cardinality outcomes in its versioned JSON report. A strategy may use the
bounded response from one turn to construct the next prompt, but the raw text
is never retained by the report.

```rust,no_run
# use rullst_ai::{AdaptiveAiEvaluator, AiCancellation, AiEvaluationDecision, AiEvaluationObservation, AiEvaluationStrategy, providers::openai::OpenAiProvider};
struct TwoTurnScenario;

impl AiEvaluationStrategy for TwoTurnScenario {
    fn initial_prompt(&mut self) -> String {
        "Provide one short policy-safe greeting".to_string()
    }

    fn observe(&mut self, observation: &AiEvaluationObservation<'_>) -> AiEvaluationDecision {
        if observation.turn() == 1 {
            let size = observation.response().map_or(0, str::len);
            AiEvaluationDecision::continue_with(format!(
                "Continue the evaluation after a {size}-byte response"
            ))
        } else if observation.response().is_some() {
            AiEvaluationDecision::pass("scenario_passed")
        } else {
            AiEvaluationDecision::inconclusive("provider_unavailable")
        }
    }
}

# async fn example(api_key: String) -> Result<(), Box<dyn std::error::Error>> {
let evaluator = AdaptiveAiEvaluator::new(
    OpenAiProvider::new(api_key).with_model("exact-reviewed-model"),
);
let mut scenario = TwoTurnScenario;
let report = evaluator
    .run(
        "application-safety-v1",
        "openai:exact-reviewed-model",
        &mut scenario,
        &AiCancellation::new(),
    )
    .await?;
# let _ = report;
# Ok(())
# }
```

The strategy executes synchronously and temporarily sees model text; it must
not log or persist that text without an application policy. The `subject` is a
caller assertion, not model discovery. The built-in offline regression proves
runner behavior only: every provider/model/version, adaptive corpus and live
result must still be evaluated and reviewed by its operator. A passed report is
not a universal safety, hallucination or jailbreak-resistance claim.

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
