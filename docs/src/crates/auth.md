# Rullst Auth 🛡️

`rullst-auth` is the official, enterprise-grade authentication and authorization module for the Rullst Framework. It adheres to the framework's strict **Zero-Panic Policy** and is designed to handle the most complex parts of user identity and session management securely.

## ✨ Features

- **Zero-Panic Guarantees:** 100% safe Rust. No unexpected crashes during token decoding or cryptography.
- **Session Management:** Secure, signed, encrypted, and HTTP-only cookie management built-in.
- **JWT Support:** Issue, sign, and verify JSON Web Tokens (JWT) for stateless API authentication.
- **OAuth2 / OIDC Integration:** Ready-to-use providers for Google, GitHub, Apple, and generic OpenID Connect workflows.
- **Role-Based Access Control (RBAC):** Elegant middleware for controller-level permissions and capability matrices.
- **Password Hashing:** Native integration with Argon2id for secure credential storage.
- **WebAuthn / Passkeys:** Future-proof, passwordless authentication support.

## 🚀 Quickstart

Add `rullst-auth` to your project:

```bash
cargo add rullst-auth
```

### Enabling the Middleware

In your Rullst application, enable the authentication layer by providing a `SessionStore` (e.g., Redis or PostgreSQL):

```rust
use rullst::{Router, routing::get};
use rullst_auth::{AuthLayer, SessionStore};
use rullst_orm::Orm;

#[tokio::main]
async fn main() {
    let pool = Orm::pool();
    let session_store = SessionStore::postgres(pool);

    let app = Router::new()
        .route("/dashboard", get(dashboard_handler))
        // Protect routes below this layer
        .layer(AuthLayer::new(session_store));
        
    // ... start server ...
}
```

### Hashing Passwords (Argon2id)

In async HTTP request handlers, prefer the non-blocking `_async` variants which offload CPU-bound Argon2id operations to Tokio's blocking thread pool (`spawn_blocking`):

```rust
use rullst_auth::{hash_password_async, verify_password_async};

// In async handlers / endpoints:
let password = "super_secure_password";
let hash = hash_password_async(password).await.expect("Hashing should not fail");

let is_valid = verify_password_async(password, &hash).await;
assert!(is_valid);
```

For offline scripts or CLI tasks, synchronous functions (`hash_password`, `verify_password`) are also available.

## 🔐 Security Audit

`rullst-auth` relies on high-quality cryptographic primitives (e.g., `argon2`, `rsa`, `hkdf`). It is continuously fuzzed and verified against side-channel timing attacks. All cryptographic operations return a typed `AuthError` on failure rather than panicking.

## 📚 Documentation

For advanced usage, including JWT custom claims, OAuth2 workflows, and RBAC matrix configuration, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
