# Rullst Studio - Roadmap

Rullst Studio is the visual development environment and dashboard for managing your Rullst applications. It aims to bridge the gap between code and data management, providing an experience similar to Prisma Studio but native to Rust.

## Phase 1: Data & API Management
- [ ] **Visual Data Browser**: A fast, web-based graphical user interface (GUI) to view, filter, edit, and delete records directly from your database without writing SQL.
- [ ] **API Playground**: Auto-generated Swagger/OpenAPI interface to test your REST endpoints interactively while developing.

## Phase 2: Observability & Profiling
- [ ] **Real-time Request Logger**: Intercept and display incoming HTTP requests, payloads, headers, and response times in real-time.
- [ ] **N+1 Query Detection & SQL Profiling**: Visually highlight slow database queries or redundant ORM calls (N+1 problems) as they happen during development.
- [ ] **Background Jobs Monitor**: A dashboard interface to monitor pending, failed, and completed background tasks (integration with `cargo-rullst` worker scaffolding).
