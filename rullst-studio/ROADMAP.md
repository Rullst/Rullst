# Rullst Studio - Roadmap

> **Status policy (2026-08-26):** this UI backlog is preserved. See the audited
> [`rullst-studio` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and
> [capability ledger](../docs/src/capability-ledger.md) for the boundary between
> real local telemetry, unavailable sources, and future distributed tooling.

Rullst Studio is the visual development environment and dashboard for managing your Rullst applications. It aims to bridge the gap between code and data management, providing an experience similar to Prisma Studio but native to Rust.

## Phase 1: Data & API Management
- [x] **Visual Data Browser**: A fast, web-based graphical user interface (GUI) to view, filter, edit, and delete records directly from your database without writing SQL.
- [x] **API Playground**: Auto-generated Swagger/OpenAPI interface to test your REST endpoints interactively while developing.

## Phase 2: Observability & Profiling
- [x] **Real-time Request Logger**: Intercept and display incoming HTTP requests, payloads, headers, and response times in real-time.
- [ ] **N+1 Query Detection & SQL Profiling**: Visually highlight slow database queries or redundant ORM calls (N+1 problems) as they happen during development.
- [x] **Background Jobs Monitor**: A dashboard interface to monitor pending, failed, and completed background tasks (integration with `cargo-rullst` worker scaffolding).

## Phase 3: Advanced Tooling (Suggestions)
- [x] **Visual ER Diagram Generator**: Automatically render a beautiful, interactive Entity-Relationship diagram based on the database schema.
- [x] **Feature Flags Manager**: A UI to view, toggle, and manage feature flags in real-time (integrating with `DbFeatureDriver`).
- [ ] **Cache & Redis Inspector**: Explore cached keys, view their contents, and manually invalidate or flush cache entries from the Studio.
- [x] **Environment & Config Viewer**: Safely inspect which environment variables and configuration settings the application is currently using in development.
