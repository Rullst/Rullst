# Security advisory exceptions

Rullst does not treat an ignored scanner finding as remediated. Any temporary
exception must have a narrow scope, a compensating control, an owner, and an
expiry date. CI must fail for every advisory that is not actively governed here.

Last reviewed: **2026-11-09**.

## Active exceptions

| Advisory | Dependency path | Compensating control | Owner | Expiry | Exit condition |
| --- | --- | --- | --- | --- | --- |
| `RUSTSEC-2026-0459` | `libsqlite3-sys` 0.37.0 (bundled SQLite 3.53) ← `sqlx-sqlite` 0.9.0 ← every Rullst SQLite feature | See below | Maintainer (Venelouis) | 2027-01-28 | An `sqlx` release that accepts `libsqlite3-sys` ≥ 0.39, then required by `rullst-orm` |
| `RUSTSEC-2026-0440` | `astral-tokio-tar` 0.6.4 ← `testcontainers` 0.27.3 ← dev-dependency of the integration tests | See below | Maintainer (Venelouis) | 2027-01-28 | A `testcontainers` release without `astral-tokio-tar`, or the tests move to another container client |

**`RUSTSEC-2026-0459`** (medium; CVE-2026-119375, published 2026-10-30): SQLite
before 3.54.0 can return uninitialized page-cache memory when it opens a
database together with a crafted sibling `-wal` file of a different page size.
The fix is `libsqlite3-sys` 0.39, but `sqlx-sqlite` 0.9.0, the newest release
and the one Rullst 13 uses, requires `>=0.30.1, <0.38.0`, so no Rullst 13
application can resolve the patched version yet. The attack is local and needs
a less trusted party to write both the database file and its `-wal`. Rullst
opens SQLite files only at paths the application configures (`DATABASE_URL`
and the opt-in local state stores of the queue, messaging, labs, media,
supervision and privacy crates), creates them itself and never opens a
database supplied by a request. An application that accepts uploaded or
replaced SQLite files must not open them with the SQLite driver until the exit
condition is met. The CI workflow `cargo rullst new` writes (since
13.0.0-alpha.5) ignores this advisory only until the same expiry, with a comment
that names the exposure and says to remove the exception when the application
upgrades to a Rullst release that updates `sqlx`. The 13.0 release notes list the advisory under Known
limitations.

**`RUSTSEC-2026-0440`** (unmaintained, no known vulnerability; published
2026-11-03): Astral ended maintenance of `astral-tokio-tar`. It is reached
only through `testcontainers`, a dev-dependency of the database integration
tests of `rullst-orm`, `rullst-core`, `rullst-capital`, `rullst-studio`,
`rullst-nexus` and `rullst-ai`. No published Rullst crate depends on it, and the
tests use it only to copy their own files into local containers. `testcontainers`
0.28.0, the newest release, still depends on it.

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
