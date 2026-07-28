# Rullst Core

`rullst-core` is the engine that drives the Rullst Framework. It provides the essential traits, routing wrappers, state management, and configuration utilities that hold the entire ecosystem together.

## What's Inside?

- **Routing Engine:** Thin abstraction over `axum` for enhanced developer experience.
- **State Management:** Typed injection container for Database Pools (`RullstPool`), Mailers, and custom App State.
- **Error Handling:** The `AppError` type for unified, zero-panic error propagation.
- **Rullst Studio Backend:** Real-time telemetry and monitoring data aggregators (`studio.rs`).

## Integrating Core

Typically, you don't interact with `rullst-core` directly unless you're building a plugin. The main `rullst` crate re-exports the necessary types from `rullst-core` automatically.
