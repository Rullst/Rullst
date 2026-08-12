# Rullst Dev Blog Example 📖

This directory contains `rullst-blog-example`, the **official full-stack reference and integration test application** for the Rullst Framework.

It is designed as a self-contained showcase demonstrating how all core subsystems of Rullst integrate seamlessly into a single production-grade Rust application.

---

## 🌟 Features Demonstrated

1. **Active Record ORM & SQLite**:
   - Zero-boilerplate data models using `#[derive(rullst_orm::Orm)]`.
   - Dynamic query building (`Post::all()`, `post.save()`).
2. **Multi-Tenancy Isolation**:
   - Automated global tenant scoping via `apply_tenant_scope` and `rullst::multitenant::current_tenant_id()`.
   - Scoped data access based on the `X-Tenant-ID` HTTP header.
3. **Server-Side Rendering (SSR) & `html!` Macro**:
   - Zero-bundle JSX-like server-side rendering with full compile-time syntax validation.
4. **Reactive WebSocket Live Components**:
   - Server-driven UI state synchronization over WebSockets via `rullst::live::Live`.
5. **Wasm Islands Hydration**:
   - Micro-frontend interactive client-side components compiled from Rust to WebAssembly (`wasm32-unknown-unknown`).
6. **Built-in Security & WAF**:
   - OWASP Secure Headers (CSP, HSTS, X-Frame-Options, X-Content-Type-Options: nosniff).
   - Double-Submit CSRF cookie protection and Web Application Firewall (WAF) threat inspection.
7. **CI/CD Integration Testbed**:
   - Powers automated End-to-End (E2E) smoke tests and OWASP DAST ZAP dynamic security scanning in GitHub Actions.

---

## 🚀 Getting Started

### 1. Run the Blog Locally

From this directory (`examples/blog/`):

```bash
cargo run
```

Or from the workspace root:

```bash
cargo run -p rullst-blog-example
```

The server will start at `http://127.0.0.1:3000`.

---

## 🧭 Route Catalog & Interactive Demos

| Route | Method | Description |
| :--- | :--- | :--- |
| `http://localhost:3000/` | `GET` | **Main Blog Feed**: Server-side rendered feed of posts and creation form. |
| `http://localhost:3000/posts` | `POST` | **Publish Post**: Saves a new post into `blog.db` with CSRF protection. |
| `http://localhost:3000/live-counter` | `GET` | **WebSocket Live Component**: Server-driven state updates in real time. |
| `http://localhost:3000/wasm-counter` | `GET` | **Wasm Island**: Client-side interactive widget running pure Rust Wasm. |
| `http://localhost:3000/robots.txt` | `GET` | **SEO Crawler Control**: Auto-generated `robots.txt`. |
| `http://localhost:3000/sitemap.xml` | `GET` | **XML Sitemap**: Auto-generated dynamic SEO sitemap. |

---

## 🏢 Testing Multi-Tenancy

The blog includes automated multi-tenant database isolation. By default, the database is seeded with tenant-specific posts.

### Requesting Tenant 1:
```bash
curl -s -A "Mozilla/5.0" -H "X-Tenant-ID: tenant1" http://localhost:3000/ | grep -i "Story of Tenant 1"
```

### Requesting Tenant 2:
```bash
curl -s -A "Mozilla/5.0" -H "X-Tenant-ID: tenant2" http://localhost:3000/ | grep -i "Exclusive for Tenant 2"
```

---

## 🧪 Automated Testing & CI/CD

This example is verified in GitHub Actions via:
- [`.github/workflows/e2e-smoke.yml`](../../.github/workflows/e2e-smoke.yml): Builds release binary, starts background server, tests HTTP 200, CSP security headers, CSRF token generation, and SQLite writes.
- [`.github/workflows/dast-zap.yml`](../../.github/workflows/dast-zap.yml): Runs OWASP ZAP dynamic vulnerability scanner against the live running instance.
