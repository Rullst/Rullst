# Tutorial 30: Axum Escape-Hatch Snapshot (`cargo rullst eject`) 🔓

`eject` generates a minimal Axum/Tokio entry point that can begin a manual
migration away from Rullst's server wrapper. It does **not** statically expand
macros, copy the application's route graph, convert middleware, or remove Rullst
dependencies automatically.

---

## Step 1: Generate a separate starting point

```bash
cargo rullst eject
```

The default output is `src/ejected_main.rs`; the existing `src/main.rs` remains
unchanged. The generated template serves a placeholder route from
`application_routes()` and wraps it in the configured Rullst security baseline,
the same `apply_security_baseline` step `rullst::Server` performs with
`Rullst.toml`. Axum is reached through the `rullst::web::axum` re-export, so the
template compiles without adding Axum to `Cargo.toml`:

```rust,no_run
use rullst::RullstConfig;
use rullst::web::axum::{Router, response::Html, routing::get};
use std::net::SocketAddr;

/// Mount the application's routes here.
fn application_routes() -> Router {
    Router::new().route("/", get(|| async { Html("<h1>Ejected Axum Server</h1>") }))
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = if std::path::Path::new("Rullst.toml").exists() {
        RullstConfig::load_from_file("Rullst.toml").await?
    } else {
        RullstConfig::default()
    };
    let environment = config.environment()?;
    let app = rullst::apply_security_baseline(application_routes(), config.security, environment)?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = rullst::async_runtime::tokio::net::TcpListener::bind(addr).await?;
    rullst::web::axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
```

The baseline covers secure headers, CORS, WAF/RASP and CSRF (headers, WAF and
CSRF in staging/production). Static files, rate limiting, lifecycle probes, hot
reload, sessions, authentication and authorization are not included.

Move application routes into `application_routes()` one bounded group at a
time and retain equivalent limits, telemetry, graceful shutdown, health
behavior, state, and authorization tests.

---

## Step 2: Treat `--force` as a deliberate replacement

```bash
cargo rullst eject --force
```

The hardened command first preserves the original entry point as
`src/main.rs.rullst-backup` and refuses to overwrite an existing backup. The
template does not carry over the original routes or module declarations; move
them from the backup before building or deploying. Keep a
normal version-control commit as the authoritative recovery path. Custom output
paths are restricted to relative Rust files under `src/` and existing targets
are not overwritten implicitly.

---

## Key takeaways

- Ejection is a migration aid, not a semantics-preserving compiler transform.
- The application remains responsible for dependency cleanup and replacements
  for ORM, auth, queues, Studio, Nexus, Capital, AI, and other selected crates.
- Run `cargo fmt`, strict Clippy, the complete test suite, and application
  security/operational checks after every migrated route group.
