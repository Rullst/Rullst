# Rullst AI roadmap

> **Status policy (2026-08-26):** this detailed backlog is preserved. Its
> verified interpretation is the [`rullst-ai` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> in the canonical roadmap and the [capability ledger](../docs/src/capability-ledger.md).

## Implemented

- Guarded high-level client for prompt, chat, vision, embeddings, JSON, and schema output.
- Prompt-injection heuristics and outbound PII masking before provider dispatch.
- Versioned deterministic offline injection, jailbreak, and PII regression corpus.
- OpenAI, Gemini, Anthropic, DeepSeek, and Ollama provider adapters.
- Capability-declared OpenAI-compatible adapter with explicit unauthenticated
  loopback and HTTPS/Bearer cloud constructors, bounded responses, no ambient
  proxy/redirects, and deterministic offline fixtures.
- Static-dispatch `StreamingAiClient<P>` with provider-independent chunk/output
  limits and an explicit cancellation signal, plus strict OpenAI-compatible
  SSE parsing when the exact endpoint/model declares that capability.
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
- Opt-in bounded remote audit delivery with an exact HMAC-SHA256 body
  signature, key/timestamp metadata, stable event identity across transient
  retries, cancellation and a closed acknowledgement contract. Receiver
  storage, deduplication, freshness enforcement, retention and key operations
  remain deployment responsibilities.
- Static-dispatch `AdaptiveAiEvaluator<P>` with bounded multi-turn strategy
  feedback, independent per-turn deadlines, explicit cancellation, typed
  pass/fail/inconclusive outcomes and raw-content-free versioned JSON reports.
  The repository tests the runner deterministically; operators still execute
  and review suites against each exact live model/configuration.

## Planned

- Equivalent streaming/cancellation implementations for each non-compatible
  provider protocol; ordinary non-streaming calls retain deadline/drop semantics.
- Compile-time JSON Schema derivation for Rust response types.
- Provider-native tool invocation loop with explicit authorization boundaries.
- Optional transactional ORM/outbox hooks around application-specific chat
  effects; the reusable SQL memory and CLI-owned Turso/custom scaffold remain
  separate choices.
- First-party `RagRetriever` adapters for external vector databases; applications can already
  implement the public static-dispatch boundary over ORM pgvector or Qdrant.
- Configurable guardrail policies and maintained domain-specific evaluation
  corpora for tool selection, groundedness and application output policy.
- Durable outbox orchestration, receiver implementations and external SIEM
  interoperability for authenticated audit events.

Roadmap items are not part of the current API guarantee until implemented and covered by contract
tests.
