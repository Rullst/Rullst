# Integrating AI into Rullst

Building AI features like Chatbots, Retrieval-Augmented Generation (RAG), or generative workflows is seamless in Rullst. Since Rullst is built on async Rust, it provides the perfect backend performance to stream AI responses to clients effortlessly.

This tutorial covers the standard approach to integrating LLMs (Large Language Models) such as OpenAI, Anthropic, or Gemini into your Rullst application.

## Prerequisites

Rullst does not force any specific AI vendor on you. The standard way to integrate AI in Rust is by using the `async-openai` or `reqwest` crate for making API calls.

To get started, add the following to your `Cargo.toml`:

```toml
[dependencies]
async-openai = "0.23.0"
tokio-stream = "0.1" # For streaming responses
```

## 1. Creating the AI Service

The best practice in Rullst is to isolate your third-party integrations into a `Service` struct to keep your controllers clean. Let's create `src/services/ai_service.rs`:

```rust
use async_openai::{
    types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};
use rullst::AppError;

pub struct AiService;

impl AiService {
    /// Generates a static response from the AI
    pub async fn generate_response(prompt: &str) -> Result<String, AppError> {
        let client = Client::new(); // Automatically reads OPENAI_API_KEY from environment

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o")
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are a helpful assistant integrated into a Rullst application.")
                    .build()
                    .unwrap()
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()
                    .unwrap()
                    .into(),
            ])
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build request: {}", e)))?;

        let response = client
            .chat()
            .create(request)
            .await
            .map_err(|e| AppError::Internal(format!("OpenAI API Error: {}", e)))?;

        let text = response.choices.first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(text)
    }
}
```

## 2. Using the AI in a Controller

Now that the service is ready, you can expose it via a REST endpoint. In your `src/controllers/ai_controller.rs`:

```rust
use rullst::{Controller, HttpMethod, Route, AppError, Context};
use crate::services::ai_service::AiService;
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatPrompt {
    prompt: String,
}

pub struct AiController;

#[rullst::async_trait]
impl Controller for AiController {
    fn routes(&self) -> Vec<Route> {
        rullst::routes![
            (HttpMethod::POST, "/api/chat", Self::handle_chat),
        ]
    }
}

impl AiController {
    async fn handle_chat(ctx: Context) -> Result<String, AppError> {
        // Parse incoming JSON
        let body: ChatPrompt = ctx.json().await?;
        
        // Call our AI Service
        let response = AiService::generate_response(&body.prompt).await?;
        
        // Return the text response
        Ok(response)
    }
}
```

## 3. Streaming AI Responses (Server-Sent Events)

Modern AI apps feel fast because they **stream** the response token-by-token. Rullst supports SSE (Server-Sent Events) natively via `rullst::Response::sse()`.

Here is how you stream the OpenAI output directly to your Rullst frontend:

```rust
// Inside AiController...
use async_openai::types::CreateChatCompletionRequestArgs;
use futures::StreamExt;
use rullst::Response;

async fn stream_chat(ctx: Context) -> Result<Response, AppError> {
    let body: ChatPrompt = ctx.json().await?;
    let client = async_openai::Client::new();
    
    // Notice the `.stream(true)` configuration
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .messages([...]) // Add messages as before
        .stream(true)
        .build()
        .unwrap();

    let mut stream = client.chat().create_stream(request).await.unwrap();

    // Map the OpenAI stream into an SSE stream compatible with Rullst
    let sse_stream = async_stream::stream! {
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    if let Some(content) = &response.choices[0].delta.content {
                        yield Ok(rullst::sse::Event::default().data(content));
                    }
                }
                Err(e) => {
                    yield Ok(rullst::sse::Event::default().data(format!("Error: {}", e)));
                }
            }
        }
    };

    Ok(Response::sse(sse_stream))
}
```

## Next Steps

With these foundations, you can build advanced features:
- Use **SQLx (Rullst ORM)** to load context from PostgreSQL, converting it into JSON and sending it as Context to your prompt (building an instant RAG).
- Serve a frontend using **HTMX**, which has built-in SSE support to easily render the streaming text chunks without writing complex Javascript.
