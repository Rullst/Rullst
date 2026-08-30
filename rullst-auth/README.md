# Rullst Auth

`rullst-auth` provides Argon2id password hashing, versioned AES-GCM cookie sessions,
role-based authorization middleware, WebAuthn/passkey ceremony verification, and
an opt-in application JWT policy.

Application JWTs are enabled with `jwt`; OAuth2/OIDC providers remain a separate
trust boundary enabled with `oauth`, which re-exports `rullst-connect`.

## Passwords

Use the asynchronous functions inside HTTP handlers so Argon2 work runs on Tokio's blocking pool:

```rust,no_run
use rullst_auth::{AuthError, hash_password_async, verify_password_async};

async fn verify_login(password: String) -> Result<bool, AuthError> {
    let hash = hash_password_async(password.clone()).await?;
    Ok(verify_password_async(password, hash).await)
}
```

Passwords longer than 72 bytes are rejected. `needs_rehash` compares the algorithm, version,
memory, iteration, and parallelism parameters.

## Encrypted sessions

`make_login_cookie` and `decrypt_session` use a versioned AES-256-GCM envelope with
authenticated metadata and an operating-system nonce. `APP_KEY` must contain at least 32 bytes,
must not be a documented placeholder, and must satisfy the entropy check.

```rust,no_run
use rullst_auth::{AuthError, decrypt_session, get_app_key, make_login_cookie};

fn round_trip(user_id: i32) -> Result<i32, AuthError> {
    let cookie = make_login_cookie(user_id)?;
    let token = cookie
        .split(';')
        .next()
        .and_then(|part| part.split_once('='))
        .map(|(_, value)| value)
        .ok_or_else(|| AuthError::General("session cookie is malformed".to_string()))?;
    decrypt_session(token, &get_app_key()?)
}
```

## WebAuthn/passkeys

`PasskeyAuth` validates exact RP origin and ID binding, one-time expiring challenges,
client-data ceremony type, user-presence/user-verification flags, ES256 COSE keys, P-256 points,
credential IDs, signatures, and monotonic counters. Only `none` attestation is advertised and
accepted. Applications must persist the returned counter atomically with the credential record.

## Application JWTs

The `jwt` feature provides `ApplicationJwtPolicy`, versioned HS256 claims, strong
key validation, required issuer/audience/subject/time/JTI claims, bounded TTL and
scope policy, and `kid`-based key rotation. Every verification receives a
`JwtRevocationStore`. Production policies reject the bundled bounded in-memory
store because it is process-local; applications must supply a shared durable
implementation for cross-instance token/session-version revocation. This API does
not verify third-party OAuth/OIDC tokens.

## RBAC

Implement `HasRole` for the authenticated user type and install
`RequireRoleLayer::<User>::new("Admin")`. Authentication middleware must insert that user into
Axum request extensions before the role layer executes.

Applications using the umbrella crate may alternatively place
`#[rullst::require_role("Admin")]` on an async handler whose authenticated
extractor binds the inner value as `user`:

```rust,no_run
# use rullst::auth::HasRole;
# use rullst::server::Extension;
# #[derive(Clone)] struct User { admin: bool }
# impl HasRole for User { fn has_role(&self, role: &str) -> bool { self.admin && role == "Admin" } }
#[rullst::require_role("Admin")]
async fn admin_dashboard(Extension(user): Extension<User>) -> &'static str {
    "authorized"
}
```

The macro validates its role and handler shape at compile time and returns 403
before the body for a missing role. It does not authenticate the request;
install authentication before either this attribute or `RequireRoleLayer`.

For resource-specific decisions, implement the fail-closed named `Policy`:

```rust
# use rullst_auth::Policy;
# struct User { id: i64, admin: bool }
# struct Post { owner_id: i64 }
struct PostPolicy;

impl Policy<User, Post> for PostPolicy {
    fn can_edit(user: &User, post: &Post) -> bool {
        user.admin || user.id == post.owner_id
    }
}

# let user = User { id: 7, admin: false };
# let post = Post { owner_id: 7 };
assert!(PostPolicy::can_edit(&user, &post));
```

The legacy `Gate<Resource>` implemented directly on the user remains available
for compatibility. Named policy structs are preferred for new application code.

## OAuth2/OIDC

Enable `oauth` for the `rullst_auth::connect` re-export. Provider configuration, discovery,
JWKS rotation, and deterministic offline fixtures are implemented by `rullst-connect`.

Security-sensitive functions return typed errors or a false verification result. The repository's
zero-panic CI checks production library paths; this policy is not an absolute guarantee about all
dependencies or host failures.
