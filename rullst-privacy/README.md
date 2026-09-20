# rullst-privacy

Unpublished v13 foundation for proportional age assurance. This package is a
workspace member with `publish = false`; it is not part of the v12 release
inventory or the default `rullst` dependency graph.

Enable `age-assurance` to use the current contract. No feature is enabled by
default. Broader consent, rights-request, retention and regional-policy support
is tracked in the [privacy roadmap](../docs/src/privacy-age-assurance-roadmap.md).

## Current boundary

- Server-owned risk policies, age thresholds and method-specific challenges.
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

There is no facial model, image capture, live vendor SDK, document recognition,
guardian verification or multi-host replay backend here. An issuer
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
adapter's scope. Storage hardware must honor SQLite's durability guarantees.

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
The suite covers policy strength, method capability, signatures, context/policy
swaps, expiry before/after storage, cancellation, clock rollback, independent
SQLite pools, a fresh-process replay check, reopen, concurrent quota, schema drift, failed inserts, lost commit
acknowledgements and mock separation. These are local executable contracts;
they do not establish power-loss recovery on a deployment's hardware, arbitrary
backup rollback detection or a real provider's accuracy.
