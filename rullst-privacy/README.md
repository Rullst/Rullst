# rullst-privacy

Unpublished v13 foundation for proportional age assurance. This package is a
workspace member with `publish = false`; it is not part of the v12 release
inventory or the default `rullst` dependency graph.

Enable `age-assurance` to use the current contract. No feature is enabled by
default. Broader consent, rights-request, retention and regional-policy support
is tracked in the [privacy roadmap](../docs/src/privacy-age-assurance-roadmap.md).

## Current boundary

- Server-owned risk policies, age thresholds and method-specific challenges.
- Native first-party declarations through `DeclarationGate`, without an external
  issuer; an affirmative answer remains `Assurance::Declared` and cannot satisfy
  a stronger method or policy.
- Random, expiring challenges bound to opaque subject, tenant, session, audience
  and action references, including the complete policy configuration.
- At-most-4-KiB versioned JSON attestations, Ed25519 signatures, explicit issuer
  capabilities and up to eight pinned keys for rotation.
- Declared, estimated, verified-attribute and offline-mock assurance remain
  distinct. Below-margin facial results require an alternative method.
- Asynchronous one-use consumption through a static-dispatch replay store; production rejects
  process-local stores and mocks. The supplied memory store is bounded and
  refuses capacity exhaustion rather than evicting valid claims.
- Opt-in shared-local SQLite claims with persisted quota, clock rollback checks,
  atomic expiry/consumption and minimized nonce digests. No database is enabled
  by the base or `age-assurance` feature.
- Opt-in PostgreSQL claims shared across application hosts using one writable
  database, with explicit initialization, serialized quota/consumption, verified
  remote TLS and admission checks for logged tables and durable server settings.

There is no facial model, image capture, live vendor SDK, document recognition,
guardian verification or automatic database failover here. An issuer
signature establishes authenticity of its assertion; it does not establish the
quality of the age determination. Signing an untrusted browser result does not
turn it into verified evidence.

## Risk presets

| Risk | Accepted methods |
| :--- | :--- |
| Low | Explicit self-declaration, evaluated facial estimation, verified attribute |
| Elevated | Evaluated facial estimation or verified attribute |
| Restricted | Verified attribute |

These are conservative engineering presets, not legal age categories. The host
chooses an applicable minimum age and a justified policy. Services that need no
age gate should not collect age evidence. For an estimator, configure a
challenge margin justified by model/audience evaluation; the three-year default
is not accuracy evidence. The default challenge TTL is five minutes, capped at
fifteen. A threshold comparison cannot replace guardian authorization.

## Integration flow

1. Authenticate the caller and resolve tenant/session/action references on the
   server. Use pairwise opaque references rather than emails or raw session tokens.
2. Construct `AgePolicy` and issue `AgeChallenge` for a permitted method. Keep
   the challenge server-side. Send `request_json()` only to the selected issuer
   through an authenticated, bounded integration.
3. A reviewed issuer or provider bridge determines the requested predicate. For
   facial estimation, this includes capture anti-spoofing/liveness and measured
   threshold performance. The bridge verifies the vendor's native protocol,
   session binding, method and data lifecycle before producing an attestation.
4. The issuer signs `signing_message(payload)` using Ed25519. The payload is the
   JSON produced by `encode_attestation`; a bridge can use the same wire shape
   from `request_json()`. Never sign a client-provided outcome without checking it.
5. `AgeVerifier::new(issuer, shared_store)` verifies the exact payload bytes,
   retained challenge, current policy, authenticated binding and trusted server
   time, then asynchronously consumes the nonce. Await `verify(...)`; it samples
   the server clock again after storage and rejects intervening expiry or clock
   rollback. `verify_with_clock(...)` supports an explicit trusted clock, never
   a client timestamp. Gate the specific action only on
   `AgeDecision::Allowed`. Every error denies the gated operation.

`ReplayStore` implementations must share durable atomic nonce claims across
instances, retain claims until expiry, reject uncertain commits and prevent
rollback from resurrecting consumed claims. The durability enum is an adapter
contract, not an automatic assessment of its implementation. No production
provider deployment evidence is claimed for this crate yet.

## Native first-party declarations

`DeclarationGate` processes an explicit authenticated `AgeDeclaration` when the
server's policy allows `SelfDeclaration`. It needs no external issuer or signing
key for that answer. Retain the server-issued challenge and resolve the current
authenticated binding again on submission; never substitute client JSON for
either. The host must protect the endpoint with CSRF and request limits.

```rust,no_run
# #[cfg(feature = "sqlite")]
# async fn declared_age(
#     policy: &rullst_privacy::age_assurance::AgePolicy,
#     binding: &rullst_privacy::age_assurance::SubjectBinding,
#     retained: &rullst_privacy::age_assurance::AgeChallenge,
#     answer: rullst_privacy::age_assurance::AgeDeclaration,
# ) -> Result<(), rullst_privacy::age_assurance::AgeError> {
use rullst_privacy::age_assurance::{AgeDecision, DeclarationGate, SqliteReplayStore};

let store = SqliteReplayStore::open("/private/app/age-replay.sqlite", 10_000).await?;
let gate = DeclarationGate::new(store)?;
let assessment = gate.assess(policy, binding, retained, answer).await?;
if assessment.decision() == AgeDecision::Allowed {
    // Execute only the action authorized by this current authenticated context.
}
# Ok(())
# }
```

Create and reuse the store/gate at application startup. The example accepts the
policy, binding and retained challenge from trusted server state, and the answer
from an explicit user choice. `MeetsThreshold` may allow the action;
`BelowThreshold` denies it; `Declined` requires an appropriate alternative.
Every accepted answer consumes the challenge, including negative or declined
answers. Missing or unknown values are not affirmative declarations.

Production rejects process-local replay state; both SQLite and PostgreSQL can
be used within their documented deployment boundaries. Signed and native paths
share the same replay namespace. Changing paths cannot consume one challenge
twice. The result is always `Assurance::Declared`, never estimated or verified
age. Stronger-method challenges are rejected even if the browser sends an
affirmative declaration. This API does not supply a complete authenticated web
journey or challenge storage/transport by itself.

## Shared-local replay storage

Enable `sqlite` for `SqliteReplayStore`. It uses a private file-backed SQLx pool,
WAL/full synchronization and serialized write transactions. All verifiers on
one host must open the same operator-owned local file with the same capacity
(1..=100,000 live claims). It persists schema, quota and the greatest accepted
claim time; unexpired claims are never evicted to admit another proof.

```rust,no_run
# #[cfg(feature = "sqlite")]
# async fn setup(issuer: rullst_privacy::age_assurance::TrustedIssuer) -> Result<(), rullst_privacy::age_assurance::AgeError> {
use rullst_privacy::age_assurance::{AgeVerifier, SqliteReplayStore};

let store = SqliteReplayStore::open("/private/app/age-replay.sqlite", 10_000).await?;
let verifier = AgeVerifier::new(issuer, store)?;
# let _ = verifier;
# Ok(())
# }
```

The parent directory must already exist. Database URLs, memory-only stores,
existing symlinks and non-regular targets are rejected. The host must prevent
untrusted directory/file replacement; checking the final path does not defeat
a hostile filesystem race. Capacity bounds rows, not total filesystem use.
Logical expiry deletion is not physical erasure of WAL pages or backups.

Cancelled or uncertain writes grant no access and may have consumed the proof.
Request fresh evidence after an uncertain outcome. Clock rollback fails closed;
out-of-order requests or skewed processes may need a retry with current server
time. Do not lower the persisted clock or clear state to work around this error.
Network filesystems, cross-host replication and PostgreSQL are outside this
SQLite adapter's scope. Storage hardware must honor SQLite's durability guarantees.

## PostgreSQL replay storage across application hosts

Enable `postgres` for `PostgresReplayStore`. It connects every verifier to the
same authoritative writable PostgreSQL database. The private pool caps itself
at four connections with five-second acquisition, statement, lock and idle
transaction timeouts. These are per-stage bounds; set an overall request
deadline at the host boundary. All queries use fixed schema-qualified names and
bound values. Errors omit SQL error details and connection credentials.
Connection URLs reject fragments and unknown/repeated query keys before SQLx
can log unrecognized option values. Arbitrary `options[...]` startup settings
are not accepted through this adapter's URL.

```rust,no_run
# #[cfg(feature = "postgres")]
# async fn setup(database_url: String) -> Result<(), rullst_privacy::age_assurance::AgeError> {
use rullst_privacy::age_assurance::PostgresReplayStore;

// Deployment step: creates only an absent schema; never resets existing state.
let initialized = PostgresReplayStore::initialize(database_url.clone(), 10_000).await?;
initialized.close().await;

// Application startup: requires the initialized schema and matching capacity.
let store = PostgresReplayStore::connect(database_url, 10_000).await?;
# store.close().await;
# Ok(())
# }
```

Initialization uses a transaction-scoped bootstrap lock. Claims lock the single
metadata row and perform clock checks, expiry pruning, capacity checks and nonce
insertion in one transaction. The 1..=100,000 quota covers the whole schema,
across all application pools and tenants; it is not a per-host allowance.
Concurrent claims are serialized, so this is a bounded baseline, not a measured
high-throughput distributed service. Runtime roles need schema `USAGE`,
`SELECT`/`UPDATE` on `metadata` and `SELECT`/`INSERT`/`DELETE` on `claims`;
schema creation belongs to the deployment role.

The store rejects missing or mismatched metadata, incomplete schema, unlogged
tables and disabled `fsync`/full-page writes. Its sessions require synchronous
commit even when the database default disables it. Remote TCP connections
enforce certificate and hostname verification; configure the trusted root
certificate for a private CA. Local sockets and loopback TCP support disposable
development databases without mandatory TLS. Configuration checks have local
tests; deployment-specific certificates and server operations require their own
acceptance. The implementation follows PostgreSQL's
[row-lock contract](https://www.postgresql.org/docs/current/explicit-locking.html#LOCKING-ROWS)
and [WAL durability settings](https://www.postgresql.org/docs/current/runtime-config-wal.html).

Operators must prevent unauthorized schema/state mutation, synchronize trusted
host clocks and maintain one authoritative writer with durable storage. Replica
promotion, replication acknowledgement policy, failover fencing and backup
restores are external deployment responsibilities. A successful local commit
does not certify an asynchronously replicated failover target. Neither adapter
can detect arbitrary database rollback from that same database alone.

## Recovery and application boundaries

Restoring an old backup can resurrect claims. Before resuming after a restore,
quiesce all verifiers, discard outstanding challenges and enforce a new policy
version everywhere (or retire all old signing keys). Merely adding a key while
retaining the old key does not invalidate its proofs. Backup rollback cannot
be detected reliably from that same database alone.

The host owns clock synchronization, challenge storage/quotas, request limits,
timeouts/cancellation for external capture, endpoint CSRF/authorization,
provider trust/key revocation, domain idempotency, accessible alternatives and
appeals. Returning an assessment does not commit the application's action;
handle uncertain domain outcomes without reusing the proof. An assessment is
not a transferable or reusable bearer credential.

## Offline example

```rust
use rullst_privacy::age_assurance::{
    AgeChallenge, AgeMethod, AgePolicy, RiskLevel, SubjectBinding,
};

let policy = AgePolicy::new("academy-review-v1", RiskLevel::Elevated, 18)?
    .with_estimation_margin(5)?;
let binding = SubjectBinding::new(
    "pairwise-user", "school-1", "opaque-session", "academy", "restricted-action",
)?;
let challenge = AgeChallenge::issue(&policy, binding, AgeMethod::FacialEstimation, 1000)?;
assert_eq!(challenge.threshold(), 23);
# Ok::<(), rullst_privacy::age_assurance::AgeError>(())
```

`MockAgeProvider::new("mock_local", outcome)` or empty credentials selects a
deterministic fixture. It is accepted only by `AgeVerifier::for_development`,
and its assessment retains `Assurance::OfflineMock`. It never processes a face.

## Privacy and verification

The wire format contains only bounded references, policy, method, threshold,
nonce, times and a predicate result; unknown fields are rejected. It contains
no selfie or birth date. Those opaque references still represent personal data
and must not enter ordinary logs. Debug output redacts subject/challenge data.
Provider capture, temporary storage, erasure, training restrictions and legal
basis require their own review. This library cannot certify worldwide compliance.

Run `cargo test -p rullst-privacy --all-features` and
`cargo clippy -p rullst-privacy --all-features --all-targets -- -D warnings`.
Run `python3 .github/check-privacy-postgres.py` from the workspace root for the
mandatory real PostgreSQL contract. It creates only its own disposable loopback
database, exercises the explicitly ignored database test and a fresh client
process, interrupts/restarts that server and checks persisted consumption again.
The standard suite compiles that test but does not provide its database evidence.
CI's strict PostgreSQL job runs the wrapper; coverage uses its `--coverage` mode.
The suite covers policy strength, method capability, signatures, context/policy
swaps, expiry before/after storage, cancellation, clock rollback, independent
SQLite pools, a fresh-process replay check, reopen, concurrent quota, schema drift, failed inserts, lost commit
acknowledgements and mock separation. PostgreSQL exercises two application pools,
concurrent bootstrap, a restricted runtime role, quota/expiry, persisted clock,
schema/unlogged-table drift, disabled durability, failed writes, cancellation,
terminated connections and expiry/rollback during an actual database lock wait.
These are local executable contracts;
they do not establish power-loss recovery on a deployment's hardware, arbitrary
backup rollback detection or a real provider's accuracy.
