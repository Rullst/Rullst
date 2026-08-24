# Rullst AI

> **Vision preserved:** capability-typed schema support, local-model boundaries,
> and autonomous-agent ideas remain itemized with an implementation opinion in the
> [capability ledger](../capability-ledger.md#ai-and-mail).

`rullst-ai` is a provider-agnostic LLM client with mandatory outbound prompt-injection checks,
PII masking, deterministic offline fixtures, JSON mode, and explicit JSON Schema output.

## Provider capabilities

| Provider | Chat | Vision | Embeddings | JSON mode | Native JSON Schema |
|---|---:|---:|---:|---:|---:|
| OpenAI | yes | yes | yes | yes | yes |
| Gemini | yes | yes | yes | yes | yes |
| Anthropic | yes | yes | no | prompt-constrained | no |
| DeepSeek | yes | no | no | yes | `deepseek-v4-flash` |
| Ollama | yes | model-dependent | yes | yes | local API only |

Unsupported capabilities return `AiError::UnsupportedCapability`; the client does not silently
switch to an unrelated endpoint or represent a fixture as a live-provider result.

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

## Offline mode

OpenAI, Gemini, Anthropic, and DeepSeek use deterministic offline mode when their API key is empty
or begins with `mock_`. Ollama uses an empty or `mock_*` host. Offline branches return before URL
construction or HTTP dispatch and cover each capability that the provider implements. Unsupported
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

Streaming responses, durable chat memory, provider-native tool execution loops, external vector
database adapters, and compile-time schema derivation remain roadmap work. The in-memory vector
index and tool registry are utilities; they do not create an authorization boundary by themselves.
