# Rullst AI 🤖

`rullst-ai` is the official integration layer for Autonomous AI Agents within the Rullst Framework. It provides typed bindings, tool definitions, and system prompts optimized for interacting with OpenAI, Anthropic, and Google DeepMind agentic systems.

## ✨ Features

- **Agent Framework Integration:** First-class abstractions to integrate conversational AI, autonomous task executors, and coding agents directly into your Rullst SaaS backend.
- **MCP (Model Context Protocol) Ready:** Natively supports structured tool discovery so AI agents can query your `rullst-orm` database securely.
- **Streaming Responses:** Built-in adapters to stream LLM responses (Server-Sent Events or WebSockets) directly through Rullst's asynchronous `axum` engine.
- **RAG Bindings:** Utility traits to quickly connect Vector Databases (Qdrant, Pinecone) with your ORM models for Retrieval-Augmented Generation.

## 🚀 Quickstart

Add `rullst-ai` to your project:

```bash
cargo add rullst-ai
```

### Basic Completion Endpoint

```rust
use rullst::{Router, routing::post};
use rullst_core::http::Json;
use rullst_ai::{providers::OpenAiClient, ChatRequest, ChatResponse};
use serde::Deserialize;

#[derive(Deserialize)]
struct Prompt {
    message: String,
}

async fn chat_handler(Json(payload): Json<Prompt>) -> Json<ChatResponse> {
    let client = OpenAiClient::from_env();
    
    let request = ChatRequest::builder()
        .system("You are a helpful assistant.")
        .user(&payload.message)
        .build();

    let response = client.complete(request).await.expect("AI request failed");
    Json(response)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/chat", post(chat_handler));
    // ... start server ...
}
```

## 🔐 AI Security Firewall & Prompt Shield v2

`rullst-ai` works natively with `rullst-security::ai_firewall` to provide zero-latency prompt inspection and defense-in-depth:

```rust
use rullst_security::{LlmFirewall, ai_firewall_middleware, PromptThreatCategory};

// 1. Direct prompt inspection before dispatch
let report = LlmFirewall::inspect_prompt(&payload.message);
if !report.is_safe {
    return Err(AiError::BlockedByFirewall(format!(
        "Threat detected: {:?}",
        report.threat_category
    )));
}

// 2. Or attach AI Firewall middleware to route
let app = Router::new()
    .route("/api/chat", post(chat_handler))
    .layer(axum::middleware::from_fn(ai_firewall_middleware));
```

### Threat Vectors Neutralized:
- **Direct Jailbreaks:** "Ignore previous instructions", "DAN mode", "Developer Mode".
- **System Prompt Exfiltration:** "Repeat initial prompt", "Reveal base instructions".
- **Delimiter Collisions:** `<|im_start|>`, `[INST]`, `<<SYS>>`.
- **Markdown Data Leaks:** Malicious image callback beacons `![leak](https://evil.com/...)`.
- **Invisible Unicode:** Strips and flags `\u{200B}` zero-width injection attacks.

## 📚 Documentation

For advanced usage, building custom AI tools, and setting up streaming endpoints, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
