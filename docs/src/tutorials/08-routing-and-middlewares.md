# Tutorial 08: Controllers, Routing & Middleware 🚦

Rullst's `Router` wraps Axum's router while preserving explicit Tower
middleware composition. This example uses the Axum 0.8 request and `Next` types.

---

## Step 1: Create a custom middleware

Generate the file with the CLI:

```bash
cargo rullst make:middleware Logger
```

It writes `src/middlewares/logger_middleware.rs` with an empty
`logger_middleware` function and registers the module. Fill in the body, for
example:

```rust
use rullst::web::axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn logger_middleware(req: Request, next: Next) -> Response {
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

// Stand-in for the generated `logger_middleware` of Step 1.
async fn logger_middleware(
    request: rullst::web::axum::extract::Request,
    next: rullst::web::axum::middleware::Next,
) -> rullst::web::axum::response::Response {
    next.run(request).await
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let api = Router::new()
        .route("/users", get(list_users).post(create_user))
        .layer(middleware::from_fn(logger_middleware));

    let auth = Router::new().route("/login", post(login));
    let app = Router::new()
        .nest("/api/v1", api)
        .nest("/auth", auth);

    Server::new(app).run(3000).await
}
```

In the application, import the generated function
(`use crate::middlewares::logger_middleware::logger_middleware;`) and pass it to
`from_fn` the same way. A layer covers only the routes added before it. Use
`nest_axum` or `merge_axum` when integrating a third-party raw
`axum::Router`.

---

## Step 3: Share application state

`rullst::Router` wraps an `axum::Router<()>`, whose handlers cannot take
`State<S>`, so it has no `with_state`. Pass shared state in one of two ways:

- **As a request extension.** `.layer(Extension(state))` inserts the value into
  every request, and handlers read it with `Extension<T>`. Rullst's own
  middleware passes the CSRF token, the CSP nonce and the signed-in user id
  this way. Like every layer, it covers only the routes added before it.
- **With Axum's `State`.** Build the routes that use `State<S>` as an
  `axum::Router<S>`, call its `with_state`, and mount the result with
  `merge_axum` (or `nest_axum` under a prefix).

```rust,no_run
use rullst::{Router, Server, ServerError};
use rullst::routing::get;
use rullst::web::axum::{self, Extension, extract::State};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    site_name: Arc<str>,
}

async fn home(Extension(state): Extension<AppState>) -> String {
    format!("Welcome to {}", state.site_name)
}

async fn about(State(state): State<AppState>) -> String {
    format!("About {}", state.site_name)
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let state = AppState { site_name: Arc::from("Rullst") };
    let stateful = axum::Router::new()
        .route("/about", get(about))
        .with_state(state.clone());
    let app = Router::new()
        .route("/", get(home))
        .layer(Extension(state))
        .merge_axum(stateful);

    Server::new(app).run(3000).await
}
```

Keep values in the state cheap to clone (`Arc`, connection pools): each request
clones it.

---

## Key takeaways

- Middleware order is security-sensitive. Use the canonical production baseline
  for secure headers, CSRF, CORS, and WAF rather than assembling those controls
  ad hoc.
- Authentication middleware establishes identity; handlers/repositories must
  still enforce resource ownership or role authorization.
- Apply request-body and concurrency limits before handlers that parse expensive
  or attacker-controlled input.
