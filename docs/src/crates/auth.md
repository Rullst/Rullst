# Rullst Auth

`rullst-auth` provides Argon2id password hashing, versioned AES-GCM cookie sessions,
role-based authorization middleware, opt-in WebAuthn/passkey ceremony verification, and
an opt-in application JWT policy.

Passkeys are enabled with `passkeys` (`auth-passkeys` in `rullst`; since
13.0.0-alpha.6 `auth` alone no longer includes them), the only feature that
adds aws-lc-rs to this crate. Application JWTs are enabled with `jwt`. The
`sqlite` feature also enables JWT and passkeys and adds durable shared JWT
revocation plus passkey device state. OAuth2/OIDC
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
the same 72-byte bound to registration, password reset and `authenticate`. For
registration and `authenticate`, overlong input returns `RecoveryError::InvalidInput`
before any account lookup, for registered and unknown emails alike; a password reset
looks up the reset token first, so an unknown, expired or used token returns
`RecoveryError::InvalidAction` whatever the password's length.

## Encrypted sessions

`make_login_cookie`, `make_login_cookie_with_epoch` and their decoders
`decrypt_session` and `decrypt_session_claims` use a versioned AES-256-GCM
envelope with authenticated metadata and a random 96-bit nonce from the `rand` crate's
thread-local generator (seeded from the operating system). `APP_KEY` must contain at least 32 bytes,
must not be a documented placeholder, and must satisfy the entropy check.
`get_app_key` reads it from the `APP_KEY` environment variable, then `.env`,
then `Rullst.toml`. Without one, staging and production fail with
`AuthError::MissingAppKey`; other environments use a development key kept
outside the source tree (since 13.0.0-alpha.5). A `.rullst_dev_key` that an
earlier version wrote into the current directory is still read, but none is
written there any more: the key lives in the per-user state directory,
`$XDG_STATE_HOME/rullst/dev-keys/` (else `~/.local/state/rullst/dev-keys/`)
on Unix and macOS or `%LOCALAPPDATA%\rullst\dev-keys\` on Windows, in a file
named after the SHA-256 of the canonical project directory (owner-only on
Unix). A test harness, an executable where `cargo test` and cargo-nextest
run unit and integration tests (Cargo's `target/<profile>/deps` up to Cargo
1.99; since Cargo 1.100, `target/<profile>/build/<package>/<hash>/out`, with a
name ending in `-<hash>`), never writes a key: it keeps a new one in memory
for the process.
`cfg(test)` cannot make this choice, because it is set only for the crate
under test, never for `rullst-auth` as its dependency. Tests that need one
key in several processes (for example a test that starts the server binary
and reads its cookies) set `APP_KEY`.

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

For sessions that can be listed and revoked one at a time, keep the cookie
and add the [session registry](#revocable-sessions-the-session-registry), or
use the complete account registry `recovery::SqlRecoveryStore`
(`recovery-sqlite` or `recovery-postgres`; see
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

`cargo rullst auth` and the SaaS starter generate this wiring, with the
session registry below on top (the LMS example has the epoch only): a
`users.session_epoch` column, the epoch in every cookie, the check in
`auth_middleware`, `User::revoke_sessions`, a `POST /logout/everywhere` route,
and a password change and a password reset that increment the epoch (below).

### Revocable sessions: the session registry

The epoch ends every session at once; it cannot end one device, list devices
or make sign-out stop a copied cookie. `session_registry` (features
`session-registry-sqlite`, `session-registry-postgres` and
`session-registry-mysql`; the facade's `auth-session-registry-*`) adds one row
per sign-in in the table `rullst_auth_sessions` of the application's own
database, over its SQLx pool. The table's `user_id` has no foreign key, since
the registry does not know the accounts table: call `revoke_all_sessions`
when an account is deleted, or link the column in the migration that owns the
table, as the one `cargo rullst auth` generates does
([accounts deleted with SQL](../session-management.md#accounts-deleted-with-sql)):

```rust,no_run
# #[cfg(feature = "session-registry-sqlite")]
# async fn example(pool: sqlx::SqlitePool, headers: axum::http::HeaderMap, stored_epoch: u64)
# -> Result<(), rullst_auth::session_registry::SessionRegistryError> {
use rullst_auth::session_registry::{DeviceSummary, NewSession, SessionRegistry, unix_now};

// `open` creates the table on first use; with a migration that calls
// `SessionRegistry::create_table`, use `SessionRegistry::connect(pool)`.
let sessions = SessionRegistry::open(pool).await?;
// Sign-in: the `Set-Cookie` value of a new registered session.
let login = NewSession::new(42, stored_epoch).with_device(DeviceSummary::from_headers(&headers));
let set_cookie = sessions.sign_in(&login).await?;
// Every request: one primary-key lookup; then compare the epoch as above.
let session = sessions.authenticate_request(&headers).await?;
session.claims().verify_epoch(stored_epoch).ok();
// "Your devices", "sign out this device", "sign out of all other devices":
let devices = sessions.list_sessions(session.user_id(), unix_now()?).await?;
sessions.revoke_session(session.user_id(), devices[0].reference()).await?;
sessions.revoke_other_sessions(session.current()).await?;
// Sign-out ends every copy of this cookie; a password reset ends them all.
sessions.sign_out(&headers).await?;
sessions.revoke_all_sessions(42).await?;
# let _ = set_cookie;
# Ok(())
# }
```

- The `v3.` cookie carries the account id, the epoch, the expiry and a random
  256-bit session secret, sealed like the other cookies. The row stores only
  a SHA-256 digest of the secret, so a copy of the table yields no cookie.
  `decrypt_session_claims` refuses `v3.` cookies, so a route that skips the
  registry fails closed.
- `authenticate` checks the row on every request, without a cache: a
  revocation applies on the next request of every process sharing the
  database. The row's last-seen time is the start of the hour, written at most
  once an hour. A database failure is `SessionRegistryError::Unavailable` and
  never authenticates; `is_signed_out()` tells an invalid, expired or revoked
  session (send the browser to sign in) from a failure.
- The list shows the creation time, the last-seen hour, the expiry and a
  `DeviceSummary` (browser and platform family, for example "Safari on iOS");
  no IP address or raw `User-Agent` is stored. `SessionRef` values are
  separate digests, safe to put in a form, and `revoke_session` takes the
  signed-in account's id, so another account's reference revokes nothing.
- An account keeps at most 50 sessions (`MAX_SESSIONS_PER_USER`); the next
  sign-in signs out the least recently used. Schedule
  `purge_expired(now, limit)` (1 to 1,000 rows) to delete expired rows.
- Requests are checked when they start: one already running when its session
  is revoked completes. For Server-Sent Events and WebSockets, pass
  `sessions.revalidation(session.current())` to
  `RealtimeSse::channel_revalidated` or `WebSocket::with_revalidation`: the
  connection ends at once when this process revokes the session and within
  the revalidation interval (15 seconds by default) when another one does.
- `revoke_all_sessions_on(&mut *transaction, user_id)` revokes inside the
  caller's transaction, for example with a password removal.

`cargo rullst auth` and the SaaS starter generate the wiring: a migration owns
the table, registration and sign-in register a session, `auth_middleware` inserts the
`AuthenticatedSession`, `/logout` deletes the session, `/sessions` lists the
devices with "Sign out" and "Sign out of all other devices", and password
changes and resets delete the account's sessions. See
[session management](../session-management.md#server-side-login-sessions).

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
v13 additions are opt-in and do not enable JWT or OAuth.

## WebAuthn/passkeys

With the `passkeys` feature (`auth-passkeys` in `rullst`), `PasskeyAuth` validates exact RP origin and ID binding, one-time expiring challenges
(`challenge_ttl_seconds` from 1 to 86,400; `PasskeyAuth::new` rejects other values),
client-data ceremony type, user-presence/user-verification flags, ES256 COSE keys, P-256 points,
credential IDs, signatures, and monotonic counters. Public keys and ES256 signatures are checked
with aws-lc-rs, the library behind Rullst's TLS; high-S signatures are accepted, as WebAuthn allows.
Only `none` attestation is advertised and accepted. With `sqlite`, `SqlitePasskeyStore` supplies bounded file-backed
registration, listing, renaming, revocation and optimistic counter CAS shared
by processes on the same SQLite file. `finish_authenticate` verifies the ES256
ceremony and atomically advances the stored counter; a stale concurrent update
fails. Revoked records remain in inventory and continue to consume quota.

Challenge state remains process-local inside `PasskeyAuth`, so a multi-instance
deployment using that API needs sticky ceremony routing or a custom shared challenge layer.
The optional `passkey-postgres` feature (added in v13) adds a separately bound shared manager
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

## Authentication contract for generators

`cargo rullst make:mfa`, `make:oauth` and `make:privacy` reach an
application's own sign-in through `src/auth_contract.rs`; the
[CLI reference](../cli_reference.md#authentication-contract) lists what each
generator calls. A version over an ORM `User` (with `email`, `name` and a
`session_epoch` column, as `cargo rullst auth` generates it) and the encrypted
`rullst_session` cookie of this crate:

```rust,ignore
use crate::models::user::User;
use rullst::server::{HeaderMap, IntoResponse, Next, Redirect, Request, Response, StatusCode};

pub struct ContractAccount { pub id: i32, pub email: String, pub name: String }

impl From<User> for ContractAccount {
    fn from(user: User) -> Self {
        Self { id: user.id, email: user.email, name: user.name }
    }
}

pub async fn require_account(mut request: Request, next: Next) -> Response {
    let Some(cookie) = rullst::auth::extract_session_cookie(request.headers()) else {
        return Redirect::to("/login").into_response();
    };
    let Ok(app_key) = rullst::auth::get_app_key() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Ok(claims) = rullst::auth::decrypt_session_claims(&cookie, &app_key) else {
        return Redirect::to("/login").into_response();
    };
    match User::find(claims.user_id).await {
        Ok(Some(user)) if user.accepts_session(&claims) => {
            request.extensions_mut().insert(user.id);
            next.run(request).await
        }
        Ok(_) => Redirect::to("/login").into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub async fn find_account(id: i32) -> Result<Option<ContractAccount>, rullst::orm::Error> {
    Ok(User::find(id).await?.map(ContractAccount::from))
}

pub async fn find_account_by_email(email: &str) -> Result<Option<ContractAccount>, rullst::orm::Error> {
    Ok(User::find_by_email(email).await?.map(ContractAccount::from))
}

pub async fn sign_in_cookie(id: i32) -> Result<String, rullst::auth::AuthError> {
    let user = User::find(id)
        .await
        .map_err(|error| rullst::auth::AuthError::General(error.to_string()))?
        .ok_or(rullst::auth::AuthError::SessionRevoked)?;
    let epoch = user.current_session_epoch().ok_or(rullst::auth::AuthError::SessionRevoked)?;
    rullst::auth::make_login_cookie_with_epoch(user.id, epoch, rullst::auth::DEFAULT_SESSION_MAX_AGE)
}

pub async fn end_sessions(id: i32) -> Result<(), rullst::orm::Error> {
    User::revoke_sessions(id).await
}

pub fn session_binding(headers: &HeaderMap) -> Option<String> {
    rullst::auth::extract_session_cookie(headers)
}

pub async fn create_account(
    transaction: &mut rullst::orm::db::Transaction<'static>,
    email: &str,
    name: &str,
) -> Result<i32, rullst::orm::Error> {
    let now = rullst::orm::Timestamp::now();
    let mut user = User {
        name: name.to_owned(),
        email: email.to_owned(),
        created_at: now,
        updated_at: now,
        ..User::default()
    };
    user.save_with_tx(transaction).await?;
    Ok(user.id)
}
```

## OAuth2/OIDC

Enable `oauth` for the `rullst_auth::connect` re-export. Provider configuration, discovery,
JWKS rotation, and deterministic offline fixtures are implemented by `rullst-connect`.

Security-sensitive functions return typed errors or a false verification result. The repository's
zero-panic CI checks production library paths; this policy is not an absolute guarantee about all
dependencies or host failures.
