# Integrating AI into Rullst

`rullst-ai` provides guarded adapters for OpenAI, Anthropic, Gemini, DeepSeek,
and Ollama. The high-level `AiClient` applies prompt-injection heuristics and PII
masking before dispatch. Those controls reduce known risks; passing them does
not prove that a prompt or model response is safe or correct.

Start with the [provider capability matrix](ai-provider-capabilities.md). It
separates implemented transport paths from model-dependent behavior and lists
unsupported streaming, tool, timeout, retry, and cancellation boundaries.

## 1. Enable the AI facade

```toml
[dependencies]
rullst = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["ai"]
}
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Use the exact published v12 version being evaluated. The RC value above is a
planned prerelease and must not be used before it exists on crates.io.

## 2. Create a guarded client

```rust
use rullst::ai::{AiClient, providers::openai::OpenAiProvider};

fn ai_client() -> Result<AiClient, std::env::VarError> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    Ok(AiClient::new(OpenAiProvider::new(api_key)))
}
```

Empty and `mock_*` keys intentionally select deterministic offline behavior.
Requiring the environment variable, as above, prevents a live deployment from
silently becoming a demo. Tests can construct `OpenAiProvider::new("")`
explicitly when offline behavior is desired.

Other built-in constructors are available under:

- `providers::anthropic::AnthropicProvider`;
- `providers::gemini::GeminiProvider`;
- `providers::deepseek::DeepSeekProvider`;
- `providers::ollama::OllamaProvider`.

## 3. Call it from an Axum handler

This bounded handler rejects oversized input before invoking the client and does
not expose the provider's full error details to the HTTP caller:

```rust
use rullst::{Server, ai::AiClient};
use rullst::web::axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::post,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ChatPrompt {
    prompt: String,
}

#[derive(Serialize)]
struct ChatResponse {
    answer: String,
}

async fn chat(
    State(client): State<AiClient>,
    Json(body): Json<ChatPrompt>,
) -> Result<Json<ChatResponse>, (StatusCode, &'static str)> {
    if body.prompt.len() > 8_192 {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "prompt is too large"));
    }

    let answer = client
        .prompt(&body.prompt)
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "AI request failed"))?;
    Ok(Json(ChatResponse { answer }))
}

async fn serve(client: AiClient) -> Result<(), rullst::ServerError> {
    let app = Router::new()
        .route("/api/chat", post(chat))
        .with_state(client);
    Server::new(app.into()).run(3000).await
}
```

Production applications should authenticate and rate-limit this route, bind
tenant identity from authenticated state, cap response sizes, and record an
audit event without logging prompts or secrets verbatim.

## 4. Inspect capabilities before optional operations

```rust
let capabilities = client.capabilities();

if capabilities.vision {
    let response = client.prompt_with_image("Describe this image", bytes).await?;
    // Use the response according to the application's trust policy.
}
```

Capability inspection prevents avoidable requests but is not a substitute for
handling `UnsupportedCapability` and upstream model errors. Configuration can
select a model that supports less than its provider transport.

## 5. Request structured output

`json_prompt` requests parseable JSON. It does not claim JSON Schema
enforcement. Use `structured_prompt_with_schema` only when the reported provider
capability and configured model support it:

```rust
use rullst::ai::StructuredOutputSchema;

let schema = StructuredOutputSchema::new(
    "answer",
    serde_json::json!({
        "type": "object",
        "properties": {"answer": {"type": "string"}},
        "required": ["answer"],
        "additionalProperties": false
    }),
)?;

let answer: serde_json::Value = client
    .structured_prompt_with_schema("Summarize the incident", &schema)
    .await?;
```

Validate business rules after deserialization. Schema-conforming model output
can still be false, malicious, stale, or unauthorized.

## 6. Streaming and tools

The built-in v12 provider transports do not expose token streaming. Rullst can
host ordinary Axum SSE responses, but an application that uses a third-party
streaming SDK owns its authentication, guardrails, backpressure, deadlines,
cancellation, error mapping, and dependency lifecycle. Do not present that
escape hatch as native `rullst-ai` streaming.

`ToolRegistry` stores local tools but is not wired to provider function calling.
Its [guarded execution API](ai-tool-security.md) requires an exact allowlist,
principal authorization, closed JSON validation, size/call limits and an audit
sink. `Destructive` and `Financial` tools additionally require a one-use human
approval bound to the exact JSON payload. Treat model output as untrusted input;
the application still authenticates the principal/approver, enforces domain
ownership and supplies durable production auditing.

## 7. RAG boundary

Rullst supplies prompt construction and an in-memory vector index. Applications
must still enforce document authorization before retrieval, prevent SSRF in any
fetcher, bound document and prompt sizes, identify tenant provenance, and avoid
sending secrets to a provider. Similarity is a ranking signal, not an access
control decision.

## 8. Durable chat memory

Enable the dedicated SQL memory feature when the application wants the
framework-owned fixed schema rather than generated models:

```toml
rullst = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["ai-sql-memory"]
}
```

The same adapter supports SQLite, PostgreSQL, MySQL, and MariaDB URLs:

```rust,no_run
use rullst::{
    ai::{
        AiClient, ChatMemoryConfig, ConversationId, SqlChatMemory, StatefulChat,
        providers::openai::OpenAiProvider,
    },
    security::TenantContext,
};

async fn chat_service(
    database_url: String,
    api_key: String,
) -> Result<
    (StatefulChat<SqlChatMemory>, TenantContext, ConversationId),
    Box<dyn std::error::Error>,
> {
    let memory = SqlChatMemory::connect(database_url, ChatMemoryConfig::default()).await?;
    memory.prepare_schema().await?;
    let service = StatefulChat::new(
        AiClient::new(OpenAiProvider::new(api_key)),
        memory,
    );
    let tenant = TenantContext::try_new("tenant-42")?;
    let conversation = ConversationId::try_new("support:case-7")?;
    service.ensure_conversation(&tenant, &conversation).await?;
    Ok((service, tenant, conversation))
}
```

`service.send(&tenant, &conversation, text).await` loads only the configured
recent message pairs, applies the ordinary AI guardrails, and stores the user
and assistant messages in one transaction. If another process committed from
the same observed revision, the slower `StatefulChat` call receives
`StatefulChatError::Memory(ChatMemoryError::RevisionConflict)`. Decide
explicitly whether the UI asks the user to retry; the library will not repeat a
potentially billable provider call.

The table stores raw message text. Production code must authenticate
conversation ownership inside the selected tenant, decide encryption and key
management, implement retention/erasure and backup policy, audit provider use
without logging secrets, and manage schema changes through its release process.
Use `cargo rullst make:chat-session` instead when you need application-owned
models/migrations or the Turso-primary profile.
