# Security advisory exceptions

Rullst does not treat an ignored scanner finding as remediated. Every temporary
exception below has a narrow scope, a compensating control, an owner, and an
expiry date. CI must fail for any advisory not listed here.

Last reviewed: **2026-08-24**.

| Advisory | Dependency path and scope | Compensating control | Owner | Expiry |
|---|---|---|---|---|
| `RUSTSEC-2023-0071` | `jsonwebtoken -> rsa`; production code verifies provider JWTs with public keys. RSA private-key operations occur only in test fixtures. Upstream has no fixed release. | Do not add RSA private-key signing or decryption to production paths; prefer EC/EdDSA for locally signed tokens; keep negative JWT verification tests. | Connect maintainers | 2026-11-30 |
| `RUSTSEC-2026-0173` | `proc-macro-error2` enters through the optional Leptos adapter and executes at build time. The advisory is for an unmaintained crate. | Keep Leptos optional; do not execute generated artifacts from untrusted sources; track removal or an upstream replacement. | Connect maintainers | 2026-11-30 |
| `RUSTSEC-2024-0436` | `paste` enters through the optional Leptos adapter and executes at build time. The advisory is for an unmaintained crate. | Keep Leptos optional; do not execute generated artifacts from untrusted sources; track removal or an upstream replacement. | Connect maintainers | 2026-11-30 |

## Review procedure

At or before expiry, the owner must either remove the dependency, upgrade to a
fixed dependency graph, or record a new review with fresh evidence and a new
short deadline. The reviewer must inspect the dependency path with Cargo Tree
and run the repository's Cargo Audit, Cargo Deny, and OSV workflows.

Patched `rustls-webpki`, `event-listener`, and `lru` findings and the removed
HTTP/2 dependency from Actix's disabled default features are deliberately not
ignored. This makes a regression in any of those versions block CI again.
