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

## 🔐 Security Audit

`rullst-ai` is designed with defense-in-depth against prompt injection. When exposing database tools to AI via MCP, the layer requires explicit `#[ai_accessible]` macros on `rullst-orm` models. By default, agents cannot execute destructive queries (`DELETE`/`UPDATE`) unless explicitly opted-in by the developer. 

## 📚 Documentation

For advanced usage, building custom AI tools, and setting up streaming endpoints, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
