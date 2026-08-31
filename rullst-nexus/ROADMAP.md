# Rullst Nexus - Roadmap

> **Status policy (2026-08-26):** this backlog is preserved; the audited
> [`rullst-nexus` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and
> [capability ledger](../docs/src/capability-ledger.md) distinguish implemented
> server-side controls from host-policy and future UI work.

Rullst Nexus is the official Admin Panel for the Rullst ecosystem. It aims to provide a fully-featured, Django Admin-like experience out of the box, with zero frontend code required.

## Phase 1: Core CRUD (Next Priority)
- [x] **Full CRUD Auto-generation**: Generate registered-model Create, Edit, and Delete forms for named-field structs deriving `#[derive(Nexus)]`, with explicit metadata for semantics the Rust type cannot reveal.
- [x] **Data Tables**: Implement server-side pagination, searching, and column sorting.
- [x] **Data Types Formatting (bounded)**: Infer Boolean widgets and render
  checkboxes plus explicitly declared Enum dropdown/Textarea metadata. Startup
  validates and bounds the registered metadata; mutation handlers bound form
  pairs/bytes, reject unknown/protected/duplicate fields, normalize Boolean
  controls and allow only the registered enum values before bound SQL.
  Automatic discovery of variants or multiline intent from an unrelated Rust
  type is deliberately not claimed.

## Phase 2: Advanced Features
- [x] **Batch Actions**: Delete explicitly selected records or deactivate them when the model exposes a writable Boolean `is_active`/`active` field, with a 1,000-record bound.
- [ ] **Custom Dashboards**: Ability to inject custom widgets, charts, and metrics directly into the admin homepage.
- [ ] **Visual SQL Builder**: A UI to generate complex queries graphically and export the generated Rust code.
