# Rullst Core ⚙️

`rullst-core` encapsulates the foundational primitives, routing engines, state management, and configuration layers of the Rullst Framework. It acts as the beating heart that orchestrates HTTP handlers, middleware, and backend worker systems.

## ✨ Features

- **Zero-Cost Abstractions:** Extends `axum` routing for sub-millisecond response times without sacrificing safety.
- **Rullst Studio Integration:** Embedded live-monitoring and tracing spans to observe application behavior in real-time.
- **Unified Error Handling:** The `AppError` enum standardizes error propagation, guaranteeing a "Zero-Panic" runtime environment and consistent API responses.
- **Dependency Injection:** Type-safe, intuitive global state management across routes and background workers.
- **Environment Management:** Native `dotenv` and TOML configuration loaders for different deployment targets (Staging, Production, Local).

## 🚀 Usage

Most developers will not depend on `rullst-core` directly, as it is re-exported seamlessly through the primary `rullst` crate. 

If you are developing a plugin or advanced middleware for the Rullst ecosystem, you can add it explicitly:

```bash
cargo add rullst-core
```

### Accessing Global State

```rust
use rullst_core::{State, AppError};
use rullst_core::http::Json;
use serde::Serialize;

#[derive(Serialize)]
struct Status {
    healthy: bool,
}

pub async fn health_check(state: State<MyGlobalState>) -> Result<Json<Status>, AppError> {
    // Safely access global configuration without unwrap()
    let is_db_ready = state.db.is_ready().await?;
    
    Ok(Json(Status { healthy: is_db_ready }))
}
```

## 🔐 Security Audit

`rullst-core` is the most audited crate in the framework. It undergoes continuous fuzzing against malformed routing requests and is structurally verified against memory leaks using Miri. All functions returning `Result` strictly avoid panicking on corrupted payloads.

## 📚 Documentation

For an architectural deep-dive into Rullst Core's event loop and middleware lifecycle, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
