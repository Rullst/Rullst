# Optional email login (v13 candidate)

`rullst-auth` features `email-login-sqlite` and `email-login-postgres` expose
`recovery::email_login::EmailLoginService`. The umbrella features are
`auth-email-login-sqlite` and `auth-email-login-postgres`. They are opt-in and
reuse the authoritative `SqlRecoveryStore` account and opaque-session registry.
They do not enable JWT, OAuth or email login on existing accounts automatically.
Source/package/coverage admission for this increment is pending.

## Account and application boundaries

Create an immutable `EmailLoginConfig` with a server-owned application/tenant
namespace, HTTPS landing URL, fixed local destination and capacity (1–100,000
account policies per namespace). Query/fragment/user-info in the landing URL and
encoded/authority/traversal syntax in the destination are rejected. The explicit
development constructor allows HTTP only on localhost/loopback.

Use `initialize` during deployment and `connect` during normal startup. Access
`accounts()` to register/authenticate accounts in this registry. Enable a policy
with `set_account_enabled` only after recent password authentication plus the
application's tenant and MFA policy. The proof must come from this service's
account registry instance and current account epoch. Disabled accounts retain
their policy/revision and count toward capacity. An administrator-provided
namespace is a storage boundary, not proof that a caller belongs to a tenant.
Resolve membership, roles and resource ownership on each authorized request.

Email access is a single factor. Do not enable this route as an alternative that
bypasses required MFA, passkeys or a stronger identity assurance policy. This
increment does not implement enrollment UX, account linking, invitation-based
registration, step-up MFA, verified age or guardian authorization.

## Request, delivery and deliberate confirmation

1. Generate an independent `BrowserBinding` on the server. Keep it in a dedicated
   `Secure; HttpOnly; SameSite=Lax` cookie, scoped to the application, with a
   maximum age of 15 minutes. Protect the request endpoint with CSRF and ingress
   limits. Never accept a browser binding chosen through a link or request body.
2. Call `request_login` with the email, binding and trusted server clock. Known,
   unknown, disabled, suppressed and throttled accounts receive the same
   `LoginRequestAccepted`. Each namespace allows 120 requests per minute and
   each account three per 15 minutes. A 250 ms minimum acknowledgement delay
   mitigates trivial enumeration; it is not a constant-time guarantee under
   database contention or faults. Rate-limit at ingress independently.
3. Issuance creates a distinct random 256-bit emailed token, stores domain-bound
   HMAC digests of both secrets, binds the account epoch and policy revision,
   and atomically queues an AES-256-GCM encrypted notice. A new accepted request
   replaces the prior link and pending/leased notice. Links expire after exactly
   15 minutes. Account opt-out, password recovery or another account-epoch
   change invalidates the pending link.
4. A trusted worker calls `claim_notice`, constructs Mail `AccountEvent::EmailLoginAt`
   with `ActionLink`, the server-configured origin and `expires_at`, then sends
   with the stable `delivery_id`. Use the recorded locale, with explicit fallback.
   Call `complete_notice` or `fail_notice` with a redacted delivery outcome. Claims
   last at most 60 seconds, allow six attempts and use bounded backoff. Delivery
   is at-least-once; transports without idempotency may send duplicates. A bounce
   or complaint suppresses the account's future transactional delivery.
5. The landing **GET/HEAD only renders confirmation**. It never consumes the
   token or creates a session. An explicit CSRF-protected POST passes the emailed
   token and cookie binding to `redeem`. The initiating browser is required;
   opening the email on another device requires starting a new request there.
   Never redeem automatically in JavaScript or on page load.
6. Redemption atomically consumes the link and creates a one-hour opaque session
   using the existing account epoch, 20-session limit and inventory/revocation
   machinery. Store it in a `Secure; HttpOnly; SameSite` cookie, clear the browser
   binding, and redirect only to `EmailLoginSession::destination()`. Verify the
   session and authorization on every request. Logout and account revocation
   remain authoritative across instances sharing the database.

Use the framework security baseline (CSRF, WAF and secure headers), bounded
request bodies, `Cache-Control: no-store` and `Referrer-Policy: no-referrer` on
all landing and authentication responses, including errors. Do not load
third-party scripts, analytics, fonts or images on the landing page. Redact
query strings, cookies and mail bodies before request logging or tracing. Never
serialize the returned session/delivery capabilities into application events.
The test-only `/fixture/mail` route in the HTTP acceptance harness substitutes
an inbox; it must never be mounted in a deployed application.

The Mail template has deterministic English, Brazilian Portuguese and Spanish
HTML/text, absolute expiry, a same-browser confirmation instruction and no
tracking pixel. It neither generates tokens nor changes authentication state.

## Storage, cancellation and operations

SQLite uses one private connection per instance, WAL, FULL synchronization and
one shared local file. It is not a multi-host database. The deployment owns the
trusted directory, file permissions, encryption-at-rest and backup. PostgreSQL
uses a bounded four-connection pool, verified remote TLS, five-second SQL/lock
limits, permanent tables, `fsync`/`full_page_writes` and synchronous commit on a
writable primary. Public methods have a ten-second operation timeout (plus the
minimum request delay). All state mutations serialize through the account
registry's write lock and recheck clock observations. A lock wait cannot extend
a link's expiry. An uncertain commit or cancellation never returns a session.

PostgreSQL bootstrap serializes explicit initializers. Normal startup performs
no DDL or missing-namespace repair. Runtime needs SELECT/INSERT/UPDATE/DELETE on
the account/session/outbox and email-login tables in the trusted `public` schema;
reserve CREATE/ALTER for the deployment role and revoke schema CREATE from
untrusted roles. Use one authoritative writable database, synchronized clocks,
replication fencing and a bounded number of server-configured namespaces.
Configuration/key drift and non-durable PostgreSQL tables fail closed.

Delivered/terminal email ciphertext is deleted immediately. Run `purge_expired`
from a trusted retention worker even during idle periods; it removes expired
links and notices. Invalidated encrypted notices that were already claimed may
remain until expiry, and in-flight transport work cannot be recalled. Such links
fail redemption. No recipient activity history is recorded by this component.

A stale database restore can revive consumed links, account epochs and revoked
sessions. Quiesce authentication, invalidate restored login challenges/outbox
and session state, reconcile account epochs and policies, then resume. This
adapter does not provide backup anti-rollback, high availability, secret rotation
or a production mail delivery SLA.

## Automated evidence

The candidate includes SQLite/PostgreSQL independent-pool races, purpose and
namespace isolation, browser mismatch, replacement, opt-out/epoch invalidation,
expiry/clock rollback, locked-database expiry, cancellation, SQL failure
atomicity, bounded retries and suppression. Process tests use fresh executables;
the owned PostgreSQL wrapper also restarts the database service. The Chromium
HTTP fixture uses the actual framework security baseline and proves scanner
GET/HEAD, CSRF rejection, deliberate POST, secure cookies, tenant authorization,
replay rejection and logout across separate database pools.

Run the ordinary SQLite contracts with `cargo test -p rullst-auth --features
email-login-sqlite`. Run the real browser/service contracts with
`RULLST_EMAIL_LOGIN_BROWSER_TESTS=1 python3 .github/check-auth-recovery-postgres.py`.
The wrapper owns a disposable loopback Docker database; it does not use external
accounts. Packaged-distribution acceptance exercises extracted archives through
the facade. These automated fixtures do not establish live mail-provider or
owner-application acceptance.
