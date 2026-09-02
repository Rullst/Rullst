# Rullst Auth

`rullst-auth` provides Argon2id password hashing, versioned AES-GCM cookie sessions,
role-based authorization middleware, WebAuthn/passkey ceremony verification, and
an opt-in application JWT policy.

Application JWTs are enabled with `jwt`. The `sqlite` feature also enables JWT
and adds durable shared JWT revocation plus passkey device state. OAuth2/OIDC
providers remain a separate trust boundary enabled with `oauth`, which
re-exports `rullst-connect`.

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
accepted. With `sqlite`, `SqlitePasskeyStore` supplies bounded file-backed
registration, listing, renaming, revocation and optimistic counter CAS shared
by processes on the same SQLite file. `finish_authenticate` verifies the ES256
ceremony and atomically advances the stored counter; a stale concurrent update
fails. Revoked records remain in inventory and continue to consume quota.

Challenge state remains process-local inside `PasskeyAuth`, so a multi-instance
deployment needs sticky ceremony routing or a custom shared challenge layer.
The adapter does not establish normative WebAuthn conformance, encrypt or
replicate the database, or replace application device-ownership policy.

## Application JWTs

The `jwt` feature provides `ApplicationJwtPolicy`, versioned HS256 claims, strong
key validation, required issuer/audience/subject/time/JTI claims, bounded TTL and
scope policy, and `kid`-based key rotation. Every verification receives a
`JwtRevocationStore`. Production policies reject the bundled bounded in-memory
store because it is process-local. With `sqlite`,
`SqliteJwtRevocationStore` persists token IDs and monotonic subject session
versions behind a stored quota. Its `BEGIN IMMEDIATE` mutations are visible to
local processes, expired token rows are pruned before capacity checks, and
`ApplicationJwtPolicy::verify_async` checks that shared state.

The SQLite boundary is durable across restarts but not replicated across hosts.
The deployment owns trusted paths, file permissions/encryption, backup,
availability and disaster recovery. This API does not verify third-party
OAuth/OIDC tokens or provide refresh tokens.

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
