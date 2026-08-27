# Security advisory exceptions

Rullst does not treat an ignored scanner finding as remediated. Every temporary
exception below has a narrow scope, a compensating control, an owner, and an
expiry date. CI must fail for any advisory not listed here.

Last reviewed: **2026-08-26**.

| Advisory | Dependency path and scope | Compensating control | Owner | Expiry |
|---|---|---|---|---|
| `RUSTSEC-2023-0071` | `jsonwebtoken -> rsa`; production code verifies provider JWTs with public keys. RSA private-key operations occur only in test fixtures. Upstream has no fixed release. | Do not add RSA private-key signing or decryption to production paths; prefer EC/EdDSA for locally signed tokens; keep negative JWT verification tests. | Connect maintainers | 2026-11-30 |

`RUSTSEC-2026-0173` (`proc-macro-error2`) and `RUSTSEC-2024-0436`
(`paste` through Leptos) were removed from the resolved dependency graph on
2026-08-26. Their workflow ignores were removed in the same change; they are
historical remediations, not active exceptions.

## Advisory response SLA

The clock starts when a maintainer receives a credible private report or an
automated advisory first appears on a protected branch. Severity uses the
highest credible impact while triage is incomplete.

| Severity | Acknowledge and assign owner | Mitigate or release target | Maximum temporary exception |
| --- | --- | --- | --- |
| Critical | 1 business day | 72 hours | 14 days |
| High | 2 business days | 7 calendar days | 30 days |
| Medium | 5 business days | 30 calendar days | 90 days |
| Low / unmaintained without known vulnerability | 10 business days | 90 calendar days | 180 days |

If the release target cannot be met, maintainers must disable or isolate the
affected capability, yank an affected prerelease when appropriate, or create a
time-bounded exception below. Critical/high exceptions require release-owner
approval and a documented reason that disabling the capability creates greater
risk. An exception is never evidence that the vulnerability is fixed.

## Review procedure

At or before expiry, the owner must either remove the dependency, upgrade to a
fixed dependency graph, or record a new review with fresh evidence and a new
short deadline. The reviewer must inspect the dependency path with Cargo Tree
and run the repository's Cargo Audit, Cargo Deny, and OSV workflows.

Patched `rustls-webpki`, `event-listener`, and `lru` findings and the removed
HTTP/2 dependency from Actix's disabled default features are deliberately not
ignored. This makes a regression in any of those versions block CI again.
