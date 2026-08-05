# Tutorial 08: Controllers, Routing & Middlewares 🚦

Learn how to structure sub-routers, apply custom Tower/Axum middlewares, and organize large Rullst applications.

---

## 🛠️ Step 1: Create a Custom Middleware

In `src/middlewares/logger.rs`:

```rust
use axum::{
    http::Request,
    middleware::Next,
    response::Response,
};

pub async fn log_request<B>(req: Request<B>, next: Next<B>) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    println!("👉 Incoming Request: {} {}", method, uri);
    
    let res = next.run(req).await;
    
    println!("👈 Response Status: {}", res.status());
    res
}
```

---

## 💻 Step 2: Organize Sub-Routers in `src/main.rs`

```rust
use axum::{routing::{get, post}, middleware};
use rullst::Server;
use crate::controllers::{users_controller, auth_controller};
use crate::middlewares::logger::log_request;

#[tokio::main]
async fn main() {
    let api_routes = axum::Router::new()
        .route("/users", get(users_controller::index))
        .route("/users", post(users_controller::create))
        .layer(middleware::from_fn(log_request));

    let auth_routes = axum::Router::new()
        .route("/login", post(auth_controller::login))
        .route("/signup", post(auth_controller::signup));

    Server::new()
        .nest("/api/v1", api_routes)
        .nest("/auth", auth_routes)
        .run()
        .await;
}
```

---

## 💡 Key Takeaways
- Use `.nest("/prefix", router)` to structure API versioning and resource scopes.
- Custom middlewares can intercept requests, inject headers, or check permissions before reaching controllers.
