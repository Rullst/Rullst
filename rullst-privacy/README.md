# rullst-privacy

Unpublished v13 release candidate for proportional age assurance and optional
consent. The source package joins the v13 distribution inventory, subject to
package validation, initial registration and publication configuration. It is
not part of v12 or the default `rullst` dependency graph.

Enable `age-assurance` for age checks or `consent` for independent purpose-bound
choices. No feature is enabled by default. Broader rights-request, retention and regional-policy support
is tracked in the [privacy roadmap](https://github.com/Rullst/Rullst/blob/v13/docs/src/privacy-age-assurance-roadmap.md).

## Optional umbrella features

The native `rullst::privacy` facade is available with explicit features:

| Umbrella feature | Privacy feature |
| :--- | :--- |
| `privacy` | Empty base; no implicit age or consent controls |
| `privacy-age` | `age-assurance` |
| `privacy-challenge-tokens` | `challenge-tokens` |
| `privacy-sqlite` | `sqlite` age replay storage |
| `privacy-postgres` | `postgres` age replay storage |
| `privacy-consent` | `consent` |
| `privacy-consent-sqlite` | `consent-sqlite` |

Enabling a feature does not select a policy, create state or mount routes.
The standalone package remains independent of Core and the umbrella.

## Optional-processing consent

The v13 CLI's explicit
[`make:privacy` consumer](https://github.com/Rullst/Rullst/blob/v13/docs/src/cli_reference.md#cargo-rullst-makeprivacy-unpublished-v13-preview)
composes these controls with the recognized SaaS/full LMS authentication and
school membership. It supplies preferences, an optional personalized greeting,
and an independent direct JSON export of only the current account's ID, name
and email. The CLI selects its matching registry version unless an explicit
`--privacy-source` selects local development source. Before publication, use that
override or a reviewed archive patch. The generated `PRIVACY.md` specifies setup
and remaining application duties.

The independent `consent` feature provides typed purpose/notice versions,
authenticated subject/tenant bindings, explicit grant/refusal/withdrawal and a
static-dispatch store contract. It has no age, crypto, database or Core dependency.
This is an engineering control for optional processing, not a selection of its
lawful basis, guardian authority or a certificate of worldwide compliance.

- No record, refusal, withdrawal, expiry or a different notice version denies
  processing. The operator must not reuse retired notice versions.
- `ConsentSubmission` carries the purpose/version actually displayed and the
  revision shown in that form. A new notice requires a new explicit choice even
  when the stored revision has not changed.
- `choose` compares that revision atomically. `withdraw` advances it without a
  stale-form precondition and covers the same purpose across versions. A delayed
  affirmative response cannot undo an acknowledged withdrawal. A fresh explicit
  choice can grant again.
- `allows` reads authoritative state for each processing action. Deferred jobs
  must check again at execution; a queued job is not lasting permission. The
  check linearizes at the store read and cannot cancel an already-started external
  effect. Stronger atomicity between consent and domain effects belongs to the
  application's transaction/processor contract.
- Grant expiry is an explicit server choice, bounded to at most 365 days as an
  engineering limit. It is not a legal retention period or a default grant.
  Clock checks reject rollback and expiry during storage; stores retain a clock
  high-water mark. Records and debug output do not expose application profiles.

`ConsentGate::new` requires shared durable state. `MemoryConsentStore` is bounded
development-only storage, accepted by `for_development`. The `consent-sqlite`
feature supplies `SqliteConsentStore` without enabling age assurance. Deployment
initializes a **new** file once with `initialize`; application startup uses
`open`, which never creates or repairs missing state. It uses a private bounded
pool, WAL/full synchronization, serialized reads/revision updates, an immutable
quota and persistent clock checks. All application processes must use the same
trusted local file. The stored scope digest remains pseudonymous personal data.

Withdrawals and expired records are not evicted to make space. Choose capacity
for the number of subject/tenant/purpose combinations. The operator owns a
trusted directory, permissions, synchronized time, storage durability and an
application-level timeout. Remote filesystems and multi-host replication are
unsupported. A stale backup can restore old grants: stop optional processing,
restore/reconcile withdrawals and move every active purpose to a fresh notice
version before resuming. No automatic erasure or backup rollback detection is
claimed. A failed/cancelled bootstrap may leave a partial file for explicit
operator recovery; normal startup must not remove it.

```rust,no_run
# #[cfg(feature = "consent-sqlite")]
# async fn example() -> Result<(), rullst_privacy::consent::ConsentError> {
use rullst_privacy::consent::*;
let store = SqliteConsentStore::open("private/consent.sqlite3", 10_000).await?;
let gate = ConsentGate::new(store)?;
// Resolve these opaque references from authenticated server state.
let subject = ConsentSubject::new("account-ref", "tenant-ref")?;
let purpose = ConsentPurpose::new("optional-digest", "notice-v1")?;
// Render the exact notice and current revision before accepting an explicit
// response. Enforce authentication/CSRF and read the displayed version from it.
let current = gate.current(&subject, &purpose).await?;
let response = ConsentSubmission::new(purpose.clone(), current.revision(), ConsentChoice::Granted)?;
let expiry = SystemConsentClock.now()?.checked_add(3600).ok_or(ConsentError::InvalidConfiguration)?;
gate.choose(&subject, &purpose, &response, expiry).await?;
if gate.allows(&subject, &purpose).await? {
    // Perform this one optional action under ordinary authorization too.
}
gate.withdraw(&subject, &purpose).await?;
# Ok(())
# }
```

Generated SaaS/LMS consumer tests exercise authenticated preferences, withdrawal
and the bounded own-account profile export. Deployment-specific acceptance and
rights workflows across the rest of an application remain separate.

## Current boundary

- Server-owned risk policies, age thresholds and method-specific challenges.
- Native first-party declarations through `DeclarationGate`, without an external
  issuer; an affirmative answer remains `Assurance::Declared` and cannot satisfy
  a stronger method or policy.
- Random, expiring challenges bound to opaque subject, tenant, session, audience
  and action references, including the complete policy configuration.
- Opt-in authenticated challenge transport with bounded HMAC-SHA256 keys and
  explicit rotation, for restoring challenges on another application instance.
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
affirmative declaration. The gate alone does not supply an authenticated web
journey; optional challenge transport is described below.

## Authenticated challenge transport

Enable `challenge-tokens` to return a server-issued challenge through a browser
form or between trusted application instances. `ChallengeTokens` authenticates
the version, key identifier and exact payload with HMAC-SHA256 before decoding
JSON, then validates the current policy, authenticated binding and server clock.
It rejects unknown keys, extra fields, changed lifetime and oversized tokens.
The token limit is 8 KiB and the decoded challenge limit is 4 KiB.

```rust,no_run
# #[cfg(feature = "challenge-tokens")]
# fn transport(
#     secret_from_key_manager: &[u8],
#     policy: &rullst_privacy::age_assurance::AgePolicy,
#     authenticated_binding: &rullst_privacy::age_assurance::SubjectBinding,
#     challenge: &rullst_privacy::age_assurance::AgeChallenge,
# ) -> Result<(), rullst_privacy::age_assurance::AgeError> {
use rullst_privacy::age_assurance::ChallengeTokens;

let tokens = ChallengeTokens::new("epoch-2", secret_from_key_manager)?;
let form_token = tokens.seal(challenge)?;
// On submission, resolve the current authenticated binding again on the server.
let retained = tokens.open(&form_token, policy, authenticated_binding)?;
// Pass retained to DeclarationGate::assess or AgeVerifier::verify and await it.
# let _ = retained;
# Ok(())
# }
```

Provision an independent high-entropy 32-byte secret shared only by trusted
application instances. `with_previous_key` accepts at most seven historical
verification keys alongside the active key; omit retired keys from the next
configuration. There is no default secret, online discovery or client-selected
algorithm. Opening a token neither consumes its nonce nor grants permission.
The gate/verifier must still consume it through the shared durable replay store.

Tokens are authenticated, **not encrypted**. Use opaque pairwise references;
never include raw cookies, email addresses or document numbers. Protect the
form with authentication, CSRF, TLS, request limits and no-store responses, and
keep tokens out of URLs and logs. A changed policy or session invalidates the
old challenge. No external age provider is required for this transport.

The v13 CLI preview supplies an optional
[`make:age-gate` SaaS/LMS consumers](https://github.com/Rullst/Rullst/blob/v13/docs/src/cli_reference.md#cargo-rullst-makeage-gate-unpublished-v13-preview).
It mounts a declaration before the existing authenticated dashboard rendering,
with explicit server policy, CSRF and durable one-use consumption. Before registry
publication, use the matching local source override or archive patch. The LMS profile binds the
school resolved by current authenticated membership; changing school invalidates
the challenge, and a declaration changes no guardian or subject-age record.
Other app actions and stronger
assurance methods retain their own authorization/integration requirements.

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
