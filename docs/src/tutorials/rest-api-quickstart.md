# Create Your First Rullst REST API

**Your result:** a typed JSON response you can verify from another terminal.
Start here when your frontend is a separate app, mobile client or integration.
[Explore all beginner paths](../start-here.md).

This quickstart creates one runnable JSON endpoint without a database or an AI
provider. It targets the unreleased v12 snapshot on `main`, for local evaluation
only. Production adoption needs a supported release and reviewed immutable
artifacts; pinning end-of-life v5 does not make it supported again.

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

```rust,no_run
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

## 4. Prefer a generated API foundation?

After [installing the matching v12 CLI](../1-getting-started.md), it can scaffold
a headless API directly. Use a different directory from the hand-written
example above:

```bash
cargo rullst new generated_api --default --api --no-database \
  --skip-initial-migration
cd generated_api
cargo run
```

This generates the blueprint's own routes, not an exact copy of the health
handler above. Read its controller and route registration before choosing a
URL to test. Use `cargo rullst make:controller project --api` for additional JSON
controllers. Generated parameterized data routes still require explicit
ownership or tenant authorization; see [RBAC and IDOR protection](13-rbac-authorization.md).
