# Create Your First Rullst REST API

This quickstart creates one runnable JSON endpoint without a database or an AI
provider. It targets the unreleased v12 snapshot on `main`; use a versioned
crate or immutable tag for production.

## 1. Create the application

```bash
cargo new first_rullst_api
cd first_rullst_api
cargo add rullst --git https://github.com/Rullst/Rullst.git --branch main
cargo add tokio --features macros,rt-multi-thread
cargo add serde --features derive
```

Keep the generated `Cargo.lock` so every checkout resolves the same framework
commit.

## 2. Add a JSON route

Replace `src/main.rs` with:

```rust
use rullst::{Server, ServerError, routes, server::Json};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    framework: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        framework: "Rullst",
    })
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let app = routes![
        get("/api/health" => health),
    ]
    .layer(rullst::server::from_fn(
        rullst::security::headers_middleware,
    ));

    Server::new(app).run(3000).await
}
```

The response is serialized from a typed Rust value. The secure-header layer is
included explicitly because transport, proxy, authentication, authorization,
CSRF, and application-specific input policy remain deployment responsibilities;
a JSON response alone is not a production security boundary.

## 3. Run and verify it

Start the application:

```bash
cargo run
```

From another terminal, request the route:

```bash
curl -i http://127.0.0.1:3000/api/health
```

The body is:

```json
{"status":"ok","framework":"Rullst"}
```

Stop the server with `Ctrl+C`.

## 4. Generate the same starting point with the CLI

The v12 CLI can scaffold a headless API directly:

```bash
cargo rullst new first_rullst_api --default --api --database sqlite \
  --skip-initial-migration
cd first_rullst_api
cargo run
```

Use `cargo rullst make:controller project --api` for additional JSON
controllers. Generated parameterized data routes still require explicit
ownership or tenant authorization; see [RBAC and IDOR protection](13-rbac-authorization.md).
