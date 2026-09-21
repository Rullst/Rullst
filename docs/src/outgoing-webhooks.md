# Durable outgoing webhooks

The unpublished v13 candidate adds Messaging `webhooks` and facade
`messaging-webhooks`. It provides a private encrypted SQLite outbox for one
immutable destination per server-owned namespace, signed HTTP deliveries,
bounded retry, cancellation and minimized failure inspection. It reuses the
existing broker's idempotent publication and fenced delivery state.

This profile supports independent processes sharing a **local persistent file**.
It does not coordinate databases on different hosts, support network filesystems
or turn ephemeral container disks into durable storage. The application owns
file/directory permissions, persistent volumes, backup custody and restart policy.
The feature does not enable itself by default.

## Publish and supervise delivery

Authorize the tenant/account and operation before selecting a namespace, creating
a destination, publishing, cancelling or inspecting events. Never accept an
arbitrary request URL as an approved destination. An authenticated management
endpoint must retain ownership checks and the normal CSRF/WAF/secure-header
baseline. The library does not add HTTP management routes or grant authority.

```rust,ignore
use rullst::messaging::{MessagingKeyring, MessagingStorageKey, SystemClock};
use rullst::messaging::webhooks::{
    WebhookConfig, WebhookDestination, WebhookOutbox, WebhookSigningKey,
};

let config = WebhookConfig::new(
    "school-webhooks",
    WebhookDestination::approved_https("https://receiver.example/events")?,
    10_000,
)?.require_production()?;
let signing = WebhookSigningKey::new("receiver-key-2026-09", encoded_signing_key)?;
let storage = MessagingKeyring::new(
    MessagingStorageKey::try_new("storage-key-2026-09", storage_key_bytes)?,
);
let outbox = WebhookOutbox::open(database_url, config, signing, storage, SystemClock).await?;
outbox.enqueue("completion/42/v1", "course.completed", json_bytes).await?;
let outcome = outbox.dispatch_next("worker-1").await?;
```

`encoded_signing_key` is canonical unpadded base64url for exactly 32 CSPRNG bytes.
Use a separate secret for encrypted storage. Empty or `mock_*` signing credentials
select deterministic offline delivery with an explicit `offline: true` result;
production configuration rejects them. Offline signing keys never authenticate
incoming HTTP signatures. Test destinations accept only literal loopback HTTP(S)
and optionally an owned test CA; production configuration rejects that mode.

`open` creates or reopens the encrypted outbox, reserves one internal control
record and subscribes its private delivery group. The encrypted control record
binds destination, key identity/fingerprint, mode, retention quota and delivery
window. Configuration/key drift fails instead of sending old payloads to a new
receiver. Changing destination or signing key requires an explicitly new namespace
and an application-owned drain/cutover policy; receiver key overlap is managed by
the host. Storage keyrings can retain old decryption keys while new writes use a
new primary. Do not remove keys while retained records depend on them.

Enqueue preserves exact JSON bytes up to 64 KiB and a bounded event kind.
Reusing a retained application event key with different bytes or kind fails;
identical retries return the original message ID. Persisted application event
keys are HMAC-derived; the raw event key is not retained. The application database
transaction and this outbox are separate commits. Bridge from an application
transactional outbox when atomic domain-change publication is required.

Run `dispatch_next` from a supervised loop with bounded concurrency, backoff and
shutdown. It claims one event at a time with a 30-second lease, resolves the
endpoint, rechecks the authoritative SQL lease, sends and then acknowledges.
Storage calls have a 10-second deadline. DNS/connect operations are bounded by
3 seconds, and HTTP by the remaining lease/window and at most 10 seconds.
`close` closes that outbox pool and its clones, not independently opened workers.

## Destination and receiver security

Production uses HTTPS with certificate validation, disables proxies and redirects,
and resolves the approved hostname freshly per attempt. All returned addresses
must pass the public-address policy, with a 16-address ceiling. The approved
addresses are pinned to that attempt's connection; a subsequent DNS answer cannot
choose another destination. Private, loopback, link-local, multicast, reserved,
documentation and translation/transition ranges are denied. IPv6 admission is
conservative ordinary global unicast. Deployments still own resolver trust,
network routing and egress policy; this does not replace a firewall.

These controls were checked against the
[IANA IPv4](https://www.iana.org/assignments/iana-ipv4-special-registry/) and
[IPv6](https://www.iana.org/assignments/iana-ipv6-special-registry/) registries and
[OWASP SSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html).
Review address policy when registries change. Deliberate conservative denial can
exclude specialized globally reachable protocol addresses.

Every POST sends `Content-Type: application/json` and five signature headers:

| Header | Value |
| --- | --- |
| `Rullst-Webhook-Id` | Stable broker message ID, preserved on retry |
| `Rullst-Webhook-Type` | Frozen event kind |
| `Rullst-Webhook-Timestamp` | Canonical integer Unix seconds for this attempt |
| `Rullst-Webhook-Key-Id` | Server-selected signing-key identifier |
| `Rullst-Webhook-Signature` | `v1=` plus canonical unpadded base64url HMAC-SHA256 |

The MAC input concatenates six byte strings, **each preceded by its 8-byte
unsigned big-endian byte length**, in this order: `rullst.webhook.signature.v1`,
key ID, delivery ID, event kind, decimal timestamp and the exact body bytes.
Do not parse/re-serialize JSON before verification. The signature header is
marked sensitive in the HTTP client's header map. Error/Debug surfaces omit keys,
payloads, destination URLs and response bodies.

Receivers should call `WebhookSignature::from_http_headers` to reject missing,
malformed or duplicate signature headers, then `WebhookSigningKey::verify` on a
bounded raw body and trusted clock. The maximum accepted skew is explicitly
1–300 whole seconds. Axum/http 1 header maps are compatible. The lower-level
`from_headers` constructor assumes the caller already rejected duplicates.

Verification uses constant-time HMAC validation and binds every signed field.
It **does not store replay state**. Authenticate before creating the receiver's
transactional deduplication record, deduplicate by delivery ID, authorize the
event's application context and record the side effect atomically when possible.
Signature verification alone neither grants tenant authority nor prevents replay.

## Retries, cancellation and retention

Any 2xx status means receiver acceptance, not completion of its external effects.
408, 425, 429 and 5xx responses, DNS failures and transport errors use bounded
exponential backoff. A bounded integer `Retry-After` up to one hour can lengthen
the delay. Redirects, other permanent responses and forbidden destinations become
terminal. Arbitrary receiver bodies are never buffered or persisted.

Automatic delivery stops after ten attempts or its immutable delivery window.
The window defaults to one day and is configurable from one minute through seven
days. A timeout can happen after the receiver accepted a request; retries preserve
the exact body and ID. If a successful HTTP response precedes a failed/uncertain
local ACK, `WebhookError::Acknowledgement` retains the stable ID and status.
Receiver-side effect deduplication remains necessary.

`cancel` permanently invalidates pending/leased/failed work. It cannot recall a
request already in flight or undo a receiver effect. Stale workers cannot
acknowledge or retry the cancelled claim. `retry_failed` starts a new explicit
operator attempt budget with the same ID and original deadline; it cannot revive
cancelled, accepted, purged or expired work. Host applications authorize these
management actions and own disciplined trusted time. The shared broker does not
provide persistent clock anti-rollback or certify restore/failover behavior.

`failed` returns up to 100 minimized terminal records: ID, kind, attempts, bounded
failure code and time. No payload or destination is returned. At most 99,999
application events can be retained, in addition to the control record. Accepted
and failed events retain encrypted content until explicit maintenance.
`purge_terminal` deletes at most 100 terminal events older than the creation
cutoff; the cutoff must precede now by the full delivery window. Active deliveries
and control state are preserved. Purging also removes sender idempotency evidence;
choose receiver retention and business idempotency accordingly.

## Local acceptance status

Owned HTTP/TLS tests pass exact-body signatures, receiver deduplication after a
lost response, certificate trust, redirect/private-address denial, concurrent
workers, cancellation during delivery, bounded retries, manual retry, expiry,
quota/configuration/namespace isolation, actual SQL outage/cancelled futures and
terminal retention. A fresh child process delivers a persisted event, and a
second process observes its durable acknowledgement. Broker lock-contention tests
also reproduce and correct the earlier pre-lock clock sampling defect.

No provider account or external recipient is used. Seventeen extracted archives
were audited; the facade consumer passed the same HTTP/TLS and fresh-process
journeys with 53 Messaging source files byte-matched to the candidate. All-feature
Messaging regression (63 ordinary tests), strict all-target Clippy and the native
recurring PostgreSQL regression passed. Full hosted workspace/coverage/security
checks and source admission remain required before declaring this candidate complete.
