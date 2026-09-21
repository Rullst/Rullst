# Recoverable server-driven Live UI

The unpublished v13 candidate adds `rullst::live::recovery` in the existing Core
and facade, with no extra feature or browser dependency. It is an explicitly
mounted native-server API; the browser uses the supplied small ES module.
Protocol and Chromium acceptance have passed. Hosted workspace/platform and extracted-package source admission passed in [PR #236](v13-delivery-plan.md#six-increment-source-admission-on-september-21). Final release admission remains separate.

The older `LiveComponent`, `Live::mount` and `make:live` example keep their
per-connection state and HTMX WebSocket protocol. Adopting the recovery API is
an application change; upgrading a dependency does not migrate those components.

## Application contract

Implement `RecoverableLiveView` with three asynchronous methods:

| Method | Required application behavior |
| --- | --- |
| `authorize` | Recheck the captured authenticated session, expiry, current tenant membership and permission for this exact component/resource. Return `Unauthorized` on revocation; storage failure is an error. |
| `snapshot` | Read authoritative state and render a complete `LiveSnapshot` with its persistent monotonic `u64` revision. Escape untrusted values before including them in HTML. |
| `apply` | Validate the requested action, fields and current write permission. Atomically compare `command.expected_revision()` with the domain revision in the same transaction as the mutation, then advance it. On mismatch return `Conflict` without writing. |

`LiveScope` binds an authenticated tenant, account and component key, not a
browser-supplied role or identity. Construct it from verified server state and
`TenantMembership::select`. Scope construction alone does not authorize access.
The implementation uses static dispatch and mounts no routes automatically.

For SQL, a bound update such as `UPDATE ... SET revision = revision + 1 WHERE
tenant = ? AND id = ? AND revision = ?` belongs inside the domain transaction,
alongside any required current authorization check. Check the affected row
count before reporting success. A separate read followed by an unconditional
write cannot protect two simultaneous sockets. Never interpolate form fields
into SQL identifiers or treat a field named `tenant` as the trusted scope.

## Route and browser composition

This adapter receives a scope and view already resolved by the host's
authenticated route. It is not an authentication extractor:

```rust,no_run
use rullst::live::recovery::{
    LiveRecovery, LiveRecoveryConfig, LiveScope, RecoverableLiveView,
};
use rullst::web::axum::{
    extract::WebSocketUpgrade, http::HeaderMap, response::Response,
};

fn configured_live() -> Result<LiveRecovery, rullst::live::recovery::LiveRecoveryError> {
    Ok(LiveRecovery::new(
        LiveRecoveryConfig::try_new("https://academy.example")?,
    ))
}

async fn authorized_upgrade<C: RecoverableLiveView>(
    shared: &LiveRecovery,
    websocket: WebSocketUpgrade,
    headers: &HeaderMap,
    trusted_scope: LiveScope,
    authenticated_view: C,
) -> Response {
    shared.upgrade(websocket, headers, trusted_scope, authenticated_view).await
}
```

Create the handler once in application state; its clones share admission limits.
Use `Server` or `apply_security_baseline` for the HTTP routes and enforce ordinary
application authentication/ownership. Serve `LIVE_RECOVERY_MODULE` at an explicit
same-origin JavaScript route with a JavaScript content type. Load a separate
application module under the site's CSP; no inline-script exemption is needed.
The browser application module can contain:

```javascript
import { connectLive } from '/assets/rullst-live.js';

const live = connectLive(
  document.getElementById('lesson-view'),
  '/live/lesson',
  document.getElementById('connection-status'),
);
// A false return means the action was not sent. There is no offline queue.
// live.send('save-answer', { answer: '42' });
```

The root and optional status element must already exist. Render
`<button type="button" data-live-action="increment">` for an action without
fields, or a form with `data-live-action="save-answer"` and named string inputs.
Duplicate field names and file inputs are rejected. Submit buttons delegate to
the form; the named fields, rather than the submit button value, form the command.
Only one action can be pending. Controls with `data-live-action` are disabled
while unavailable or awaiting a response; programmatic `send` enforces the same
admission even when a form's submit button has no such attribute.

Every accepted snapshot replaces the root's HTML. This is not DOM diffing:
unsent edits, focus, selection and root-child event listeners may be replaced.
Keep unsaved drafts outside this root if the application needs to preserve them,
and use delegated listeners. Server-rendered HTML is trusted content; the
transport is not a sanitizer. Never include secrets, executable user HTML or
unescaped form values in a snapshot.

## Recovery and outcomes

On connection or reconnection, the server sends current full state. Commands use
`rullst.live.v1`, a random 32-hex correlation ID and a decimal-string revision to
avoid JavaScript integer rounding. A stale command yields a fresh snapshot with
`conflict`. A successfully committed action yields `applied` and a newer revision.

The browser emits `rullst:live-state` with `{ state, pending }` and
`rullst:live-result` with `{ id, outcome }`. Outcomes are `recovered`, `applied`,
`conflict` or `unknown`. If the connection fails while an action is pending,
`unknown` means it may already have committed. The client never automatically
replays it. Recover authoritative state before allowing a person to retry an
external effect; use separate durable domain idempotency for payments or email.
The correlation ID alone does not supply deduplication.

Reconnect attempts use capped exponential backoff with jitter (250 milliseconds
to a base cap of ten seconds, at most twelve consecutive failed attempts).
A received valid snapshot resets the retry budget. Handshake/action waits have
a thirty-second browser deadline. `reconnect()` explicitly retries with a fresh
budget; `dispose()` removes listeners and closes the socket. Recovery converges
to state, not replay of every missed transient event or automatic broadcast of
changes made by other clients. A still-connected client learns other changes on
its next action/conflict or explicit reconnect.

Origin must match the configured HTTPS origin exactly; omitted, multiple, `null`
and foreign origins fail. The only offered subprotocol must be `rullst.live.v1`.
For local fixtures, `loopback_for_tests` permits HTTP at a literal loopback IP.
The production route must not infer this configuration from an untrusted Host
or forwarded header. Use secure application cookies and trusted TLS/proxy setup.

Authorization runs before upgrade, before domain actions, after domain I/O and
before sending each snapshot, and periodically while connected. An observed
revocation closes with 4401; the client clears its root and reports `denied`.
Changing or clearing a browser cookie alone cannot revoke an existing socket:
revoke its captured server session. Already-committed actions are not rolled
back by later revocation. A race after the final authorization check cannot
recall bytes already sent; the host owns transaction-level permission checks.

HTTP upgrade failures are intentionally opaque to browser WebSocket clients;
they use bounded reconnect and eventually `unavailable`. Ordinary disconnects
can retain the previously displayed HTML. Do not interpret that cached display
as a current authorization decision. An explicit forbidden/invalid close clears
it; applications can also clear their view on logout or any disconnected state.

## Bounds and deployment

| Resource | Default and hard boundary |
| --- | --- |
| Shared connections | 128; configurable 1–1,024 per cloned handler family |
| Actions per connection | 128; configurable 1–1,024, then reconnect required |
| Callback or send | Five seconds; configurable 100 milliseconds–ten seconds |
| Session revalidation | Every five seconds; configurable 100 milliseconds–thirty seconds |
| Connection lifetime | One hour; configurable one second–one hour |
| Command/frame | 16 KiB; 32 unique fields, each name/action at most 64 ASCII bytes, each value at most 2 KiB |
| Snapshot | 64 KiB UTF-8 HTML; encoded response at most 512 KiB |
| Socket buffers | 16 KiB read/write buffer targets; 1 MiB maximum write buffer |

The server pings periodically and closes an unresponsive peer after more than
three revalidation intervals. Domain callbacks must perform asynchronous,
cancellation-safe work; a Tokio timeout cannot preempt blocking CPU work or prove
that a canceled database operation rolled back. Limits bound this handler, not
all process memory or aggregate users across replicas. Use ingress/per-account
rate limits for repeated reconnects and infrastructure-level abuse controls.

Oversized frames terminate the connection before domain actions. Rejecting the
payload at the transport limit can reset TCP before a close frame reaches the
peer; clients must not depend on receiving an application close code in that case.

Application persistence and shared revision/permission authority are required
for restart or cross-replica recovery. Neither this handler nor the browser
persists domain state. Keep revision epochs consistent when restoring backups.
Configure the proxy for WebSocket upgrades and suitable idle timeouts; TLS,
proxy behavior, replication and failover need deployment-specific validation.
This candidate does not add WebSocket ticket issuance, frame encryption, DOM
patching, an offline mutation queue or automatic blueprint migration.

## Executable acceptance

The [protocol suite](../../rullst-core/tests/live_recovery/protocol.rs) uses actual
WebSockets, a parameterized SQLite domain transaction and the production HTTP
baseline. It tests concurrent revision conflicts, uncertain committed actions,
origin/session/tenant denial, revocation before disclosure, binary/oversized
input, admission, idle peers, action limits, lifetimes and callback deadlines.

The [Chromium journey](../../.github/live-recovery-browser.mjs) exercises two
pages, offline/online transitions, exactly one send of an uncertain action,
read-only learner permissions, CSRF-protected logout and recovery after killing
and restarting the actual application process against the same database.
It is explicitly executed in the existing Linux workspace and coverage jobs.
These disposable fixtures use no real account credentials. They establish local
behavior, not release admission or a guarantee for every proxy/browser/deployment.
