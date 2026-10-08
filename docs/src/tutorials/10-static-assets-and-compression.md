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

The server serves an existing sibling without comparing it with its source, so
an asset edited after the last build would keep reaching browsers that accept
`br` or `zstd` in its old form. `cargo rullst dev` therefore deletes every
`.br`/`.zst` sibling that is not newer than its source when it starts and before
each rebuild, and reports how many it removed. A sibling without a source file
is kept, because it may be hand-made. Outside `cargo rullst dev`, rerun
`cargo rullst build` after editing assets and before `docker build` copies
`static/` into an image.

The Zstandard middleware serves `name.zst` only when `Accept-Encoding` lists
`zstd` with a quality above zero (`zstd;q=0` is a refusal) and the path below
`/static/` has only plain segments. Paths with `.`, `..`, empty segments,
backslashes or percent-encoding skip the `.zst` lookup and go to `ServeDir`
uncompressed. `Content-Encoding: zstd` is added only to `2xx` and `304`
responses. A hand-made `.zst` is used only for a type the middleware can label
(the generated types plus `.htm`, `.mjs`, `.cjs`, `.map`, `.webmanifest`,
`.csv`, `.pdf`, `.ttf` and `.otf`); for any other file the uncompressed asset
is served, never a `.zst` body typed `application/octet-stream`.

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

## Step 3: Cache headers

Since v13 the standard static mount sets `Cache-Control` itself (Rullst does
not rename files; your asset tool does):

| Request path below `/static/` | `Cache-Control` |
| :--- | :--- |
| Fingerprinted name: a `.`- or `-`-separated segment of 8–64 lowercase hex characters, with at least one digit and one letter, after the first part and before the extension (`app.3f9a2c1b.css`, `chunk-3f9a2c1b.js`, `app.3f9a2c1b.js.map`) | `public, max-age=31536000, immutable` |
| Any other file (`app.css`, `report-20261008.pdf`) | `no-cache` |

`no-cache` lets browsers and caches store the file but revalidate it before
each use. `ServeDir` sends `ETag` and `Last-Modified` and answers a matching
`If-None-Match` or `If-Modified-Since` with `304 Not Modified` and no body.
The policy is set on `2xx` and `304` responses only, for every encoding
(`br`, `zstd` or none). A missing file and every dynamic response keep the
production security baseline's `Cache-Control: no-store`.

An immutable file is never asked for again until it expires, so give an asset
a new hashed name whenever its content changes, and do not hand-name a mutable
file with a hash-like segment. Check the headers through your proxy or CDN:

```bash
curl -sI http://127.0.0.1:3000/static/css/app.css | grep -i -E 'cache-control|etag'
curl -sI -H 'If-None-Match: "<etag from above>"' \
  http://127.0.0.1:3000/static/css/app.css   # HTTP/1.1 304 Not Modified
```

---

## Key takeaways

- Use `static/` for the framework's standard asset path and build integration.
- Keep source files alongside generated sidecars in the deployed artifact.
- Content-hashed names are cached for a year as `immutable`; other static files
  are revalidated with `ETag`/`Last-Modified`.
