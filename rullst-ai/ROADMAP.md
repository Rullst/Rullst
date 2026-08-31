# Rullst AI roadmap

> **Status policy (2026-08-26):** this detailed backlog is preserved. Its
> verified interpretation is the [`rullst-ai` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> in the canonical roadmap and the [capability ledger](../docs/src/capability-ledger.md).

## Implemented

- Guarded high-level client for prompt, chat, vision, embeddings, JSON, and schema output.
- Prompt-injection heuristics and outbound PII masking before provider dispatch.
- Versioned deterministic offline injection, jailbreak, and PII regression corpus.
- OpenAI, Gemini, Anthropic, DeepSeek, and Ollama provider adapters.
- Deterministic empty/`mock_*` offline paths for supported provider capabilities.
- Explicit separation between parseable JSON mode and provider-enforced JSON Schema output.
- Ordered provider fallback, RAG prompt helper, vector index, and tool registry.
- Bounded `RagPipeline` with trusted tenant context, guarded embedding/context/generation,
  Unicode-safe context budgets, fail-closed no-context behavior, source metadata, mandatory
  secret-minimized audit, and a tenant-partitioned process-local cosine retriever.
- `cargo rullst make:chat-session` application scaffold with SQLx and
  Turso-primary models, reversible migrations, bounded ordered history,
  serialized sends, propagated persistence errors, and materialized runtime
  contract tests.
- Static-dispatch tenant-aware `ChatMemory` and `StatefulChat` contracts with a
  bounded offline implementation plus an opt-in durable SQLx adapter for
  SQLite, PostgreSQL, MySQL, and MariaDB. SQL exchanges commit atomically and a
  revision compare-and-swap rejects stale cross-process writers.
- Deny-by-default HTTPS egress fetcher with exact-host allowlist, validated and
  pinned DNS answers, manual redirects, peer verification, and streaming limits.

## Planned

- Streaming response abstraction and cancellation contract.
- Compile-time JSON Schema derivation for Rust response types.
- Provider-native tool invocation loop with explicit authorization boundaries.
- Optional transactional ORM/outbox hooks around application-specific chat
  effects; the reusable SQL memory and CLI-owned Turso/custom scaffold remain
  separate choices.
- First-party `RagRetriever` adapters for external vector databases; applications can already
  implement the public static-dispatch boundary over ORM pgvector or Qdrant.
- Configurable guardrail policies and adaptive/live model-specific safety evaluations.

Roadmap items are not part of the current API guarantee until implemented and covered by contract
tests.
