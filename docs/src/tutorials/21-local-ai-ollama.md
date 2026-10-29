# Tutorial 21: Explicit Local AI with Ollama 🤖

Use an explicit `OllamaProvider` when a workload must not fall through to a
configured cloud provider. `AiClient::auto()` is a convenience fallback chain;
it is not an isolation policy when cloud API keys are also present.

You need the `ai` feature of `rullst` (`cargo rullst new --ai`, or
`cargo rullst add ai` in an existing project) and an Ollama server you run and
provision yourself.

---

## Step 1: Configure the local endpoint

Add both settings to the process environment or the project's `.env`:

```dotenv
OLLAMA_HOST=http://127.0.0.1:11434
OLLAMA_MODEL=llama3:8b
```

Rullst never loads `.env` into the process environment, so the code below reads
them with `rullst::config::project_setting` (the process environment first, then
`.env`), the same way the generated starters read their own settings.

Bind Ollama to loopback or a controlled private interface. Pull and license the
chosen model (`ollama pull llama3:8b`) through a reviewed provisioning step
rather than silently during a request.

---

## Step 2: Construct the provider explicitly

```rust,no_run
use rullst::ai::{AiClient, AiError, providers::ollama::OllamaProvider};

/// Reads a setting from the process environment, then `.env`.
fn required_setting(name: &str) -> Result<String, AiError> {
    rullst::config::project_setting(name)
        .map_err(|error| AiError::ConfigError(error.to_string()))?
        .ok_or_else(|| AiError::ConfigError(format!("{name} is required")))
}

pub async fn summarize_lesson(text: &str) -> Result<String, AiError> {
    let host = required_setting("OLLAMA_HOST")?;
    let model = required_setting("OLLAMA_MODEL")?;
    let client = AiClient::new(OllamaProvider::new(host, model));

    client
        .prompt(&format!(
            "Summarize this lesson in three factual bullet points:\n{text}"
        ))
        .await
}
```

A missing setting returns `AiError::ConfigError`, and a server that cannot be
reached returns `AiError::RequestError`; neither falls back to another provider.

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
