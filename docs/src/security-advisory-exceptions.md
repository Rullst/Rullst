# Security advisory exceptions

Rullst does not treat an ignored scanner finding as remediated. Any temporary
exception must have a narrow scope, a compensating control, an owner, and an
expiry date. CI must fail for every advisory that is not actively governed here.

Last reviewed: **2026-10-22**.

There are **no active advisory exceptions** in the current audited dependency
graph (`deny.toml` and the audit workflows ignore no advisory).

## Remediation history

- On 2026-10-22, the time-bounded exception for `RUSTSEC-2026-0367`,
  `RUSTSEC-2026-0368` and `RUSTSEC-2026-0369` (`zip` 6.0.0 as a build
  dependency of `libduckdb-sys`, approved on 2026-10-17 until 2026-10-31) was
  removed: `duckdb`/`libduckdb-sys` 1.10506.1 build with `zip` 9, and
  `rullst-orm` now requires it. The same change moved `rustls-webpki` to
  0.103.16 and `webpki-roots`/`webpki-root-certs` to 1.0.10
  (`RUSTSEC-2026-0381` to `-0383`) and `ammonia` to 4.2.4 (`RUSTSEC-2026-0373`).
- On 2026-09-08, `RUSTSEC-2023-0071` was removed from the production and fuzz
  dependency graphs by moving `jsonwebtoken` from its `rust_crypto` backend
  to `aws_lc_rs`. The CLI scaffolds the same backend for newly generated JWT
  middleware. Auth and Connect's RS256/OIDC tests remain enabled, and Cargo
  Audit runs without an exception.
- On 2026-08-26, `RUSTSEC-2026-0173` (`proc-macro-error2`) and
  `RUSTSEC-2024-0436` (`paste` through Leptos) were removed from the resolved
  dependency graph. Their workflow ignores were removed in the same change.

These are historical remediations, not active exceptions.

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
and run the repository's `supply-chain.yml` workflow (Cargo Deny, Cargo Audit
and the exception governance check).

Patched `rustls-webpki`, `event-listener`, and `lru` findings and the removed
HTTP/2 dependency from Actix's disabled default features are deliberately not
ignored. This makes a regression in any of those versions block CI again.
