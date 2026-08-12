# 🧪 Monorepo Examples & Reference Applications

Rullst includes full-stack reference applications located in the [`examples/`](https://github.com/Rullst/Rullst/tree/main/examples) directory of the monorepo.

These applications serve as **live integration testbeds, CI/CD validation suites, and architectural references** demonstrating how all Rullst subsystems interact in a single production-ready codebase.

---

## 🆚 Monorepo Examples vs. CLI Blueprints

It is important to distinguish between **Monorepo Examples** (`examples/`) and **CLI Blueprints** (`cargo-rullst`):

| Aspect | Monorepo Examples (`examples/blog`) | CLI Scaffolding Blueprints (`cargo rullst new ... --blueprint blog`) |
| :--- | :--- | :--- |
| **Purpose** | Framework-level integration testing, CI/CD verification (DAST/E2E), and architectural showcase. | Starter boilerplate for developers creating new web applications. |
| **Dependencies** | Local path dependencies (`path = "../../rullst"`). | Public registry dependencies (`rullst = "12.0.0"` from crates.io). |
| **Architecture** | Compact, single-binary showcase with Wasm islands, WebSockets, multi-tenancy, and SQLite. | Modular MVC structure (`controllers/`, `models/`, `migrations/`, `pages/`, `Nexus CMS`). |
| **Database** | Embedded SQLite without external database requirements. | Configurable: PostgreSQL, MySQL, or SQLite with connection pooling. |
| **Frontend** | Pure Rust SSR with `html!` macro + LiveView + Wasm Islands. | User-selected engine: HTMX + Tailwind, React/Vite, Vue, Svelte, Leptos, etc. |

---

## 📖 `rullst-blog-example` Deep-Dive

The primary showcase application is located at `examples/blog`.

### Key Subsystems Demonstrated:

### 1. Active Record ORM with SQLite
```rust
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "posts", global_scope = "apply_tenant_scope")]
pub struct Post {
    pub id: i32,
    pub tenant_id: String,
    pub title: String,
    pub body: String,
}
```

### 2. Multi-Tenant Database Isolation
Tenant scoping is applied automatically to all Active Record operations:
```rust
impl PostQueryBuilder {
    pub fn apply_tenant_scope(self) -> Self {
        if let Some(tid) = rullst::multitenant::current_tenant_id() {
            self.where_eq("tenant_id", tid)
        } else {
            self
        }
    }
}
```

### 3. Real-Time WebSockets (`rullst::live`)
Interactive state synchronization without client-side JavaScript frameworks:
- Route: `/live-counter`
- Component: `CounterComponent` handling `increment` and `decrement` events over a persistent WebSocket connection.

### 4. Wasm Islands in Pure Rust
Micro-frontend hydration where Rust compiles directly to WebAssembly (`wasm32-unknown-unknown`):
- Route: `/wasm-counter`
- Client-side reactivity executed inside the browser DOM using `wasm-bindgen`.

---

## 🚀 Running Examples Locally

```bash
# Clone the repository
git clone https://github.com/Rullst/Rullst.git
cd Rullst/examples/blog

# Start the server
cargo run
```

Access the application at `http://127.0.0.1:3000`.

---

## 🛡️ CI/CD Integration

The example application is continuously tested on every commit:
- **E2E Smoke Suite**: [`.github/workflows/e2e-smoke.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/e2e-smoke.yml)
- **OWASP DAST ZAP Security Scan**: [`.github/workflows/dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml)
