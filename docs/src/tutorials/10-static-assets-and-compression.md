# Tutorial 10: Static Assets & Pre-Compression 📦

The standard `Server` serves an existing `static/` directory at `/static`.
Production builds can create Brotli and Zstandard sidecars for eligible text and
Wasm assets in that directory.

---

## Step 1: Use the standard static directory

```text
static/
├── css/
│   └── app.css
├── js/
│   └── app.js
└── favicon.svg
```

Reference those files through `/static/...`, for example
`/static/css/app.css`. `Server::run` mounts the directory when it exists; no
additional `ServeDir` layer is required for this standard path.

```rust,no_run
use rullst::{routes, routing::get, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![get("/" => || async { "Rullst" })];
    Server::new(app).run(3000).await?;
    Ok(())
}
```

If you mount a different directory manually through Axum/Tower, its routing and
pre-compressed negotiation become application responsibilities.

---

## Step 2: Build production sidecars

```bash
cargo rullst build
```

The release-mode command builds the application and creates `.br` and `.zst`
siblings for `html`, `css`, `js`, `json`, `svg`, `wasm`, `xml`, and `txt` files
under `static/`. The standard server negotiates Brotli through `ServeDir` and
Zstandard through its static middleware.

Verify deployed behavior rather than assuming negotiation worked:

```bash
curl --compressed -I -H 'Accept-Encoding: br' \
  http://127.0.0.1:3000/static/css/app.css
curl -I -H 'Accept-Encoding: zstd' \
  http://127.0.0.1:3000/static/css/app.css
```

Check `Content-Encoding`, `Content-Type`, cache headers, and `Vary` through the
actual TLS proxy/CDN. Pre-compression avoids compression work per request; it
does not eliminate file I/O or network latency.

---

## Key takeaways

- Use `static/` for the framework's standard asset path and build integration.
- Keep source files alongside generated sidecars in the deployed artifact.
- Fingerprint immutable asset names and configure cache policy at the
  application/CDN boundary.
