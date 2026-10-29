# Tutorial 24: Interactive Scalar API Documentation 📖

Scaffold a Scalar OpenAPI documentation router served at `/docs`.

---

## 🛠️ Step 1: Scaffold Scalar Docs

```bash
cargo rullst make:scalar
```

Run it in the project root. It generates `src/controllers/docs_controller.rs`
and declares it in `src/controllers/mod.rs` (creating both when they are
missing) and refuses to overwrite an existing controller. The command does not
edit route registration: the application must merge the router returned by
`docs_controller::router()`.

---

## 💻 Step 2: Mount in Application

`docs_controller::router()` returns `scalar_docs_router("/openapi.json")`. This
self-contained example mounts the same router:

```rust,no_run
use rullst::{Router, Server};
use rullst::scalar::scalar_docs_router;

#[rullst::runtime::main]
async fn main() -> Result<(), rullst::ServerError> {
    let app = Router::new().merge_axum(scalar_docs_router("/openapi.json"));

    Server::new(app).run(3000).await
}
```

In a generated project, declare `mod controllers;` in `src/main.rs` if it is not
there yet (the Blank and API starters have no `controllers` module), and merge
the router where `main` builds the server; in the Blank and API starters that
is `Server::new(router().merge_axum(controllers::docs_controller::router()))`.
Merge it there rather than inside `router()`: each starter's `router()` adds
`headers_middleware`, whose default Content Security Policy blocks the page's
remote and inline scripts, which leaves `/docs` blank.

---

## 📄 Step 3: Generate the specification and open the page

```bash
cargo rullst generate:openapi
cargo rullst dev
```

`generate:openapi` scans the project's routes and writes `openapi.json` in the
project root. Open `http://localhost:3000/docs` in your browser to try the
endpoints interactively.

---

## 💡 Key Takeaways
- The current page loads a version-pinned Scalar asset from jsDelivr. A failed
  CDN load shows only a status page naming the OpenAPI document; it is not an
  offline interactive UI.
- The status-only fallback prints the configured OpenAPI location as text; it
  does not create an executable link from the configured value.
- In staging and production `rullst::Server` applies the same security
  headers to every route, so the default CSP blocks the remote and inline
  assets there. Vendor the asset and integrate the page with the application's
  nonce/hash policy before using it outside local development.
- The router serves `/openapi.json` from the `openapi.json` file in the
  process working directory (`cargo rullst generate:openapi` writes it there);
  if it is missing or malformed, the endpoint fails with
  `503 Service Unavailable` instead of fabricating an empty specification.
  Validate the release artifact in CI.
