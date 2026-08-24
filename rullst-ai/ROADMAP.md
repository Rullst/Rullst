# Rullst AI roadmap

## Implemented

- Guarded high-level client for prompt, chat, vision, embeddings, JSON, and schema output.
- Prompt-injection heuristics and outbound PII masking before provider dispatch.
- OpenAI, Gemini, Anthropic, DeepSeek, and Ollama provider adapters.
- Deterministic empty/`mock_*` offline paths for supported provider capabilities.
- Explicit separation between parseable JSON mode and provider-enforced JSON Schema output.
- Ordered provider fallback, RAG prompt helper, vector index, and tool registry.

## Planned

- Streaming response abstraction and cancellation contract.
- Compile-time JSON Schema derivation for Rust response types.
- Provider-native tool invocation loop with explicit authorization boundaries.
- Durable chat-memory adapters and ORM lifecycle hooks.
- Retrieval adapters for external vector databases.
- Configurable, versioned guardrail policies and model-specific safety evaluations.

Roadmap items are not part of the current API guarantee until implemented and covered by contract
tests.
