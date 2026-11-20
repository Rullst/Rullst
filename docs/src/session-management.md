# Active sessions and remote logout

Rullst has two server-side session systems. The **login-session registry**
(`rullst::auth::session_registry`, since 13.0.0-alpha.6) keeps a record per
`rullst_session` cookie in the application's own database; `cargo rullst auth`
and the SaaS starter use it. The **recovery store** (`SqlRecoveryStore`, below)
is a complete account registry with its own credentials, opaque sessions and
password recovery.

## Server-side login sessions

A `make_login_cookie_with_epoch` cookie is stateless: signing out deletes it
from the browser, but a copy (stolen from a shared computer, a backup, malware
or a proxy log) keeps working until it expires, and the session epoch can only
end every session of an account at once. The registry closes that gap.

- **Sign-in.** `SessionRegistry::create_session` (or `sign_in`, which reads
  `APP_KEY` and the clock and returns the `Set-Cookie` value) stores one row in
  `rullst_auth_sessions` and returns a `v3.` cookie: the account id, its
  session epoch, the expiry and a random 256-bit session secret, sealed with
  AES-256-GCM under `APP_KEY`. The row keeps only a SHA-256 digest of the
  secret as its primary key, so a copy of the table names no cookie, and a
  separate 128-bit digest as the account-visible reference.
- **Every request.** `authenticate` (or `authenticate_request`) opens the
  cookie and looks the row up by its primary key. There is no cache: a
  revocation takes effect on the next request of every process that shares
  the database, and a database failure fails closed (`Unavailable`). The
  last-seen time is the start of the hour, written at most once an hour per
  session. `decrypt_session_claims` refuses `v3.` cookies, so a route that
  skips the registry cannot accept a revoked one; the session epoch still
  works through `AuthenticatedSession::claims().verify_epoch`.
- **Sign-out and devices.** `sign_out`/`revoke` delete the current row, so
  every copy of the cookie stops working. `list_sessions(user_id, now)` lists
  the account's live sessions, most recently used first;
  `revoke_session(user_id, &reference)` signs out one (a reference of another
  account's session revokes nothing), `revoke_other_sessions(&current)` all
  but this one and `revoke_all_sessions(user_id)` all of them: call it on a
  password reset or change, on "sign out everywhere" and when an account is
  deactivated.
- **Data kept.** The account id, creation time, last-seen hour, expiry and a
  device summary: a browser family and a platform family read from the
  sign-in's `User-Agent` ("Firefox on Linux"), never the header itself or a
  version. No IP address, location or request history is stored, following
  the data-minimization rules of the [legal compliance roadmap](legal-compliance-roadmap.md).
  The summary is a hint for the account owner, not verified device identity.
- **Limits and retention.** An account keeps at most 50 sessions; the next
  sign-in signs out its least recently used one (exact on SQLite; concurrent
  sign-ins on PostgreSQL and MySQL can pass it briefly by their number).
  Revocation deletes the row. A sign-in deletes its account's expired rows,
  and `purge_expired(now, limit)` deletes up to 1,000 expired rows per call:
  schedule it. Every database call has a five-second deadline.
- **Stores and table ownership.** The registry uses the application's SQLx
  pool: `session-registry-sqlite` (the facade's
  `auth-session-registry-sqlite`), `-postgres` and `-mysql` (MySQL and
  MariaDB). A migration should own the table: its `up` calls
  `SessionRegistry::create_table(pool)` (the table and its indexes, kept when
  they exist; on PostgreSQL concurrent calls serialize on an advisory lock),
  its `down` drops `rullst_auth_sessions`, and the application calls
  `SessionRegistry::connect(pool)`, which creates nothing and fails with
  `Unavailable` while the table is missing. `SessionRegistry::open(pool)`
  creates the table on first use and then connects: the fallback for an
  application without such a migration, whose runtime database role then
  needs `CREATE` once and whose `db:rollback` leaves the table. SQLite is
  shared-local: processes on one host using the same file.
  `SessionRegistryFor<rullst::orm::RullstPool>` names the registry of the
  ORM's strict backend.
- **What a revocation does not reach.** Each request is authenticated when
  it starts, so a request already running when its session is revoked
  completes normally; the next one is refused. Long-lived responses are
  covered below.

```rust,no_run
# #[cfg(feature = "auth-session-registry-sqlite")]
# async fn handlers(headers: rullst::server::HeaderMap) -> Result<(), rullst::auth::session_registry::SessionRegistryError> {
use rullst::auth::session_registry::{DeviceSummary, NewSession, SessionRegistry};

let pool = rullst::db::sqlx::SqlitePool::connect("sqlite://app.db").await
    .map_err(|_| rullst::auth::session_registry::SessionRegistryError::Unavailable)?;
// `open` creates the table on first use; with a migration that calls
// `SessionRegistry::create_table`, call `SessionRegistry::connect` instead.
let sessions = SessionRegistry::open(pool).await?;
// After the password check, at the account's session epoch (0 without one):
let set_cookie = sessions
    .sign_in(&NewSession::new(42, 0).with_device(DeviceSummary::from_headers(&headers)))
    .await?;
// In the authentication middleware:
let session = sessions.authenticate_request(&headers).await?;
// "Sign out of all other devices":
sessions.revoke_other_sessions(session.current()).await?;
# let _ = set_cookie;
# Ok(())
# }
```

### Generated applications

`cargo rullst auth` and the SaaS starter enable the registry feature of their
strict database profile (`auth-session-registry-sqlite`, `-postgres` or
`-mysql`; `cargo rullst auth` now refuses a project without exactly one strict
profile) and write a `*_create_rullst_auth_sessions_table` migration, which
creates the table on `db:migrate` and drops it on `db:rollback`.
`src/middlewares/auth_middleware.rs` connects to it over `Orm::pool()` on
first use, registers every sign-in (`start_session`) and inserts the
signed-in account's `AuthenticatedSession` next to its id and, since
13.0.0-alpha.6, the `User` it loaded (`Extension<User>`), so a handler such
as the dashboard renders it without a second query. Each request behind the
middleware runs two primary-key queries (the session row and the account),
plus the hourly last-seen update; nothing is cached, so a revoked session
fails on its next request. Sign-out
deletes the session, `/sessions` lists the account's devices with a "Sign out"
button each and "Sign out of all other devices", a password change deletes the
account's sessions and gives the asking browser a new one, a password reset
deletes them all, and `User::revoke_sessions` ("sign out everywhere") bumps the
epoch and deletes them. `make:mfa` and `make:oauth` register their sign-ins the
same way in such a starter, and `make:oauth`'s `LinkAndResetPassword` policy
deletes the account's sessions in the transaction that removes its password
(`SessionRegistry::revoke_all_sessions_on`), so they also leave "Your
devices"; in a starter generated before 13.0.0-alpha.6 they keep issuing epoch
cookies (`cargo rullst upgrade` reports those files, `V13-AUTH-SESSION-REGISTRY`,
and the [alpha.5 to alpha.6 checklist](migration-v13.md#upgrading-an-alpha5-application-to-alpha6)
lists the changes). The middleware's `revalidation(&session)` returns the check for
long-lived responses below.

### Long-lived connections

A Server-Sent Events stream or a WebSocket outlives the request that
authorized it. Rullst's helpers revalidate it: `RealtimeSse::channel_revalidated`
(and `subscribe_revalidated`) and `WebSocket::with_revalidation` take a
`rullst::realtime::Revalidate` check, run it every revalidation interval
(15 seconds by default, `RealtimeSseConfig::with_revalidation_interval` or the
WebSocket's interval argument, 1 second to 5 minutes) and when the check's
change signal fires, and end the connection when it answers `false` or takes
longer than 5 seconds. A WebSocket checks while it waits for the client and
before sending when a check is due.

`SessionRegistry::revalidation(&current)` is that check for a registered
session: it answers whether the session's row still exists and has not
expired, and every revocation through a registry of the same process
(sign-out, revoking one, the other or every session, also inside a
transaction) signals the checks of the sessions it ends. So a stream of a
session revoked in the same process ends at once, and one revoked by another
process or host ends within one revalidation interval. The registry check
looks at the session row only; an application that revokes by its session
epoch alone should check the epoch in its own `Revalidate` (the generated
applications delete the rows whenever they bump the epoch).

```rust,ignore
// In a route behind auth_middleware, with `Extension(session): Extension<AuthenticatedSession>`:
let check = crate::middlewares::auth_middleware::revalidation(&session).await?;
Ok(state.sse.channel_revalidated(&state.channels, "dashboard", check))
```

Cookies issued before an application adopts the registry are refused, so every
user signs in once more; see the [migration guide](migration-v13.md#changes-from-the-published-1210-source).

## The recovery store

Since v13, session management extends `rullst-auth::recovery::SqlRecoveryStore` with a bounded
session inventory, selective sibling logout and logout of all other sessions.
It uses the existing authoritative account registry and opaque bearer sessions;
it does not add another crate. Hosted workspace/platform and extracted-package source admission passed in [PR #236](v13-delivery-plan.md#six-increment-source-admission-on-september-21).

Choose `recovery-sqlite` or `recovery-postgres` on Auth. The facade exposes
`auth-sessions-sqlite` and `auth-sessions-postgres` without enabling Mail or the
general ORM. Existing `account-mail-*` consumers gain the same APIs after their
explicit schema migration.

## Adoption and isolation

Follow the [account-registry adoption contract](account-mail-v12-1.md) for keys,
password verification, cookies and account migration. Run `migrate()` during a
controlled deployment before using the new library: it adds a session-details
table and an expiry index. Existing opaque sessions continue working and report
unknown creation/label values if they predate those details. Encrypted-only
`rullst_session` cookies are revoked through their own per-user
[session epoch](crates/auth.md#revocable-cookies-the-session-epoch), which ends
all of a user's cookies at once, and independently issued JWTs through their
own revocation store.

Applications generated by `cargo rullst auth` and the SaaS starter use the
[login-session registry](#server-side-login-sessions) on top of the epoch; the
LMS example uses epoch cookies only. Their password change stores the new hash
and increments the epoch in one `UPDATE`, then gives the browser that made the
change a new session at the new epoch, so only that browser stays signed in.
Their password reset increments it in the transaction that consumes the
single-use reset token and signs in nobody, so every device must sign in again.
See [Tutorial 11](tutorials/11-authentication-system.md#what-the-scaffold-currently-enforces).

Stores are application/tenant boundaries selected by trusted server
configuration, with separate databases and keys. A browser cannot choose a
database, account subject or tenant by supplying an identifier. Every protected
request calls `verify_session` against that same authoritative store. PostgreSQL
can serve multiple hosts; SQLite requires processes sharing the same local file
and does not provide multi-host replication. `SqlRecoveryStore::connect` requires
certificate- and hostname-verified TLS (`sslmode=verify-full`) for every remote
PostgreSQL host and accepts only connection/TLS query options, as email login and
API tokens do.

Management methods authenticate the supplied current session again inside a
serialized SQL transaction. They never accept a target account. The host still
checks current tenant membership and protects its routes with CSRF, the security
baseline, ingress limits and `Cache-Control: no-store`. Use trusted server time,
never browser timestamps, and maintain a reliable deployment clock.

## Inventory and logout

Each account has at most 20 sessions. `active_sessions` returns only unexpired
sessions matching the current account version, with a management ID, expiry,
optional creation time, optional
display label and a current-session marker. The ID is a purpose-separated keyed
digest and cannot authenticate as the session. Debug output hides IDs and labels.

```rust,no_run
# #[cfg(feature = "auth-sessions-sqlite")]
async fn sign_out_other_devices(
    store: &rullst::auth::recovery::SqlRecoveryStore,
    current_cookie: &str,
    trusted_now: u64,
) -> Result<usize, rullst::auth::recovery::RecoveryError> {
    // Host middleware has already authorized this account/tenant and checked CSRF.
    store.revoke_other_sessions(current_cookie, trusted_now).await
}
```

`revoke_other_session(current_cookie, &SessionId, trusted_now)` removes one
active sibling. A missing, expired, foreign-account or current-session target is
rejected. `revoke_other_sessions` removes all siblings and advances the account
session version, preserving the current bearer token. A password-authentication
proof obtained before that operation can no longer mint a session; authenticate
again before creating a later session. Repeating logout of siblings is safe.
Use the existing `revoke_session(current_cookie)` for current-device logout.

`create_session_with_label` accepts a `SessionLabel` of 1–80 UTF-8 bytes without
control characters or surrounding whitespace. Labels are explicit display text,
not verified device identity. Escape them in HTML. No IP addresses, raw user
agents, fingerprints or activity history are collected. The ordinary
`create_session` records creation time without a label.

## Expiration, retention and failures

Expired sessions are rejected at their exclusive expiry boundary. Time passing
does not physically delete rows. Creating a session prunes that account's expired sessions;
schedule the operator-owned `purge_expired_sessions(trusted_now, limit)` to remove
1–100 expired sessions per transaction, including inactive accounts. Metadata is
deleted atomically with sessions during retention, logout and password reset.
Operator database/backups/encryption and retention scheduling remain deployment
responsibilities; do not expose this maintenance API as a public endpoint.

Session creation, verification, management and retention operations have a
ten-second database-operation deadline. Storage failures remain errors rather
than authentication success; inventory also rejects out-of-bounds metadata.
A timeout or lost commit acknowledgement does not prove that a write rolled
back: query authoritative state before reporting its outcome.
Already-authorized in-flight requests are not recalled by logout; long-lived
WebSockets and other identity systems need their own revalidation policy.

## Automated acceptance

Tests exercise account/store separation, metadata migration, expiry, password
recovery, selective/all-other logout, transaction rollback, concurrent logout,
retention and denied access after storage failure. Real HTTP fixtures use two
independent application processes, the Core security baseline and tenant checks;
they prove that a completed logout is visible on the other process's next
request and persists after application restart.

The digest-pinned disposable PostgreSQL journey additionally restarts the actual
database and checks retained/revoked sessions afterwards. A held database lock
tests the operation deadline. The archive consumer exercises the public facade
from extracted packages. These are automated contract checks, not evidence of a
particular production identity migration, failover topology or human-tested UI.
