# Tutorial 32: Bounded security telemetry in Studio

Rullst's honeypot, RASP and Studio security page provide local defense-in-depth
signals. They are not an autonomous SOC, a universal blocker, an AI incident
responder or a durable SIEM integration.

## 1. Compose local request controls

```rust,no_run
use axum::Router;
use rullst_security::{CspSecurityLayer, HoneypotLayer, HoneypotState, RaspSecurityLayer};
use std::net::SocketAddr;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let app = Router::new()
    // Add application routes first.
    .layer(RaspSecurityLayer)
    .layer(CspSecurityLayer)
    .layer(HoneypotLayer::new(HoneypotState::default()));

let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
axum::serve(
    listener,
    app.into_make_service_with_connect_info::<SocketAddr>(),
)
.await?;
# Ok(())
# }
```

`ConnectInfo` supplies the accepted socket peer used by the honeypot. Deployments
behind a proxy must establish and test a trusted client-identity boundary; the
middleware deliberately does not trust arbitrary forwarding headers.

## 2. Understand each signal

- Honeypots match configured exact synthetic paths and keep bounded, expiring
  process-local bans.
- RASP performs bounded heuristic inspection for selected patterns and can have
  false positives and negatives.
- CSP and headers depend on the final rendered page, proxy, browser and TLS
  deployment.
- `SecurityStore` is local telemetry. An event is not HMAC-verified or durably
  delivered merely because it appears in Studio.

No built-in path asks an LLM to ban a peer or automatically mounts a blocking
policy. The opt-in `ThreatSentinel` can classify three bounded,
caller-supplied aggregate patterns and issue an HMAC-authenticated,
subject-bound, expiring, one-shot **process-local** proof-of-work challenge.
The application must supply trustworthy observations and subject identity,
translate the outcome into an HTTP protocol, and decide where that gate belongs.
It is not AI attribution, a distributed replay store, or an autonomous ban.
Sensitive automated actions still require authenticated policy, limits,
durable audit and human approval where appropriate.

## 3. Inspect the local Studio view

Mount Studio only through its documented access capability and open
`/studio/security`. The page renders current local telemetry and keeps
unavailable sources visibly unavailable. A multi-instance deployment needs a
shared, authenticated event pipeline with retry, acknowledgement, retention and
dead-letter handling before it can be described as an operational SIEM.

See the [Threat Radar and SOC guide](../threat-radar-soc-guide.md) and the
[v12 security evidence ledger](../v12-security-claims.md) for exact boundaries.
