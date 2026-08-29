# Tutorial 30: Framework Escape Hatch (`cargo rullst eject`) 🔓

Reduce framework lock-in by generating an inspectable Axum/Tokio starting point. Review the generated snapshot and run `cargo check`; not every optional subsystem can be mechanically ejected.

---

## 🛠️ Step 1: Eject Framework Abstractions

Execute the ejection command:

```bash
cargo rullst eject
```

To overwrite `src/main.rs` directly:
```bash
cargo rullst eject --force
```

---

## 💻 Step 2: What Ejection Does

The ejection tool statically expands Rullst macros, route tables, and server initializations into pure standard Rust code inside `src/ejected_main.rs`:

```rust
use axum::{routing::get, Router};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(|| async { "Pure Axum Application" }));

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

---

## 💡 Key Takeaways
- A migration aid that reduces framework coupling; generated output and remaining dependencies must be reviewed.
- The current generator emits an Axum/Tokio entry point. Optional Rullst
  subsystems and application code may still require a manual migration.
