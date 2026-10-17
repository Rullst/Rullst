# Security advisory exceptions

Rullst does not treat an ignored scanner finding as remediated. Any temporary
exception must have a narrow scope, a compensating control, an owner, and an
expiry date. CI must fail for every advisory that is not actively governed here.

Last reviewed: **2026-10-17**.

## Active exceptions

| Advisory | Dependency path | Compensating control | Owner | Expiry | Exit condition |
| --- | --- | --- | --- | --- | --- |
| `RUSTSEC-2026-0367` | `zip` 6.0.0 ← build dependency of `libduckdb-sys` 1.10506.0 ← optional `duckdb`/`orm-duckdb` feature | See below | Maintainer (Venelouis) | 2026-10-31 | `duckdb-rs` release on `zip` ≥ 9, or DuckDB adapter removed |
| `RUSTSEC-2026-0368` | same | See below | Maintainer (Venelouis) | 2026-10-31 | same |
| `RUSTSEC-2026-0369` | same | See below | Maintainer (Venelouis) | 2026-10-31 | same |

The three advisories (published 2026-10-09/10, patched only in `zip` ≥ 9; every
earlier `zip` release has been yanked) concern extracting an
attacker-controlled archive: preserved SUID/SGID bits, symbolic links that point
outside the destination, and permissions applied through symbolic links.
In Rullst's dependency graph `zip` 6.0.0 is reachable only as a **build
dependency** of `libduckdb-sys`, compiled only when an application enables the
optional `duckdb`/`orm-duckdb` adapter. Its build script uses it to unpack the
DuckDB sources shipped inside the `libduckdb-sys` crate itself (checksummed by
crates.io), not an archive from an application, a user or the network, and no
Rullst runtime code links or calls `zip`. A compromised `libduckdb-sys` could run
arbitrary code from its build script regardless of `zip`. Since the yank, a new
project cannot even resolve the `duckdb` feature (13.0.0-alpha.2 Known
limitations). Removing the adapter in a prerelease would be a breaking change
with no reduction in reachable risk, so the exception is time-bounded to the
next prerelease instead. The release owner's approval is recorded on the pull
request that added it.

## Remediation history

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
and run the repository's Cargo Audit, Cargo Deny, and OSV workflows.

Patched `rustls-webpki`, `event-listener`, and `lru` findings and the removed
HTTP/2 dependency from Actix's disabled default features are deliberately not
ignored. This makes a regression in any of those versions block CI again.
