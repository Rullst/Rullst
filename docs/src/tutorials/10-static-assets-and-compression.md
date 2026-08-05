# Tutorial 10: Static Assets & Pre-Compression 📦

Learn how Rullst serves static assets (CSS, JS, images) with built-in Brotli and Zstandard pre-compression for production builds.

---

## 🛠️ Step 1: Configure Static Asset Directory

Place your static files inside the `public/` or `static/` directory:

```
public/
├── css/
│   └── app.css
├── js/
│   └── app.js
└── favicon.ico
```

---

## 💻 Step 2: Mount Static Folder in `main.rs`

```rust
use tower_http::services::ServeDir;
use rullst::Server;

#[tokio::main]
async fn main() {
    Server::new()
        .nest_service("/public", ServeDir::new("public"))
        .run()
        .await;
}
```

---

## 🚀 Step 3: Production Build Compression

When building for production:

```bash
cargo rullst build
```

Rullst automatically runs Brotli and Zstandard compression passes on your static files, generating `.br` and `.zst` sidecars so web servers serve compressed assets instantly with zero CPU overhead.

---

## 💡 Key Takeaways
- Use `ServeDir` to serve images, CSS, and favicon files.
- `cargo rullst build` pre-compresses static assets for sub-millisecond edge delivery.
