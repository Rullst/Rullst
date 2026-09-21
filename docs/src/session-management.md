# Active sessions and remote logout

The v13 candidate extends `rullst-auth::recovery::SqlRecoveryStore` with a bounded
session inventory, selective sibling logout and logout of all other sessions.
It uses the existing authoritative account registry and opaque bearer sessions;
it does not add another crate. Hosted workspace/platform and extracted-package source admission passed in [PR #236](v13-delivery-plan.md#six-increment-source-admission-on-september-21). Final release admission remains separate.

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
cookies and independently issued JWTs require their own adoption/revocation path.

Stores are application/tenant boundaries selected by trusted server
configuration, with separate databases and keys. A browser cannot choose a
database, account subject or tenant by supplying an identifier. Every protected
request calls `verify_session` against that same authoritative store. PostgreSQL
can serve multiple hosts; SQLite requires processes sharing the same local file
and does not provide multi-host replication.

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
