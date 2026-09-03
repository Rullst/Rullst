# Rullst Studio - Roadmap

> **Status policy (2026-08-26):** this UI backlog is preserved. See the audited
> [`rullst-studio` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and
> [capability ledger](../docs/src/capability-ledger.md) for the boundary between
> real local telemetry, unavailable sources, and future distributed tooling.

Rullst Studio is the visual development environment and dashboard for managing your Rullst applications. It aims to bridge the gap between code and data management, providing an experience similar to Prisma Studio but native to Rust.

## Phase 1: Data & API Management
- [x] **Bounded Visual Data Browser**: Read, filter, edit primitive non-key
  values and delete one primary-key-selected row across SQLite, PostgreSQL,
  MySQL and MariaDB. Values are bound and schema identifiers come from a strict
  allowlist; composite primitive keys are supported, backend-specific values
  stay read-only, deletion requires exact textual confirmation, and every write
  requires the unforgeable verified-loopback/same-origin request marker.
- [~] **API Playground**: Interactive Swagger UI for an `OpenApi` document explicitly supplied through `Studio::with_openapi`. Studio does not infer a complete specification from arbitrary Axum routes.

## Phase 2: Observability & Profiling
- [~] **Real-time Request Logger**: Bounded SSE view of method, URI, status, and latency for the routes carrying its middleware. Bodies and headers are deliberately not captured by default because they can contain credentials or personal data.
- [x] **Bounded SQL Profiling**: HMAC-authenticated distributed v1 spans can
  carry only a caller-redacted operation label, kind, W3C-compatible IDs,
  timing and status. Studio flags operations at least 100 ms and three equal
  SQL labels in one trace. Repetition is explicitly heuristic rather than
  proof of N+1; SQL text, bindings, attributes and errors are not accepted.
- [x] **Background Jobs Monitor (bounded)**: Inspect up to 50 real records exposed by a supplied queue, including pending/processing/failed/completed, with failed retry/purge and completed-history purge. SQLite removes successes by default or explicitly retains 1–100,000 with atomic pruning through `Queue::sqlite_with_completed_history`; payload access/retention remains host policy. Unsupported Redis/custom inspection operations stay visible errors.

## Phase 3: Advanced Tooling (Suggestions)
- [x] **Visual ER Diagram Generator**: Render a read-only Mermaid diagram from parameterized SQLite, PostgreSQL, MySQL, or MariaDB schema inspection. Unsupported/unconfigured sources remain visibly unavailable and identifiers are normalized before Mermaid rendering.
- [x] **Bounded Feature Flags Manager**: View and toggle rows in the same `rullst_feature_flags` table used by `DbFeatureDriver`. A successful Studio write advances a constant-size process epoch, so already-warm drivers refresh on their next evaluation. Other processes/direct writers still converge by TTL unless the application distributes invalidation.
- [x] **Metadata-only Cache & Redis Inspector**: When the application supplies
  a `Cache`, inspect at most 100 entries and invalidate one through a
  process-bound opaque HMAC token. Logical keys, values and bulk flush stay out
  of the browser. Memory and Redis implement the bounded metadata contract;
  custom drivers fail explicitly until they opt in.
- [x] **Environment & Config Viewer**: Inspect environment keys with deny-by-default value redaction plus a safe projection of the process-global typed `RullstConfig`; database URLs, filesystem paths, secrets, cookies and credentials are never rendered.
