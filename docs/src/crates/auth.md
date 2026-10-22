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
memory, iteration, and parallelism parameters. The optional `SqlRecoveryStore` applies
the same 72-byte bound to registration, password reset and `authenticate`; overlong input
returns `RecoveryError::InvalidInput` before any account lookup, for registered and
unknown emails alike.

## Encrypted sessions

`make_login_cookie`, `make_login_cookie_with_epoch` and their decoders
`decrypt_session` and `decrypt_session_claims` use a versioned AES-256-GCM
envelope with authenticated metadata and an operating-system nonce. `APP_KEY` must contain at least 32 bytes,
must not be a documented placeholder, and must satisfy the entropy check.

The cookie helpers select `Secure` from `RULLST_ENV`, then `APP_ENV`, then `.env`, then
`Rullst.toml`. When either process variable is set, `.env` is not read for that decision. A
malformed `.env` produces a fixed `AuthError::General` message naming only the failing entry
number; file content, including values after an unclosed quote, never appears in the error.
A malformed `Rullst.toml` likewise reports only its line and column.
Since 13.0.0-alpha.2 the Core security baseline that `Server` mounts also adds
`Secure` to these `rullst_session` cookies, in any environment, when the request
arrived over HTTPS: an `https` request URI, or `X-Forwarded-Proto: https` from a
proxy that `[security] trusted_proxies` lists with `trust_forwarded_proto = true`
(the same evidence Nexus Basic Auth accepts). It never removes `Secure`.

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

### What the stateless cookie can and cannot do

The server keeps no list of sessions. A `rullst_session` token holds only the
user id, its expiry and, for the epoch cookies below, a session epoch, sealed
with `APP_KEY`:

- It proves the server issued the cookie, expires on its own, and stops
  working once the application changes the user's stored session epoch.
- It cannot end one device's session while keeping the others, list active
  sessions, or be revoked without that stored epoch. `make_logout_cookie`
  only asks the browser to delete its copy: a copied cookie stays valid until
  it expires (30 days for `make_login_cookie`), the epoch changes, or
  `APP_KEY` is rotated, which signs out every user.

For server-side sessions that can be listed and revoked one at a time, use
`recovery::SqlRecoveryStore` (`recovery-sqlite` or `recovery-postgres`; see
[active-session management](../session-management.md)).

### Revocable cookies: the session epoch

Store a session epoch per user, an integer starting at 0, put it in the cookie
at sign-in and compare it on every request:

```rust,no_run
use rullst_auth::{
    AuthError, DEFAULT_SESSION_MAX_AGE, decrypt_session_claims, get_app_key,
    make_login_cookie_with_epoch,
};

/// At sign-in: the cookie carries the user's current epoch.
fn sign_in(user_id: i32, stored_epoch: u64) -> Result<String, AuthError> {
    make_login_cookie_with_epoch(user_id, stored_epoch, DEFAULT_SESSION_MAX_AGE)
}

/// On every request: `token` is the value `extract_session_cookie` returns and
/// `stored_epoch` loads the user's current epoch.
fn signed_in_user(
    token: &str,
    stored_epoch: impl Fn(i32) -> Option<u64>,
) -> Result<i32, AuthError> {
    let claims = decrypt_session_claims(token, &get_app_key()?)?;
    let current = stored_epoch(claims.user_id).ok_or(AuthError::SessionRevoked)?;
    claims.verify_epoch(current) // `AuthError::SessionRevoked` on a mismatch
}
```

To sign out every device, increment the stored epoch in one statement
(`UPDATE users SET session_epoch = session_epoch + 1 WHERE id = ?`) when the
user chooses "sign out everywhere", after a password change or reset, and when
the account is deactivated or deleted. Issue a new cookie with the new epoch
if the current browser should stay signed in.

- `max_age` is counted in whole seconds, from 1 second to 400 days
  (`MAX_SESSION_MAX_AGE`), and sets both the token expiry and the cookie
  `Max-Age`. A shorter lifetime limits how long a copied cookie works if
  nobody revokes it.
- `decrypt_session_claims` also accepts cookies made by `make_login_cookie`
  and `encrypt_session`, as epoch 0: start stored epochs at 0 and existing
  sessions keep working, until the first bump revokes them.
- `decrypt_session` rejects epoch cookies, so a route that still calls it
  fails closed instead of skipping the epoch check.
- `rullst-auth` has no session middleware: the application loads the stored
  epoch, which costs one lookup per request.

`cargo rullst auth` and the SaaS and LMS starters generate this wiring: a
`users.session_epoch` column, epoch cookies at sign-in and registration, the
check in `auth_middleware`, `User::revoke_sessions`, a
`POST /logout/everywhere` route, and a password change and a password reset
that increment the epoch (below).

### Password reset tokens

For a users table the application owns, `PasswordResetToken` is the emailed
secret of a reset link (new in v13): 32 bytes from the operating system's
CSPRNG, written as 64 lowercase hex characters. Email
`expose()` and store only `digest()`, a domain-separated SHA-256:

```rust
use rullst_auth::{
    AuthError, PasswordResetToken, password_reset_digests_match, password_reset_token_digest,
};

// On a reset request: email `token.expose()` in a link such as
// https://app.example.com/password/reset?token=…, and store the digest with
// the account id and an expiry.
let token = PasswordResetToken::generate()?;
let stored = token.digest();

// When the link comes back: a malformed value has no digest, so it never
// reaches the database; look the digest up and compare in constant time.
let submitted = password_reset_token_digest(token.expose()).ok_or(AuthError::InvalidCredentials)?;
assert!(password_reset_digests_match(&stored, &submitted));
# Ok::<(), AuthError>(())
```

Expiry, single use and one live token per account belong to the
application's store. The generated `password_reset_tokens` table keeps one
token per account (its primary key), expires it after
`RESET_TOKEN_LIFETIME_MINUTES` (30 as generated; 10 minutes to 24 hours, see
[Tutorial 11](../tutorials/11-authentication-system.md#what-the-scaffold-currently-enforces))
and deletes
it in the transaction that stores the new password hash and increments the
session epoch. A password change keeps the browser that made it signed in with
a cookie at the new epoch; a reset signs out every device and redirects to the
login page. See [Tutorial 11](../tutorials/11-authentication-system.md#what-the-scaffold-currently-enforces)
for the generated routes and their answers.

## Optional v13 account features

With `recovery-sqlite` or `recovery-postgres`, `recovery::SqlRecoveryStore`
provides an authoritative account registry, password recovery and revocable
opaque sessions, including [active-session management](../session-management.md).
`email-login-sqlite`/`email-login-postgres` add
[single-use email login](../email-login.md), and
`api-tokens-sqlite`/`api-tokens-postgres` add
[scoped API tokens](../api-tokens.md). The facade features are
`auth-sessions-*`, `auth-email-login-*` and `auth-api-tokens-*`. These
v13 prerelease candidates are opt-in and do not enable JWT or OAuth.

## WebAuthn/passkeys

`PasskeyAuth` validates exact RP origin and ID binding, one-time expiring challenges
(`challenge_ttl_seconds` from 1 to 86,400; `PasskeyAuth::new` rejects other values),
client-data ceremony type, user-presence/user-verification flags, ES256 COSE keys, P-256 points,
credential IDs, signatures, and monotonic counters. Only `none` attestation is advertised and
accepted. With `sqlite`, `SqlitePasskeyStore` supplies bounded file-backed
registration, listing, renaming, revocation and optimistic counter CAS shared
by processes on the same SQLite file. `finish_authenticate` verifies the ES256
ceremony and atomically advances the stored counter; a stale concurrent update
fails. Revoked records remain in inventory and continue to consume quota.

Challenge state remains process-local inside `PasskeyAuth`, so a multi-instance
deployment using that API needs sticky ceremony routing or a custom shared challenge layer.
The optional v13 `passkey-postgres` candidate adds a separately bound shared manager
with real PostgreSQL/browser evidence; see [shared passkey ceremonies](../shared-passkey-ceremonies.md).
The adapter does not establish normative WebAuthn conformance, encrypt or
replicate the database, or replace application device-ownership policy.

## Application JWTs

The `jwt` feature provides `ApplicationJwtPolicy`, versioned HS256 claims, strong
key validation, required issuer/audience/subject/time/JTI claims, bounded TTL and
scope policy, and `kid`-based key rotation. Lifetimes are whole seconds; a TTL
or `max_ttl` under one second is rejected. Every verification receives a
`JwtRevocationStore`. Production policies reject the bundled bounded in-memory
store because it is process-local. With `sqlite`,
`SqliteJwtRevocationStore` persists token IDs and monotonic subject session
versions behind a stored quota. Its `BEGIN IMMEDIATE` mutations are visible to
local processes, expired token rows are pruned before capacity checks, and
`ApplicationJwtPolicy::verify_async` checks that shared state. This store and
`SqlitePasskeyStore` accept only an ordinary database file: `:memory:` in any
form, `file:` URI filenames and the `vfs`, `immutable` and `mode=memory` URL
parameters fail with `InvalidConfiguration`.

Both bundled stores cap token-ID rows at three quarters of the quota and at 64
active rows per subject. Past either cap, `revoke_token` records a subject cutoff
that rejects every token of that subject issued no later than the revoked one,
so one principal cannot exhaust capacity for others and subject revocations keep
a reserve. Subject entries are never pruned. Opening an existing SQLite file adds
two backward-compatible columns; earlier releases ignore subject cutoffs, so
upgrade every process sharing the file.

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
