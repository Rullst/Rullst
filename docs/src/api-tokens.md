# Scoped application API tokens (v13 candidate)

Auth features `api-tokens-sqlite` and `api-tokens-postgres` expose
`recovery::api_tokens::ApiTokenService`. Facade features are
`auth-api-tokens-sqlite` and `auth-api-tokens-postgres`. They reuse authoritative
recovery accounts and do not enable email login, JWT or OAuth. This increment
is an implementation candidate; full hosted/source/package admission is pending.

## Create and manage credentials

Configure an immutable server-owned `ApiTokenConfig`: namespace, exact scope
allowlist, capacity (1–100,000 rows) and maximum lifetime (1 second–30 days).
Scopes are explicit, case-sensitive names such as `orders:read`: 1–32 distinct
names, each 1–64 lowercase ASCII letters, digits or `:._-`, starting with a
letter. Wildcards and implied parent scopes are unsupported. Use `initialize`
as a deployment task and `connect` for normal startup without DDL or repair.

1. Authenticate the account through `service.accounts().authenticate`. The
   resulting proof belongs to this exact registry instance and account epoch.
   The application must additionally require recent authentication, its MFA
   policy and current tenant/domain permissions. Intersect requested scopes
   with those permissions before calling `issue`; the configured allowlist
   alone is not an account authorization policy.
2. `issue` accepts the proof, approved scopes, a bounded `SessionLabel`, lifetime
   and trusted `SystemAuthClock`. Return `IssuedApiToken::expose_bearer` once over
   HTTPS with `Cache-Control: no-store`. Store it on the integration side in a
   secret store, never in URLs, analytics, browser local storage or application
   logs. Management endpoints retain CSRF, secure headers and ingress limits.
3. `inventory` returns this account's active metadata and opaque management IDs,
   never secrets or digests. Each account has at most 20 active credentials per
   namespace, within the deployment capacity. No IP, user-agent or usage history
   is collected. Run `purge_expired` even when issuance is idle.
4. `rotate` requires the exact current revision. It keeps ID/scopes/creation time,
   replaces the secret, increments the revision and optionally renews the expiry
   within policy. Only one concurrent rotation succeeds. Revoked or expired
   credentials cannot be rotated. A lost/uncertain rotation response requires
   authenticated inventory/revocation and a new rotation or separately approved
   issuance; the service cannot recover the secret.
5. `revoke` is owner-scoped and idempotent; foreign and absent IDs return false.
   `revoke_all` affects this account in this namespace. Recovery/account-epoch
   invalidation rejects all older API credentials and management proofs.

Each wire credential contains a versioned `rlt1_` prefix, random 256-bit management
ID and independent random 256-bit secret. Only a purpose/namespace-bound HMAC
is persisted. A management ID, provider key or browser-session token cannot be
used as this bearer. Credential/principal Debug output is redacted.

## Authenticate each request

Call `verify` for every request with the route's required `ApiScopes` and trusted
server clock. It reads current SQL state, compares the HMAC in constant time,
checks scope inclusion, account epoch and expiry, and fails on storage faults.
Resolve current tenant membership, account roles and resource ownership after
verification. An `ApiTokenPrincipal` is a snapshot, not an authorization cache
or a credential for issuing more tokens.

For exact mutating machine routes, compose
`service.machine_verifier(required_scopes)` with
`MachineEndpoint::verified_bearer(Method::POST, "/api/orders", verifier)` and
the Core security baseline. The verifier requires one `Authorization: Bearer`
header and inserts the principal as an Axum extension. Cookies, Origin and
browser fetch-site headers cannot substitute for machine authentication.
Only the verified exact method/path receives the machine CSRF exception;
ordinary browser routes retain CSRF, and WAF/headers remain active. Apply bounded
bodies, ingress rate limits and no-store responses. GET/read routes can call
`verify` from their own authenticated handler; this constructor is deliberately
limited to the existing mutating machine-route contract.

The executable `api_token_http` fixture composes two independent database pools,
real HTTP servers, the actual security baseline, a required write scope and
tenant/owner authorization before a side effect. It proves body preservation,
missing/duplicate/ambient credentials, wrong scope/tenant/account, route
confusion, rotation, revocation and storage-outage rejection.

## Storage and operations

SQLite uses one private connection, WAL and FULL synchronization on a trusted
local file; it is not a multi-host store. PostgreSQL uses one authoritative
writable database, verified remote TLS, four connections and five-second SQL
deadlines. Public operations have ten-second timeouts. Runtime requires USAGE
on the trusted `public` schema and SELECT/INSERT/UPDATE/DELETE on the recovery
and API-token tables; reserve DDL for the deployment role. The adapter checks
permanent tables and durable PostgreSQL settings on every operation.

Token operations serialize with recovery/account changes through the existing
registry write lock. This favors explicit revocation ordering over unlimited
parallel throughput; size and load-test this shared registry for the application.
Expiry and persisted clock observations are rechecked after database waits and
before a credential or principal is returned. Cancellation/uncertain commit
never returns a new bearer. Revocation has a database ordering point; work
already authorized before it cannot be recalled or made atomic with a later
external side effect. Never replace SQL checks with positive local caching.

The deployment owns synchronized clocks, key storage, database encryption,
replication fencing and a bounded number of configured namespaces. Rotating a
configuration/key requires a deliberate migration; mismatched instances fail
closed. A stale backup can revive deleted tokens and old account epochs. Quiesce
authentication, invalidate restored tokens and reconcile epochs before resuming.
The adapter does not implement backup anti-rollback or automatic failover.

Run `cargo test -p rullst-auth --features api-tokens-sqlite` for ordinary local
contracts and `python3 .github/check-auth-recovery-postgres.py --suite api-tokens`
for the owned disposable PostgreSQL, HTTP, restricted-role, durability and
process/server-restart contracts. The package campaign repeats the lifecycle
through extracted facade/Auth/Core archives. No real provider account is needed.
