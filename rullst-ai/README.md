# Rullst AI

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

Unsupported capabilities return `AiError::UnsupportedCapability`; they do not silently switch to
an unrelated endpoint or fabricate a live-provider result.

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

## Offline mode

OpenAI, Gemini, Anthropic, and DeepSeek select deterministic offline mode when their API key is empty
or starts with `mock_`. Ollama uses an empty or `mock_*` host. Offline branches return before URL
construction or HTTP dispatch and cover every capability the provider implements. Unsupported live
capabilities remain typed errors in offline mode.

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
