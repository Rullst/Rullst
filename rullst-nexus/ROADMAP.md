# Rullst Nexus - Roadmap

> **Status policy (2026-08-26):** this backlog is preserved; the audited
> [`rullst-nexus` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and
> [capability ledger](../docs/src/capability-ledger.md) distinguish implemented
> server-side controls from host-policy and future UI work.

Rullst Nexus is the official Admin Panel for the Rullst ecosystem. It aims to provide a fully-featured, Django Admin-like experience out of the box, with zero frontend code required.

## Phase 1: Core CRUD (Next Priority)
- [x] **Full CRUD Auto-generation**: Use macro introspection to dynamically generate Create, Edit, and Delete forms for any struct deriving `#[derive(Nexus)]`.
- [x] **Data Tables**: Implement server-side pagination, searching, and column sorting.
- [x] **Data Types Formatting**: Automatically render boolean fields as toggles, Enums as dropdowns, and text as textareas.

## Phase 2: Advanced Features
- [x] **Batch Actions**: Select multiple records and trigger mass operations (e.g., Delete All, Deactivate).
- [ ] **Custom Dashboards**: Ability to inject custom widgets, charts, and metrics directly into the admin homepage.
- [ ] **Visual SQL Builder**: A UI to generate complex queries graphically and export the generated Rust code.
