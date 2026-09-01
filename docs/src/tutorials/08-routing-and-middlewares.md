# Tutorial 08: Controllers, Routing & Middleware 🚦

Rullst's `Router` wraps Axum's router while preserving explicit Tower
middleware composition. This example uses the Axum 0.8 request and `Next` types.

---

## Step 1: Create a custom middleware

In `src/middlewares/logger.rs`:

```rust
use rullst::web::axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn log_request(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let response = next.run(req).await;

    // Prefer structured tracing in production; do not log query strings,
    // cookies, authorization headers, or request bodies by default.
    println!("[HTTP] {method} {path} -> {}", response.status());
    response
}
```

---

## Step 2: Organize sub-routers

```rust,no_run
use rullst::{Router, Server, ServerError};
use rullst::routing::{get, post};
use rullst::web::axum::middleware;

async fn list_users() -> &'static str { "users" }
async fn create_user() -> &'static str { "created" }
async fn login() -> &'static str { "login" }

async fn log_request(
    request: rullst::web::axum::extract::Request,
    next: rullst::web::axum::middleware::Next,
) -> rullst::web::axum::response::Response {
    next.run(request).await
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let api = Router::new()
        .route("/users", get(list_users).post(create_user))
        .layer(middleware::from_fn(log_request));

    let auth = Router::new().route("/login", post(login));
    let app = Router::new()
        .nest("/api/v1", api)
        .nest("/auth", auth);

    Server::new(app).run(3000).await
}
```

Use `nest_axum` or `merge_axum` when integrating a third-party raw
`axum::Router`.

---

## Key takeaways

- Middleware order is security-sensitive. Use the canonical production baseline
  for secure headers, CSRF, CORS, and WAF rather than assembling those controls
  ad hoc.
- Authentication middleware establishes identity; handlers/repositories must
  still enforce resource ownership or role authorization.
- Apply request-body and concurrency limits before handlers that parse expensive
  or attacker-controlled input.
