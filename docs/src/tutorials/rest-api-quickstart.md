# Create Your First Rullst REST API

> 🇧🇷 [Leia esta página em português](../pt-BR/tutorials/rest-api-quickstart.md).

**Your result:** a typed JSON response you can verify from another terminal.
Start here when your frontend is a separate app, mobile client or integration.
[Explore all beginner paths](../start-here.md) or the
[complete API learning-path index](../api-reference.md#httprest-a-connected-learning-path).

This quickstart creates one runnable JSON endpoint without a database or an AI
provider. It targets the v13 prerelease `13.0.0-rc.1`; the v12 line reached
end of life (see the [release record](../v12.md)). You need Rust 1.96.0 or
newer, and the latest stable Rust is recommended
([Tutorial 01](01-hello-world.md#1-install-rust-and-cargo) shows how to
install it).

## 1. Create the application

<!-- first-hour: create -->
```bash
cargo new first_rullst_api
cd first_rullst_api
cargo add rullst@=13.0.0-rc.1
cargo add tokio --features macros,rt-multi-thread
cargo add serde --features derive
```

The exact requirement keeps this tutorial on the v13 prerelease (Cargo never
selects a prerelease on its own, and the v12 line reached end of life on
16 October 2026). Commit the generated
`Cargo.lock` so subsequent locked builds use the same resolved package versions;
it does not pin an unrelated moving Git branch or automatically update the app.

## 2. Add a JSON route

Replace `src/main.rs` with:

<!-- first-hour: main write=src/main.rs -->
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
    ];

    Server::new(app).run(3000).await
}
```

The response is serialized from a typed Rust value. `Server` adds the secure
headers and the CSRF check in every environment; transport, proxy,
authentication, authorization and application-specific input policy remain
deployment responsibilities, so a JSON response alone is not a production
security boundary.

## 3. Run and verify it

Start the application:

<!-- first-hour: run -->
```bash
cargo run
```

From another terminal, request the route:

<!-- first-hour: request -->
```bash
curl -i http://127.0.0.1:3000/api/health
```

The body is:

<!-- first-hour: body output -->
```json
{"status":"ok","framework":"Rullst"}
```

Stop the server with `Ctrl+C`.

## 4. Prefer a generated API foundation?

After [installing the v13 CLI](../1-getting-started.md#1-installation), it can
scaffold a headless API directly. Use a different directory from the
hand-written example above:

<!-- first-hour: generated new-terminal -->
```bash
cargo rullst new generated_api --default --api --no-database \
  --skip-initial-migration
cd generated_api
cargo run
```

This is the API starter (the Blank starter without HTML). Its routes are
registered in `router()` in `src/main.rs`, not copied from the health handler
above: `GET /` answers with a JSON welcome message (`curl http://127.0.0.1:3000/`),
`POST /api/messages` requires `Authorization: Bearer <API_TOKEN>` with the
random token written to `.env`, and `/health` and `/ready` serve container
probes. Use `cargo rullst make:controller project --api` for additional JSON
controllers. Generated parameterized data routes still require explicit
ownership or tenant authorization; see [RBAC and IDOR protection](13-rbac-authorization.md).
