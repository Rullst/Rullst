# Rullst Auth

`rullst-auth` provides Argon2id password hashing, versioned AES-GCM cookie sessions,
role-based authorization middleware, and WebAuthn/passkey ceremony verification.

The crate does not currently issue application JWTs. OAuth2/OIDC providers are available by
enabling the `oauth` feature, which re-exports `rullst-connect`.

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

## RBAC

Implement `HasRole` for the authenticated user type and install
`RequireRoleLayer::<User>::new("Admin")`. Authentication middleware must insert that user into
Axum request extensions before the role layer executes.

## OAuth2/OIDC

Enable `oauth` for the `rullst_auth::connect` re-export. Provider configuration, discovery,
JWKS rotation, and deterministic offline fixtures are implemented by `rullst-connect`.

Security-sensitive functions return typed errors or a false verification result. The repository's
zero-panic CI checks production library paths; this policy is not an absolute guarantee about all
dependencies or host failures.
