# Zero Lock-In & Migration Guide: Axum + SQLx ↔ Rullst

Rullst is built directly on top of `Axum`, `Tokio`, and `Tower`. It does not invent proprietary HTTP abstractions or incompatible router types. Every Rullst controller, extractor, and middleware maps 1:1 to standard Axum and Tower equivalents.

This document serves as both:
1. An **Incremental Adoption Guide** (moving an existing `Axum + SQLx` app to Rullst).
2. An **Escape Hatch Specification** (extracting Rullst routes to a pure `Axum` app).

---

## 1. Incremental Migration (Axum → Rullst)

You do **not** need to rewrite your application to adopt Rullst. You can mount existing `axum::Router` instances directly into a Rullst server.

### Step 1: Mounting Existing Axum Routers

Existing Axum routes can be attached directly to Rullst's `Server`:

```rust
use axum::{routing::get, Router};
use rullst::server::Server;

async fn legacy_axum_handler() -> &'static str {
    "Hello from existing Axum handler!"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Existing Axum Router
    let legacy_router = Router::new().route("/legacy", get(legacy_axum_handler));

    // 2. Attach seamlessly into Rullst Server
    Server::new()
        .nest("/api/v1", legacy_router)
        .listen("127.0.0.1:3000")
        .await?;

    Ok(())
}
```

### Step 2: Using Raw `sqlx::Pool` alongside Rullst ORM

Rullst ORM operates on top of standard `sqlx::Pool`. If you have raw SQL queries written for `sqlx`, they run natively:

```rust
use rullst::db::sqlx;

pub async fn custom_raw_query(pool: &sqlx::PgPool) -> Result<Vec<String>, sqlx::Error> {
    let names = sqlx::query_scalar!("SELECT name FROM users WHERE active = true")
        .fetch_all(pool)
        .await?;

    Ok(names)
}
```

---

## 2. Escape Hatch: Extracting Rullst to Pure Axum

If your project requirements change and you decide to extract your codebase from Rullst back to pure `Axum + SQLx`:

### Extractor & Handler Equivalence Matrix

| Rullst Extractor / Return | Pure Axum Equivalent | Lock-In Cost |
| :--- | :--- | :--- |
| `rullst::server::Json(data)` | `axum::Json(data)` | 0 (direct alias) |
| `rullst::server::Path(id)` | `axum::extract::Path(id)` | 0 (direct alias) |
| `rullst::server::Query(q)` | `axum::extract::Query(q)` | 0 (direct alias) |
| `rullst::server::Request` | `axum::http::Request<axum::body::Body>` | 0 (direct alias) |
| `rullst::server::Response` | `axum::response::Response` | 0 (direct alias) |
| `rullst::server::Next` | `axum::middleware::Next` | 0 (direct alias) |

### Refactoring a Controller to Pure Axum

A Rullst controller function:

```rust
// In Rullst:
use rullst::server::{Json, Path, IntoResponse};

pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {
    Json(serde_json::json!({ "id": id }))
}
```

Refactored to pure Axum (1-line import change):

```rust
// In pure Axum:
use axum::{extract::Path, response::IntoResponse, Json};

pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {
    Json(serde_json::json!({ "id": id }))
}
```

---

## Summary

Rullst is designed to preserve access to standard Axum and SQLx APIs. Some
generated and framework-specific code still requires an explicit migration when
removing Rullst dependencies.
