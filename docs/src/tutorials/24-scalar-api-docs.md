# Tutorial 24: Interactive Scalar API Documentation 📖

Scaffold a Scalar OpenAPI documentation router served at `/docs`.

---

## 🛠️ Step 1: Scaffold Scalar Docs

```bash
cargo rullst make:scalar
```

This generates `src/controllers/docs_controller.rs`. The application must merge
the returned router; the command does not edit route registration automatically.

---

## 💻 Step 2: Mount in Application

```rust
use rullst::{Router, Server};
use rullst_core::scalar::scalar_docs_router;

#[tokio::main]
async fn main() -> Result<(), rullst::ServerError> {
    let app = Router::new().merge_axum(scalar_docs_router("/openapi.json"));

    Server::new(app).run(3000).await
}
```

Open `http://localhost:3000/docs` in your browser to test endpoints interactively!

---

## 💡 Key Takeaways
- The current page loads a version-pinned Scalar asset from jsDelivr. A failed
  CDN load shows only a link to the OpenAPI JSON; it is not an offline
  interactive UI.
- The status-only fallback prints the configured OpenAPI location as text; it
  does not create an executable link from the configured value.
- A strict CSP may block the remote and inline assets. Vendor the asset and
  integrate the page with the application's nonce/hash policy before using it
  outside local development.
- The router reads `openapi.json`; if it is missing or malformed, the endpoint
  fails with `503 Service Unavailable` instead of fabricating an empty
  specification. Validate the release artifact in CI.
