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
unchanged. The generated server contains only a demonstration root route:

```rust,no_run
use axum::{routing::get, Router};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(|| async { "Ejected Axum server" }));
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

Move application routes one bounded group at a time and retain equivalent
security headers, CSRF/CORS/WAF order, limits, telemetry, graceful shutdown,
health behavior, state, and authorization tests.

---

## Step 2: Treat `--force` as a deliberate replacement

```bash
cargo rullst eject --force
```

The hardened command first preserves the original entry point as
`src/main.rs.rullst-backup` and refuses to overwrite an existing backup. Keep a
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
