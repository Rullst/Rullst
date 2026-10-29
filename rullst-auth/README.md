# Rullst Auth

`rullst-auth` provides Argon2id password hashing, versioned AES-GCM cookie sessions,
role-based authorization middleware, WebAuthn/passkey ceremony verification, and
an opt-in application JWT policy.

**Maturity: Extension** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).

## Installation

This README describes the v13 prerelease; Cargo never selects a prerelease on
its own, so pin it exactly:

```bash
cargo add rullst-auth@=13.0.0-alpha.4
```

Passwords, cookie sessions, passkey ceremonies, RBAC and policies need no
feature. The optional parts are Cargo features:

- `jwt`: the application JWT policy (`ApplicationJwtPolicy`).
- `sqlite`: `jwt` plus durable SQLite JWT revocation and the SQLite passkey
  device registry.
- `passkey-postgres`: passkey ceremonies shared between hosts through
  PostgreSQL.
- `recovery-sqlite` / `recovery-postgres`: `recovery::SqlRecoveryStore`
  (password recovery and revocable server-side sessions);
  `email-login-sqlite` / `email-login-postgres` and `api-tokens-sqlite` /
  `api-tokens-postgres` build on them.
- `oauth`: re-exports `rullst-connect` as `rullst_auth::connect`. OAuth2/OIDC
  providers remain a separate trust boundary.

In a Rullst application, enable the `auth` feature of `rullst` (the crate is
then `rullst::auth`), plus `auth-jwt`, `auth-sqlite`, `auth-passkey-postgres`,
`auth-sessions-sqlite`/`-postgres`, `auth-email-login-sqlite`/`-postgres` or
`auth-api-tokens-sqlite`/`-postgres` for the optional parts.

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
`get_app_key` reads it from the `APP_KEY` environment variable, then `.env`,
then `Rullst.toml`. Without one, staging and production fail with
`AuthError::MissingAppKey`; other environments generate a development key in
`.rullst_dev_key`.

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
[active-session management](https://github.com/Rullst/Rullst/blob/main/docs/src/session-management.md)).

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

`cargo rullst auth` and the SaaS starter generate this wiring (the LMS example has it too): a
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
token per account (its primary key), expires it after 30 minutes and deletes
it in the transaction that stores the new password hash and increments the
session epoch. A password change keeps the browser that made it signed in with
a cookie at the new epoch; a reset signs out every device and redirects to the
login page. See [Tutorial 11](https://github.com/Rullst/Rullst/blob/main/docs/src/tutorials/11-authentication-system.md#what-the-scaffold-currently-enforces)
for the generated routes and their answers.

## Active-session management candidate (v13)

With `recovery-sqlite` or `recovery-postgres`, `SqlRecoveryStore` can list the
current account's active opaque sessions, revoke one sibling or revoke all
others while preserving the current session. Explicit display labels are
bounded; no device fingerprint, IP address or activity history is collected.
Run the additive schema migration and verify SQL state on every request.
Encrypted-only cookies and independent JWTs do not adopt this policy
implicitly; encrypted cookies use the session epoch above.
Hosted source/package admission passed in PR #236; final release admission
remains separate. See the
[session contract](https://github.com/Rullst/Rullst/blob/main/docs/src/session-management.md) for retention, deadlines,
tenant boundaries and automated evidence.

## Email login candidate (v13)

Optional `email-login-sqlite` / `email-login-postgres` adds explicit account-opt-in,
browser-bound single-use links and an encrypted delivery outbox. A deliberate
CSRF-protected POST consumes the link and creates an existing revocable opaque
session in one transaction. GET/HEAD never authenticate. The application still
owns tenant authorization, MFA policy, secure cookies and sensitive-URL logging.
SQLite shares one local file; PostgreSQL shares one authoritative writable server.
See the [email-login contract](https://github.com/Rullst/Rullst/blob/main/docs/src/email-login.md) for limits, retention,
Mail composition and process/browser evidence. Combined hosted and
extracted-package source admission passed in PR #239; final release admission
remains separate.

## API token candidate (v13)

Optional `api-tokens-sqlite` / `api-tokens-postgres` supplies owner-managed opaque
credentials with literal scopes, bounded lifetime, atomic revision-based rotation
and authoritative SQL revocation. Only HMAC digests are stored. Account-epoch
changes invalidate older credentials. The Core machine-route adapter requires
an Authorization bearer and preserves the security baseline; applications still
enforce current tenant membership and resource permissions. See the
[API-token contract](https://github.com/Rullst/Rullst/blob/main/docs/src/api-tokens.md) for setup, operational limits
and restart/HTTP evidence. Combined hosted and extracted-package source
admission passed in PR #239; final release admission remains separate.

## WebAuthn/passkeys

`PasskeyAuth` validates exact RP origin and ID binding, one-time expiring challenges
(`challenge_ttl_seconds` from 1 to 86,400; `PasskeyAuth::new` rejects other values),
client-data ceremony type, user-presence/user-verification flags, ES256 COSE keys, P-256 points,
credential IDs, signatures, and monotonic counters. Only `none` attestation is advertised and
accepted. With `sqlite`, `SqlitePasskeyStore` supplies a bounded file-backed
device registry: registration quotas, listing/renaming/revocation, restart-safe
public credentials and optimistic counter CAS are shared by processes using the
same SQLite file. `finish_authenticate` verifies the ES256 ceremony first and
then atomically advances the stored counter; a concurrent stale update fails.
Multi-statement SQLite mutations use cancellation-safe transactions: dropping
an unfinished registration rolls it back before the pooled connection is reused.
Revoked entries remain visible in device inventory and continue to count toward
the configured quota so revocation history is not silently recycled. If the
wall clock is stepped back below a credential's creation time, its last use and
revocation are recorded at that creation time instead of failing.

WebAuthn challenge state remains bounded and process-local inside `PasskeyAuth`.
The optional v13 `passkey-postgres` path adds
`passkey::shared::SharedPasskeyAuth<PostgresCeremonyStore>` for challenges shared
by separate hosts. It binds tenant/account/session, RP configuration and current
credential fingerprints, consumes once under a database transaction, and checks
expiry again after cryptographic work. Store capacity, lifetime and an
independently retained deployment epoch must match every instance. Normal startup
opens existing state; deployment initialization is explicit.

The host still establishes registration authority, resolves the account, forwards
an optional authenticator user handle, and atomically persists current credential
ownership/revocation/counter state before granting a session. The existing
SQLite registry and the shared ceremony adapter are not one distributed
transaction. See the [shared ceremony contract](https://github.com/Rullst/Rullst/blob/main/docs/src/shared-passkey-ceremonies.md)
for its evidence; hosted and archive acceptance passed in PR #223, while the
final release campaign remains separate. Neither adapter
establishes normative WebAuthn conformance, manages replication/failover, or
replaces application identity and device-ownership policy.

## Application JWTs

The `jwt` feature provides `ApplicationJwtPolicy`, versioned HS256 claims, strong
key validation, required issuer/audience/subject/time/JTI claims, bounded TTL and
scope policy, and `kid`-based key rotation. Every verification receives a
`JwtRevocationStore`. Production policies reject the bundled bounded in-memory
store because it is process-local.

Lifetimes are whole seconds: a fractional TTL is truncated, and a TTL or
`max_ttl` under one second is rejected because it could never verify.
Token expiration is an exclusive deadline: verification rejects `now >= exp`,
including when clock skew is configured. Skew only tolerates a future `iat` or
`nbf`; it cannot revive an expired token after its revocation entry is pruned.

With `sqlite`, `SqliteJwtRevocationStore` persists token IDs and monotonic
subject session versions behind a stored entry quota. Mutations serialize with
`BEGIN IMMEDIATE` in cancellation-safe transactions, expired token rows are pruned before capacity checks, and
`ApplicationJwtPolicy::verify_async` reads the shared state:

```rust,no_run
use rullst_auth::{ApplicationJwtPolicy, JwtError, SqliteJwtRevocationStore};

async fn verify_shared(
    policy: &ApplicationJwtPolicy,
    token: &str,
) -> Result<(), JwtError> {
    let store = SqliteJwtRevocationStore::connect(
        "sqlite://storage/auth.sqlite",
        100_000,
    ).await?;
    policy.verify_async(token, &store).await?;
    Ok(())
}
```

Both bundled stores cap token-ID rows at three quarters of the entry quota and at 64 active
rows per subject. Past either cap, `revoke_token` records a subject cutoff instead: every
token of that subject issued no later than the revoked one is rejected, which is broader but
never weaker. One principal that repeatedly logs in and out therefore cannot exhaust the
quota, and subject revocations keep the remaining quarter. Subject entries are never pruned;
size `max_entries` for the subjects that may revoke. `SqliteJwtRevocationStore::connect`
adds a `subject` and a `revoked_through_iat` column to an existing file. Earlier releases can
still open it but ignore subject cutoffs, so upgrade every process sharing the file.

The SQLite adapter is durable across restarts and shared across local processes,
not replicated across hosts. `SqliteJwtRevocationStore` and `SqlitePasskeyStore`
therefore accept only an ordinary database file: `:memory:` in any form, `file:`
URI filenames and the `vfs`, `immutable` and `mode=memory` URL parameters fail
with `InvalidConfiguration`. The deployment owns its trusted directory, file
permissions/encryption, backup, availability and disaster recovery. This API
does not verify third-party OAuth/OIDC tokens or provide refresh tokens.

## RBAC

Implement `HasRole` for the authenticated user type and install
`RequireRoleLayer::<User>::new("Admin")`. Authentication middleware must insert that user into
Axum request extensions before the role layer executes.

Applications using the umbrella crate (with its `auth` feature) may
alternatively place `#[rullst::require_role("Admin")]` on an async handler whose authenticated
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
