//! Provider-agnostic LLM client for Rullst with mandatory outbound guardrails.
//!
//! [`AiClient`] wraps one [`AiProvider`] (OpenAI, Gemini, Anthropic, DeepSeek,
//! Ollama or an OpenAI-compatible endpoint, see [`providers`]) and runs the
//! prompt-injection and PII checks of [`guardrails`] before every provider call.
//! [`AiClient::auto`] selects a provider from environment variables. Without
//! one it uses a deterministic offline fixture in development and test and
//! returns [`AiError::ConfigError`] in staging and production; empty or
//! `mock_*` credentials never reach the network.
//!
//! The crate also provides JSON mode and provider-enforced JSON Schema output
//! ([`StructuredOutputSchema`]), bounded streaming with explicit cancellation
//! ([`StreamingAiClient`]), tenant-bound chat memory ([`StatefulChat`]), a
//! tenant-aware RAG pipeline ([`rag`]), a deny-by-default egress fetcher
//! ([`EgressFetcher`]) and adaptive evaluations ([`AdaptiveAiEvaluator`]).
//!
//! **Maturity: Extension**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)): optional, and
//! supported for its documented scope. Everything above is available with
//! default features; the `sql-memory` feature adds `SqlChatMemory` for SQLite,
//! PostgreSQL, MySQL and MariaDB.
//!
//! See the [book page](https://rullst.github.io/Rullst/book/crates/ai.html) and the
//! [README](https://github.com/Rullst/Rullst/blob/main/rullst-ai/README.md).
//!
//! # Reporting problems
//!
//! If this crate does not behave as documented, open an issue at
//! <https://github.com/Rullst/Rullst/issues/new/choose> with the Rullst version,
//! enabled features, platform and a minimal reproduction, without secrets or
//! private data. Report security vulnerabilities privately as described in
//! [`SECURITY.md`](https://github.com/Rullst/Rullst/blob/main/SECURITY.md),
//! never in a public issue.
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod ai;
pub use ai::*;
