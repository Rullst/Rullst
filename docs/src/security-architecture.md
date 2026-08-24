# Security architecture and boundaries

> **Vision preserved:** unfinished enterprise controls and absolute former claims
> remain visible, with a recommendation for each, in the
> [capability ledger](capability-ledger.md#security-authentication-studio-and-nexus).

`rullst-security` contains composable controls for HTTP applications. The crate
does not install every control automatically and does not replace secure domain
logic, a trusted reverse proxy, operating-system hardening, or independent
security testing.

## Defense in depth

```mermaid
flowchart LR
    Peer[Trusted peer identity] --> Edge[Rate limit and honeypots]
    Edge --> Request[CSRF, CSWSH and bounded RASP/WAF]
    Request --> Identity[Authentication, MFA and RBAC]
    Identity --> App[Application and parameterized data access]
    App --> Response[Secure headers and supported DLP filtering]
    App --> Audit[Tamper-evident audit records]
```

Each arrow is an application integration point. If a layer is not mounted, its
counter and API being present in the crate do not protect traffic.

## Control map

| Risk area | Available primitives | Boundary |
| --- | --- | --- |
| Broken access control | `RbacGuard`, ownership helpers, authenticated user context. | The application must apply a check to every protected object and operation. |
| Cryptographic storage | AES-256-GCM field encryption and zeroizing secret wrappers. | Operators own key generation, storage, rotation, separation, and recovery. |
| Injection | SQLx binds, strict identifier validation, sanitizer and bounded RASP patterns. | Heuristics are not a complete language parser; domain validation and parameterization remain mandatory. |
| Misconfiguration | Nonce-based CSP and a strict HTTP-header baseline. | Proxies and page content change the deployed policy; no scanner grade is guaranteed. |
| Authentication abuse | Login jail, rate limiter, timing helpers, TOTP and WebAuthn integration. | Account recovery, RP/origin configuration, trusted peer identity, and capacity planning remain application concerns. |
| Data integrity | HMAC audit records with canonical encoding and sequence verification. | Durability, deletion resistance, key protection, and independent verification require external storage and operations. |
| Data leakage | Text-aware DLP, PII masking, and log redaction helpers. | Unsupported content types, encodings, streams, and oversize bodies follow explicit policy and must be tested. |
| AI input risk | Prompt-injection heuristics and PII masking in the high-level AI client. | No heuristic can prove a prompt safe or guarantee detection of every secret. |

This is a control mapping, not a claim of complete OWASP Top 10 coverage.

## Identity and network trust

Rate limiting, bans, honeypots, and audit attribution use the direct peer address
unless a trusted-proxy policy explicitly accepts forwarded metadata. Never trust
`X-Forwarded-For`, tenant headers, or role headers directly from an arbitrary
client. Tenant membership and roles must come from an authenticated session or a
cryptographically trusted internal gateway.

## Request and response inspection

RASP/WAF rules are bounded to avoid uncontrolled CPU or memory work. They can
reject known suspicious patterns in supported URI, header, and bounded body data,
but application queries must still use binds and access control.

DLP modifies only supported textual responses whose body can be safely buffered
within configured limits. Applications must test JSON, HTML, binary, compressed,
SSE, streaming, and oversized responses so headers and bodies remain
protocol-correct.

## Audit chains

Audit records use an unambiguous canonical representation and a non-empty HMAC
key. Verification must cover the ordered sequence, not just isolated records. A
valid chain is tamper-evident; it cannot stop an attacker who can delete every
record or steal the key. Store the log and key in separate protected systems.

## Supply-chain and compliance evidence

Repository workflows can run dependency policy, RustSec, CodeQL, fuzzing,
sanitizers, SemVer checks, SBOM generation, and release attestations. Their scope
and blocking status are defined by the workflow files. Informational checks must
not be described as formal gates.

Neither those workflows nor this crate certify an application for SOC 2, ISO
27001, PCI DSS, FedRAMP, or any OWASP level. See `AUDIT.md` and
`SECURITY_COMPLIANCE.md` for reproduction and evidence requirements.

## Deployment checklist

- Mount middleware in a tested order and use exact webhook CSRF exemptions.
- Configure secure session, webhook, audit, and encryption keys; reject weak or
  empty live credentials.
- Define trusted proxies and use the direct peer as the default identity.
- Apply owner/tenant/role checks server-side for every data operation.
- Validate per-response CSP nonces against rendered pages.
- Export telemetry and audit records to durable, access-controlled storage.
- Run negative integration tests and an independent security review for the
  application's actual configuration.
