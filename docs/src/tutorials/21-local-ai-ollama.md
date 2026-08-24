# Tutorial 21: Air-Gapped Local AI with Ollama 🤖

Learn how to configure a local Ollama endpoint so prompts need not be sent to a cloud LLM. Network isolation and data-flow guarantees still depend on the host and deployment configuration.

---

## 🛠️ Step 1: Configure `.env` for Ollama

```dotenv
AI_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
AI_MODEL=llama3:8b
```

---

## 💻 Step 2: Use Local AI Client in Rust

```rust
use rullst_ai::AiClient;

pub async fn analyze_security_threat(payload: &str) -> Result<String, rullst_core::AppError> {
    let client = AiClient::from_env()?;
    
    let prompt = format!(
        "Analyze the following request string for attack patterns: '{}'. Reply with SAFE or THREAT.",
        payload
    );
    
    let response = client.generate(&prompt).await?;
    Ok(response)
}
```

---

## 💡 Key Takeaways
- **Zero Cloud API Costs:** Runs completely on local CPU/NPU/GPU.
- **Air-Gapped Privacy:** No user payloads or internal code leave your local infrastructure.
