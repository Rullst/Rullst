# Tutorial 12: JWT vs Session Authentication 🔑

Learn how to configure JWT token authentication for REST APIs / Mobile apps and HTTP-only Cookie Sessions for web applications in Rullst.

---

## 🛠️ Step 1: Generate JWT Middleware

```bash
cargo rullst make:jwt
```

This creates `src/middlewares/jwt_auth.rs`.

---

## 💻 Step 2: Protecting Routes with JWT

```rust
use axum::{Router, routing::get, middleware};
use crate::middlewares::jwt_auth::verify_jwt;

pub fn protected_routes() -> Router {
    Router::new()
        .route("/profile", get(user_profile))
        .route("/orders", get(user_orders))
        .layer(middleware::from_fn(verify_jwt))
}
```

---

## 💡 Key Takeaways
- Use **Session Cookies** for traditional web applications rendered on the server (prevents XSS token theft).
- Use **JWT Tokens** (`Bearer <token>`) for mobile applications and headless REST APIs.
