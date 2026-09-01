# Bounded Offline Synchronization for Omni

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

Rullst's opt-in `offline-sync` feature supplies the native state and encrypted
snapshot boundary needed to build a resilient Omni client. It is not mounted
automatically by `make:omni`: the generated shell remains a minimal web shell,
and the application chooses which entities may exist offline.

The contract deliberately keeps the server authoritative. Local records are a
cache; queued writes are proposals. Roles, tenant membership, scores, streaks,
entitlements and trusted time must be recomputed or revalidated by the server.

## Enable the native profile

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", features = ["offline-sync"] }
```

The encrypted snapshot codec is native-only. A browser offline implementation
needs a separately reviewed IndexedDB/service-worker adapter and browser tests;
the current feature does not silently substitute local storage.

## Create an account-bound state

```rust
use rullst::offline_sync::{
    OfflineAccountId, OfflineSnapshotCipher, OfflineSyncPolicy, OfflineSyncState,
};

let account = OfflineAccountId::new("account_01j8student")?;
let policy = OfflineSyncPolicy::default();
let mut state = OfflineSyncState::new(account.clone());

// Load these 32 high-entropy bytes from Keychain/Keystore-class storage.
// Never embed a production key in source or store it next to the snapshot.
let device_key = load_device_key()?;
let cipher = OfflineSnapshotCipher::new("device-key-2026-01", device_key)?;
```

The cipher uses randomized AES-256-GCM. Its authenticated data binds the
envelope domain, key id and exact account id, so another account, key, modified
nonce/tag or modified ciphertext fails closed. The owned key is redacted from
`Debug` and zeroized on drop, but Rust cannot erase copies made beforehand.

## Queue a lesson attempt

Use a unique event entity for an immutable attempt. Do not queue an authoritative
score: the server grades the answer and returns its own revision and value.

```rust
use rullst::client_contract::IdempotencyKey;
use rullst::offline_sync::{OfflineEntityKey, OfflineMutation};
use serde_json::json;

let attempt = OfflineMutation::upsert(
    IdempotencyKey::new("attempt_01j8french7")?,
    OfflineEntityKey::new("lesson_attempts", "attempt_01j8french7")?,
    None,
    client_epoch_ms, // UX ordering only; never trusted by the server.
    json!({
        "lesson_id": "french-basics-7",
        "answer": "bonjour"
    }),
)?;
state.queue(policy, attempt)?;

let batch = state.push_batch(policy, 25)?;
```

Queue order is FIFO and replay keys are unique across pending and conflicted
operations. Counts, payload bytes, snapshot bytes and push size all have
configurable limits below hard framework ceilings.

Send `SyncPushBatch` inside the versioned `rullst.client` envelope. On the
server, authenticate the session, derive account/tenant context, validate the
entity and domain payload, and atomically persist both the replay key and its
result/effect. A client key alone does not provide exactly-once behavior.

## Apply authoritative results

The server returns a `SyncPushResult` whose outcomes are one of:

- `Applied`: durable result plus the current server record or tombstone;
- `Conflict`: current server record plus a stable bounded code;
- `Rejected`: stable code and an explicit retry hint.

Applying a response is transactional within the state value. Unknown or
duplicated replay keys, mismatched entities, regressing revisions, reused
revisions with different data and regressing server time leave the original
state unchanged.

A retryable rejection stays in the FIFO queue. A conflict or permanent
rejection moves the proposal out of automatic replay. The application must
either accept server state or retry the original proposal against the latest
server revision with a **new** idempotency key. There is no automatic
client-wins mode.

## Coordinate a bounded foreground sync

Implement `OfflineSyncTransport` on an application adapter that already owns
its authenticated session, TLS policy and endpoint. The trait uses static
dispatch: it does not box a provider or place credentials in offline state.
The `account_id` argument is only a local binding/routing hint; the server must
derive the real account, tenant, ownership and authorization from the
authenticated request.

Map the response's versioned client-contract envelope into
`AuthoritativePush` or `AuthoritativePull`, including server-authored time, then
run one bounded foreground attempt:

```rust
use rullst::offline_sync::{
    OfflineSyncCoordinator, OfflineSyncRunPolicy,
};

let run_policy = OfflineSyncRunPolicy::new(
    25,     // mutations per push
    4,      // push requests this run
    20,     // pull pages this run
    15_000, // timeout for every transport request
)?;

let report = OfflineSyncCoordinator::synchronize(
    &authenticated_transport,
    &mut state,
    policy,
    run_policy,
).await?;
```

The coordinator pushes before pulling, stops retrying a batch that produced no
local progress, bounds page/request counts, times out every transport future and
rejects `has_more` when the opaque cursor does not advance. It does not invent
retry delays or silently start an OS background task. Successfully accepted
pages remain in `state` if a later request fails, so seal and atomically persist
the state after both success and error paths. The server must persist replay
decisions atomically so a request accepted before a client crash can safely
return the same result later.

## Pull, reconnect and full resync

Apply incremental `SyncPullPage` values in order. If a newer server revision
touches an entity with a divergent local proposal, Rullst preserves that
proposal as an explicit conflict before caching server state.

When the server sets `requires_full_resync`, `apply_pull` changes nothing and
returns `FullResyncRequired`. Fetch a complete authorized snapshot, then call
`replace_server_snapshot`. New local entities whose base is still absent stay
pending; stale proposals become conflicts instead of disappearing.

`recover_server_cache` clears derived records, cursor and accepted server time
while preserving pending work and conflicts. It is useful after an application
decides that its server cache is unusable; it is not a substitute for detecting
corrupt encrypted bytes, which already fail authentication or schema checks.

## Persist and erase

```rust
let encrypted = cipher.seal(policy, &state)?;
platform_store_atomically(&encrypted)?;

let restored = cipher.open(policy, &account, &encrypted)?;
```

Write the encrypted bytes atomically inside the platform's application data
directory. Store the key separately in Keychain/Keystore-class storage and
apply OS backup/privacy policy intentionally. Rullst currently supplies neither
that platform adapter nor background scheduling.

For logout/account deletion, call `state.erase()`, delete every persisted
snapshot (including backups and temporary files), and remove the associated
secure-storage key. The method performs logical state erasure; it cannot erase
prior clones, filesystem snapshots, cloud backups or server data.

## What remains before calling an app offline-first

- reviewed Keychain/Keystore and atomic file/SQLite adapters per platform;
- schema migration implementations beyond the current fail-closed v1 marker;
- a concrete authenticated HTTP adapter plus application retry/backoff,
  cancellation and OS background execution around the bounded coordinator;
- browser storage support where the web product needs offline data;
- airplane-mode, process-kill, quota, corrupt-state, account-switch and
  reconnection tests on physical Android/iOS devices;
- application-specific conflict UX, retention and erasure verification.

The module closes the protocol/state/cryptographic foundation, not those
platform and product obligations.
