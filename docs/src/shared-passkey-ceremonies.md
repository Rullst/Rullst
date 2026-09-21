# Shared passkey ceremonies

This unpublished v13 implementation passed real-PostgreSQL and Chromium
contracts, hosted workspace checks and isolated-archive acceptance in
[PR #223](https://github.com/Rullst/Rullst/pull/223). The final release campaign
remains separate; this is not a WebAuthn conformance claim. The existing synchronous
`PasskeyAuth` API retains its process-local, single-use challenges. The existing
SQLite credential registry persists credentials and counter CAS, not ceremonies.

The optional path belongs in `rullst-auth`. Enable `passkey-postgres`, or
`auth-passkey-postgres` through the umbrella package. `SharedPasskeyAuth<S>` composes a concrete
`PasskeyCeremonyStore` through static dispatch and the existing ES256/`none`
verification engine. The first real adapter is opt-in PostgreSQL, because it
serves separate application hosts. A shared SQLite ceremony adapter, additional
algorithms/attestation formats and discoverable-account login are separate work.
No blanket blueprint rewrite or new public crate is proposed.

## Identity and API boundary

A server-created `PasskeyBinding` carries bounded opaque tenant, subject and
session references, plus a stable application-owned opaque user handle of
1–64 bytes. Constructing it validates shape; it does not authenticate a caller,
establish account ownership or authorize registration. Registration requires an
authenticated account or the application's separately authorized enrollment flow.
For account-first login, bind the selected authoritative account and a random
pre-authentication session; the challenge alone must never choose that account.
Protect both flows with CSRF/origin and abuse controls, and refresh authorization
before persisting a credential or creating a session.

Each challenge binds a framed digest of that context, the normalized RP ID and
origin, user-verification requirement and deployment epoch. Registration options
use the supplied user handle, not an email or numeric account ID. Account-first
authentication requires 1–32 current, host-owned credentials; a digest of each
credential ID, public key and current counter is stored with the challenge.
Registration and authentication are distinct purposes. Finishing cannot change
the account, tenant, session, RP configuration, user handle or credential set.
The initial response subset does not implement discoverable account selection.
Forward the authenticator's optional `userHandle` to `finish_authenticate`; when
present it must match the bound account handle.

Options use fresh 256-bit random challenges. Responses have explicit size limits
before decoding. The private crypto validation helpers are shared with the
existing API; no public stateless verification bypass is introduced. A finish
checks client challenge/origin/type, atomically consumes the bound store entry,
then validates the authenticator response and checks expiration again before
returning. Failed verification after consumption requires a new challenge.
Successful assertion verification returns the updated public credential. The
host must atomically persist its counter against the previous value and check
current revocation/ownership before granting a session; this increment does not
invent a distributed transaction with the application's credential repository.

## Durable store boundary

`PasskeyCeremonyStore` is a trusted implementation contract. It exposes bounded
issue, atomic bound consume and completion-expiry validation, a stable persisted
configuration and explicit durability. `SharedPasskeyAuth` rejects a store that
declares process-local durability. A declaration by a custom implementation is
not an independent durability certification.

Configuration supplies a separately retained opaque deployment epoch, a maximum
of 1–100,000 pending challenges and a 1–600 second lifetime. PostgreSQL owns one
fixed private schema with metadata and pending rows. All operations validate
configuration and a persistent clock high-water mark under a transaction-level
metadata lock. Clocks are trusted server clocks; hosts must synchronize them.
A backward clock fails closed. Challenge and identity references are digested;
rows contain purpose, bounded credential fingerprints and issued/expiry times,
not passwords, private keys, display names, raw sessions or response bodies.

Initialization is explicit and never repairs missing runtime state. Connections
use a private bounded pool, primary durable tables, synchronous commits, a fixed
search path and bounded acquisition, lock and statement waits. A twelve-second
whole-operation deadline also bounds startup and issue/consume/confirm when a
transport stops returning replies; timeout reports an uncertain outcome. Non-loopback TCP
requires TLS certificate/hostname verification. Issue serializes quota checks,
bounded expired-row cleanup and insertion. Consume compares all bindings and
purpose, validates the bounded stored row and deletes it in the same transaction.
A mismatched context cannot remove another ceremony. There is one winner across
pools and processes. No unexpired challenge is evicted to make room.

Cancellation before commit rolls back. An uncertain commit or a deadline crossed
while waiting/committing returns an error; a consumed challenge may remain spent.
The host retries with a new ceremony, never a local fallback. Expired rows may be
removed logically; backups and physical erasure remain deployment concerns.
Operators own schema permissions, synchronized clocks, TLS trust, encryption,
backups and failover fencing. A stale restore with the same epoch can resurrect
challenges; quiesce authentication and rotate the independently held epoch with
fresh state before resuming. This adapter cannot detect physical rollback alone.

## Configuration example

Initialize `PostgresCeremonyStore` once with a deployment role. Application
startup uses `connect` and cannot recreate missing state. Keep the same epoch,
capacity and lifetime on every participating instance:

```rust,no_run
use rullst_auth::passkey::{PasskeyConfig, shared::{
    CeremonyStoreConfig, PostgresCeremonyStore, SharedPasskeyAuth,
}};

async fn configure(url: String) -> Result<
    SharedPasskeyAuth<PostgresCeremonyStore>, Box<dyn std::error::Error>
> {
    let policy = CeremonyStoreConfig::new("independently-held-epoch", 10_000, 300)?;
    let store = PostgresCeremonyStore::connect(url, policy).await?;
    let rp = PasskeyConfig::new("Example", "example.com", "https://example.com");
    Ok(SharedPasskeyAuth::new(&rp, store)?)
}
```

The current option DTOs retain the existing Rust JSON field conventions. Map
binary challenges/user handles/credential IDs and browser dictionary names at
the HTTP boundary; the tested browser fixture shows this conversion. The fixture
is not a production identity service or a generated authentication controller.

## Acceptance

Local evidence includes 85 normal all-feature Auth tests, strict all-target
Clippy and production zero-panic checks. The explicit PostgreSQL fixture passes
independent managers/pools, actual ES256 registration/assertion, wrong bindings,
credential snapshots, parallel completion, post-consumption expiry, quotas,
corruption, lock-wait expiry, cancellation, stalled transport, a separate process
and server restart.
Chromium's virtual authenticator completes registration and authentication
through different HTTP managers and a PostgreSQL credential-counter CAS; missing
cookie/CSRF and assertion replay are rejected. These checks do not demonstrate
physical authenticator/device-fleet interoperability or failover safety.
A focused real-database mutation sample detects all 21 selected changes to
binding fingerprints, allowed credentials, issuance, consumption and expiry.
This is a bounded sample, not a whole-Auth mutation score.

Remaining acceptance and maintenance requirements:

Use real PostgreSQL with two independently constructed managers and pools, then
a new process and database restart. Cover registration/assertion with actual
ES256 signatures; replay, parallel completion, account/tenant/session/RP/purpose
confusion, empty or changed credential sets, quotas, exact expiry, lock-wait
expiry, cancellation, corrupt/oversized rows, missing metadata and configuration
mismatch. Keep the existing synchronous and SQLite device-registry regressions.
A minimal HTTP/browser consumer must demonstrate beginning on one manager and
finishing on another without relying on shared application memory. Generated
applications and an external authenticator/device fleet are separate acceptance.
Run strict production lints, feature isolation and affected mutations; full
workspace, archive and hosted release gates remain mandatory.

The protocol boundary follows the W3C's registration and assertion verification
procedures and their challenge/account binding requirements. This implementation
still supports a deliberately bounded subset. See [WebAuthn Level 3 registration](https://www.w3.org/TR/webauthn-3/#sctn-registering-a-new-credential)
and [assertion verification](https://www.w3.org/TR/webauthn-3/#sctn-verifying-assertion).
