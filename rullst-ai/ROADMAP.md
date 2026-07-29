# Rullst AI - Roadmap

The goal of `rullst-ai` is to provide a seamless, high-level developer experience for building AI-powered applications in Rust, tightly coupled with the Rullst ORM native vector search capabilities (`pgvector`).

## Phase 1: Core AI Integrations
- [x] **Native LLM Wrappers**: Idiomatic, asynchronous wrappers for the most popular models (OpenAI, Anthropic, Google Gemini) to generate text and parse structured JSON responses directly into Rust structs.
- [x] **Auto-Embeddings Sync**: Introduce an `#[ai_embedding(field="content")]` macro attribute. Whenever a model is saved or updated, automatically call the Embedding API in the background and save the resulting vector to the database via ORM hooks.

## Phase 2: RAG Pipeline
- [x] **RAG in-a-box**: A single macro/function that takes a user prompt, automatically queries the database using `order_by_cosine_distance`, builds the context window, and returns the generated LLM response.
- [ ] **Chat Memory**: Built-in state management for Conversational AI, persisting chat history directly to the `messages` SQL table automatically.

## Phase 3: Structured Outputs & Tool Calling
- [ ] **Strict JSON Schemas (`#[derive(AiSchema)]`)**: Force the LLM to reply exactly in the format of a Rust Struct using Structured Outputs. The macro generates the JSON Schema automatically at compile time.
- [ ] **Agentic Tool Calling**: Allow developers to register simple Rust functions annotated with `#[ai_tool]`. The LLM can decide to "call" these functions, and Rullst parses arguments and executes the Rust code transparently.

## Phase 4: Multimodal & Vision
- [ ] **Vision Abstraction**: Native support for sending local images, URLs, or binary blobs to models like GPT-4o and Gemini Pro Vision via a fluent API (e.g., `client.prompt_with_image("What is this?", bytes)`).

## Phase 5: Local AI & Fallbacks
- [ ] **Resilient AI Routing**: Automatic fallback routing if an API (e.g., OpenAI) goes down or times out, switching to Anthropic or a local Ollama model to guarantee 100% uptime for AI features.

## Phase 6: CLI Tooling for AI
- [ ] **`cargo rullst make:chat-session`**: Automatically generates the `ChatSession` and `ChatMessage` models along with their SQL migrations for instant Chat Memory setup.
- [ ] **`cargo rullst make:ai-agent <Name>`**: Scaffolds an AI agent struct with pre-configured tools.
