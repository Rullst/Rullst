# Tutorial 12: JWT and Cookie Sessions 🔑

Cookie sessions and bearer JWTs are transport choices with different operational
trade-offs. Neither is automatically “for web” or “for mobile,” and neither
removes the need for TLS, authorization, rotation, expiry, and revocation design.

---

## Step 1: Generate JWT middleware

```bash
cargo rullst make:jwt
```

This creates `src/middlewares/jwt_auth.rs`, registers its module, and adds the
direct `jsonwebtoken`, `chrono`, and `serde` dependencies when missing.

Configure a high-entropy secret plus exact issuer and audience values:

```bash
export JWT_SECRET="$(openssl rand -base64 48)"
export JWT_ISSUER="https://identity.example.test"
export JWT_AUDIENCE="rullst-api"
```

The generated HS256 validator requires `sub`, `iss`, `aud`, `iat`, and `exp`,
checks expiry, and rejects secrets shorter than 32 bytes or with weak character
diversity. Keep the secret out of source control and logs.

---

## Step 2: Protect a route group

```rust,ignore
use rullst::{Router, routing::get};
use rullst::web::axum::middleware;
use crate::middlewares::jwt_auth::jwt_middleware;

pub fn protected_routes() -> Router {
    Router::new()
        .route("/profile", get(user_profile))
        .route("/orders", get(user_orders))
        .layer(middleware::from_fn(jwt_middleware))
}
```

Valid claims are inserted into request extensions. The handler must still map
`sub` to a current account and enforce resource/tenant authorization. For
long-lived systems, design key rotation and immediate revocation rather than
assuming expiry alone is sufficient.

---

## Choosing deliberately

- An `HttpOnly`, `Secure`, appropriately `SameSite` cookie reduces direct token
  reads by browser JavaScript. It does not neutralize XSS: injected code can
  still act as the user, and cookie requests need CSRF protection.
- A bearer token is convenient for interoperable API clients, but storage in a
  browser or native app is an application security decision. A stolen bearer
  token can be replayed until rejected or expired.
- Cookie and JWT strategies can coexist at different boundaries, but keep one
  authoritative identity/session lifecycle and test logout/revocation behavior.
