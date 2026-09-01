# Tutorial 21: Explicit Local AI with Ollama 🤖

Use an explicit `OllamaProvider` when a workload must not fall through to a
configured cloud provider. `AiClient::auto()` is a convenience fallback chain;
it is not an isolation policy when cloud API keys are also present.

---

## Step 1: Configure the local endpoint

```dotenv
OLLAMA_HOST=http://127.0.0.1:11434
OLLAMA_MODEL=llama3:8b
```

Bind Ollama to loopback or a controlled private interface. Pull and license the
chosen model through a reviewed provisioning step rather than silently during a
request.

---

## Step 2: Construct the provider explicitly

```rust,no_run
use rullst_ai::ai::{AiClient, AiError, providers::ollama::OllamaProvider};

pub async fn summarize_lesson(text: &str) -> Result<String, AiError> {
    let host = std::env::var("OLLAMA_HOST")
        .map_err(|_| AiError::ConfigError("OLLAMA_HOST is required".to_string()))?;
    let model = std::env::var("OLLAMA_MODEL")
        .map_err(|_| AiError::ConfigError("OLLAMA_MODEL is required".to_string()))?;
    let client = AiClient::new(OllamaProvider::new(host, model));

    client
        .prompt(&format!(
            "Summarize this lesson in three factual bullet points:\n{text}"
        ))
        .await
}
```

The high-level client applies Rullst's mandatory prompt/PII guardrails before
dispatch. Model output remains untrusted: escape it for HTML, validate structured
data, and never use an LLM verdict as the only authentication, authorization,
malware, or abuse-control decision.

---

## Isolation checklist

- Confirm the resolved endpoint, host firewall, container network, DNS, proxy,
  and telemetry configuration.
- Remove cloud-provider credentials from the process when policy requires local
  only; an explicit provider prevents fallback but least privilege still helps.
- Review application/Ollama logs, model caches, backups, crash dumps, and swap
  for sensitive data.
- Empty or `mock_*` Ollama hosts select deterministic offline mock behavior; a
  mock response is test evidence, not evidence that a live model ran.

“Local” does not prove an air gap. Verify the complete deployed data flow.
