# `rullst-security`

> **Vision preserved:** controls and security ambitions that are partial or not
> implemented were not discarded. See their itemized status and recommendation
> in the [capability ledger](../capability-ledger.md#security-authentication-studio-and-nexus).

`rullst-security` provides composable security controls and telemetry primitives.
Availability in the crate does not install a control automatically, and no single
middleware can guarantee that an application is secure.

## Main modules

- `headers`: strict security-header baseline with per-response CSP nonces.
- `rasp` and WAF helpers: bounded inspection for common suspicious request
  patterns.
- `honey` and `deception`: exact trap paths, trusted-peer attribution, bounded
  telemetry, and TTL-limited bans.
- `login_guard`, `rate_limit`, and `timing_guard`: authentication-abuse controls.
- `rbac` and ownership helpers: explicit server-side authorization primitives.
- `mfa`: six-digit TOTP verification and percent-encoded `otpauth` URIs.
- `cswsh`: normalized Origin/Host validation for WebSocket handshakes.
- `dlp`, `sanitizer`, and `log_redactor`: content-aware masking and sanitization
  for supported representations.
- `vault`: authenticated AES-256-GCM field encryption and zeroizing secret
  wrappers.
- `audit`: canonical HMAC records and ordered continuity verification.
- `siem` and telemetry: bounded operational event export.
- `ai_firewall`: heuristic prompt inspection; the high-level AI client integrates
  prompt and PII guardrails.

## Important boundaries

- RASP and prompt inspection are bounded heuristics, not complete parsers or proof
  that input is safe.
- Client IP defaults to the direct peer. Forwarded headers require an explicitly
  trusted proxy policy.
- Tenant and role identity must come from verified authentication claims.
- DLP supports defined textual responses and size limits; applications must test
  binary, encoded, compressed, streaming, SSE, and oversized responses.
- Security headers provide a baseline. Deployed content and proxies determine the
  effective CSP and any third-party scanner result.
- An HMAC audit chain is tamper-evident only while the key and ordered records are
  protected. It cannot prevent wholesale deletion or key theft.
- Offline/local AI privacy depends on selecting a local endpoint and preventing
  fallback to a cloud provider; it is not implied by the crate alone.

## Minimal integration checklist

1. Establish authentication and trusted peer/tenant context before authorization
   or rate-limit layers consume it.
2. Apply RBAC/owner checks to every protected CRUD and batch operation.
3. Mount CSRF on browser routes and exempt only exact, cryptographically verified
   webhook routes.
4. Configure strong, non-empty session, webhook, audit, and encryption keys.
5. Render the CSP nonce supplied for the current response and test the final page.
6. Export telemetry and audit records to durable, access-controlled storage.
7. Run application-specific negative integration and penetration tests.

For the full control map and deployment boundaries, see
[Security architecture and boundaries](../security-architecture.md). For audit
and compliance language, see the repository `AUDIT.md` and
`SECURITY_COMPLIANCE.md`.
