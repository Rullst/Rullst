# Rullst Studio - Roadmap

> **Status policy (2026-08-26):** this UI backlog is preserved. See the audited
> [`rullst-studio` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and
> [capability ledger](../docs/src/capability-ledger.md) for the boundary between
> real local telemetry, unavailable sources, and future distributed tooling.

Rullst Studio is the visual development environment and dashboard for managing your Rullst applications. It aims to bridge the gap between code and data management, providing an experience similar to Prisma Studio but native to Rust.

## Phase 1: Data & API Management
- [~] **Visual Data Browser**: Read and filter allowlisted SQLx tables with bounded pagination and escaped output. Edit/delete UI is not implemented; Studio must not be described as general database CRUD.
- [~] **API Playground**: Interactive Swagger UI for an `OpenApi` document explicitly supplied through `Studio::with_openapi`. Studio does not infer a complete specification from arbitrary Axum routes.

## Phase 2: Observability & Profiling
- [~] **Real-time Request Logger**: Bounded SSE view of method, URI, status, and latency for the routes carrying its middleware. Bodies and headers are deliberately not captured by default because they can contain credentials or personal data.
- [ ] **N+1 Query Detection & SQL Profiling**: Visually highlight slow database queries or redundant ORM calls (N+1 problems) as they happen during development.
- [~] **Background Jobs Monitor**: Inspect up to 50 records exposed by a supplied queue, including pending/processing/failed and any completed records retained by a custom backend, with retry and failed-job purge. The SQLite worker removes successful jobs, so it does not provide completion history.

## Phase 3: Advanced Tooling (Suggestions)
- [x] **Visual ER Diagram Generator**: Render a read-only Mermaid diagram from parameterized SQLite, PostgreSQL, MySQL, or MariaDB schema inspection. Unsupported/unconfigured sources remain visibly unavailable and identifiers are normalized before Mermaid rendering.
- [x] **Bounded Feature Flags Manager**: View and toggle rows in the same `rullst_feature_flags` table used by `DbFeatureDriver`. A successful Studio write advances a constant-size process epoch, so already-warm drivers refresh on their next evaluation. Other processes/direct writers still converge by TTL unless the application distributes invalidation.
- [ ] **Cache & Redis Inspector**: Explore cached keys, view their contents, and manually invalidate or flush cache entries from the Studio.
- [x] **Environment & Config Viewer**: Inspect environment keys with deny-by-default value redaction plus a safe projection of the process-global typed `RullstConfig`; database URLs, filesystem paths, secrets, cookies and credentials are never rendered.
