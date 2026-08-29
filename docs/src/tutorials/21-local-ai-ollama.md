# Tutorial 21: Air-Gapped Local AI with Ollama 🤖

Learn how to configure a local Ollama endpoint so prompts need not be sent to a cloud LLM. Network isolation and data-flow guarantees still depend on the host and deployment configuration.

---

## 🛠️ Step 1: Configure `.env` for Ollama

```dotenv
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3:8b
```

---

## 💻 Step 2: Use Local AI Client in Rust

```rust
use rullst_ai::ai::AiClient;

pub async fn analyze_security_threat(payload: &str) -> Result<String, rullst_ai::AiError> {
    let client = AiClient::auto()?;
    
    let prompt = format!(
        "Analyze the following request string for attack patterns: '{}'. Reply with SAFE or THREAT.",
        payload
    );
    
    let response = client.prompt(&prompt).await?;
    Ok(response)
}
```

---

## 💡 Key Takeaways
- No cloud LLM request is required when `AiClient::auto()` selects the configured
  Ollama endpoint. Hardware, electricity, hosting, and model licenses still have
  costs.
- “Local” does not prove an air gap. Verify host networking, Ollama bind address,
  model downloads, DNS/proxies, application logs, telemetry, and backups.
