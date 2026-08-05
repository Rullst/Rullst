# Tutorial 24: Interactive Scalar API Documentation 📖

Scaffold zero-config Scalar OpenAPI interactive documentation playground served at `/docs`.

---

## 🛠️ Step 1: Scaffold Scalar Docs

```bash
cargo rullst make:scalar
```

This generates `src/controllers/docs_controller.rs` and mounts the `/docs` route.

---

## 💻 Step 2: Mount in Application

```rust
use rullst_core::scalar::scalar_docs_router;

#[tokio::main]
async fn main() {
    let app = axum::Router::new()
        .merge(scalar_docs_router("/openapi.json"));

    rullst::Server::new().merge(app).run().await;
}
```

Open `http://localhost:3000/docs` in your browser to test endpoints interactively!

---

## 💡 Key Takeaways
- Modern alternative to classic Swagger UI.
- CDN assets loaded dynamically with automatic offline fallback.
