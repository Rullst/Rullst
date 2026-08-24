# Threat Radar and security telemetry

> **Vision preserved:** external intelligence, verified audit feeds, durable
> telemetry, and SOC ambitions remain itemized with an implementation opinion in
> the [capability ledger](capability-ledger.md#security-authentication-studio-and-nexus).

Rullst exposes development dashboards for security events in Studio and Nexus.
They are observability surfaces, not a managed SOC, a penetration test, or proof
of compliance.

## Surfaces and access

- Studio mounts its security view under `/studio/security`.
- Nexus exposes its administrative security view only inside the authenticated,
  authorized Nexus router.

Both surfaces contain sensitive operational information. Bind Studio to a trusted
interface and protect any deployed administrative route with TLS, authentication,
role checks, and rate limiting.

## What the counters mean

The dashboards read counters and bounded recent events from the in-process
`SecurityStore`. A value increments only when the corresponding middleware or
helper is installed and records an event. A zero means “nothing recorded by this
process”, not “the application is proven attack-free”. Restarting a process may
reset in-memory state unless the application exports it to durable telemetry.

Typical sources include:

- honeypot route hits and active TTL-limited bans;
- sanitizer, DLP, CSRF/CSWSH, RBAC, rate-limit, and login-jail events;
- prompt-injection filtering and PII masking events;
- secure-header applications and timing-guard executions.

Client identity comes from the trusted peer by default. Forwarded headers are
usable only when the application has explicitly configured and authenticated a
trusted proxy boundary.

## Audit-chain status

`AuditChain` signs canonical, length-delimited records with HMAC and can verify
record integrity and sequence continuity. It detects modifications only when the
key is protected and the complete sequence is retained.

The dashboard must display `Unavailable` until an audit source and continuity
verifier are actually connected. It must never infer “verified” merely because an
event was emitted. An HMAC chain is tamper-evident; it cannot prevent deletion,
key compromise, or loss of the entire log.

## Security controls and limits

| Control | What it provides | Important limit |
| --- | --- | --- |
| RASP/WAF inspection | Bounded heuristics for common malicious request patterns. | It is not a complete parser or substitute for parameterization and domain validation. |
| Secure headers | A strict nonce-based CSP/header baseline. | The deployed page, proxy, browser, and policy determine scanner results; no A+ grade is guaranteed. |
| Honeypots | Exact synthetic trap paths and temporary bans. | They do not identify every scanner or replace edge rate limiting. |
| Login guard | Shared failure tracking, delay, cleanup, and jail policy. | Account and recovery policy remain application responsibilities. |
| DLP and PII filters | Supported textual-response masking with content-type and size safeguards. | Binary, encoded, streaming, and unsupported responses must follow explicit fail-open/fail-closed policy. |
| AI guardrails | Prompt checks and masking in the high-level AI client. | Heuristics cannot guarantee that every adversarial prompt or sensitive value is detected. |

## Local verification

Use a test-only application instance and assert both the HTTP response and the
recorded event. Do not probe a third-party or production target without written
authorization.

Recommended negative tests include:

- exact honeypot paths versus innocent paths containing similar substrings;
- spoofed forwarding headers from untrusted peers;
- ban expiry and bounded-cardinality behavior;
- valid, invalid, reordered, and truncated audit sequences;
- JSON, HTML, binary, compressed, SSE, streaming, and oversized DLP responses;
- CSP nonces that match the rendered response and differ between requests;
- authorization checks for every Nexus CRUD and batch operation.

## Production checklist

- Mount every security layer required by the application; availability in a
  crate does not install it automatically.
- Configure trusted proxies explicitly and retain the direct peer address.
- Export logs and metrics to durable, access-controlled storage.
- Protect audit keys separately from audit records and rotate them with a
  documented verification procedure.
- Test application-specific CSP, CSRF exceptions, tenant boundaries, and
  authorization rules.
- Keep Studio private and verify Nexus authentication, TLS, rate limiting, and
  admin-role enforcement end to end.
