# Rullst Auth

`rullst-auth` handles the most complex parts of user authentication, authorization, and session management for you.

## Features
- **Session Management:** Secure, signed, and encrypted cookies.
- **JWT Support:** Issue and verify JWTs for stateless API authentication.
- **OAuth2 Integration:** Ready-to-use providers for Google, GitHub, and more.
- **Role-Based Access Control (RBAC):** Middleware for controller-level permissions.
- **WebAuthn / Passkeys:** Future-proof authentication support.

## Usage

In your Rullst application, enable auth middleware:

```rust
use rullst_auth::{AuthLayer, SessionStore};

let app = Router::new()
    .route("/dashboard", get(dashboard_handler))
    .layer(AuthLayer::new(SessionStore::redis(redis_pool)));
```
