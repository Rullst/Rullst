# Quality Scorecard

The published v12 source, `eb11f892ae28f076e7a83c38a635316c6ed89028`,
received **94/100 (A)** in
[Rust CI run 34895536764](https://github.com/Rullst/Rullst/actions/runs/34895536764).
The artifact `quality-scorecard-eb11f892ae28f076e7a83c38a635316c6ed89028`
records all sixteen crate scores and the gates used to award them.

These are repository-owned audit scores. They measure the stated engineering
criteria, not feature completeness, security certification, provider
homologation or superiority over other frameworks.

## How scores are awarded

The versioned
[audit policy](https://github.com/Rullst/Rullst/blob/main/.github/quality-scorecard-policy.json)
sets reviewed per-crate ceilings. Rust CI constrains those values using its
actual results; a green run cannot invent additional points.

| Dimension | Weight | Evidence |
| :--- | ---: | :--- |
| API and architecture | 20 | Explicitness, cohesion and public boundaries, constrained by compile/lint, feature and MSRV gates |
| Verification depth | 25 | Reviewed test depth, constrained by cross-platform workspace results |
| Security and failure design | 20 | Fail-closed, error and secret boundaries, constrained by lint and specialist checks |
| Documentation and DX | 15 | Reviewed guides, examples, features, migrations and evidence links |
| Operations and release | 20 | Reviewed durability, live-service and recovery contracts, constrained by relevant gates |

Failed, cancelled or skipped applicable gates suppress the dimensions they
prove. A targeted diagnostic run cannot substitute for the full CI matrix.
See [workflow triggers and scopes](workflows.md) for when the report is emitted.

Grades are **A+ 97–100**, **A 90–96**, **B 80–89**, **C 70–79**,
**D 60–69**, and **F below 60**.

## Stable v12 results

The v12 requirement was A or better for fifteen non-IoT packages and B or
better for IoT. The stable-source run met it. The policy was reviewed on
September 3, 2026; the later release run applied its conditioning gates rather
than claiming a new independent expert audit.

| Crate | Current audited score | Grade | Principal remaining evidence boundary |
| :--- | ---: | :---: | :--- |
| `rullst-core` | 96 | A | Dependency operations, distributed deployment and host authorization |
| `rullst-orm` | 96 | A | Online snapshot isolation, managed/PITR backup, vendor operations and application writer/tenant/key policy |
| `rullst-security` | 96 | A | Trusted rollback checkpoints, external SIEM delivery, independent audit and certification |
| `rullst-connect` | 95 | A | Remote-provider leases/reconciliation, key/directory/backup operations, multi-host refresh and provider conformance |
| `rullst` | 96 | A | Whole-file recovery/backup operations, multi-host coordination and maturity inherited from opt-in domain crates |
| `rullst-auth` | 95 | A | Shared ceremonies, multi-host state, refresh workflow and normative WebAuthn conformance |
| `rullst-mail` | 95 | A | Authoritative malware/CDR inspection, multi-host operations and inbox/provider evidence |
| `rullst-messaging` | 96 | A | Remote protocols/replication, full metadata encryption and provider operations |
| `cargo-rullst` | 95 | A | Production deployment, provider accounts and real-application acceptance |
| `rullst-ai` | 95 | A | Exact live-model results, non-compatible streaming/provider loops, durable audit receiver operations and external retrievers |
| `rullst-studio` | 94 | A | Durable/OTLP storage, key operations and shared operator authorization |
| `rullst-capital` | 93 | A | Live authorization, authoritative outbox/reconciliation and homologation |
| `rullst-orm-macros` | 95 | A | Compiler/ecosystem compatibility beyond the tested matrix |
| `rullst-nexus` | 95 | A | Host identity/domain policy, global/custom-route authorization, immutable audit delivery and production operations |
| `rullst-macros` | 94 | A | Real browser/network ecosystems and host identity policy remain external |
| `rullst-iot` | 83 | B | Concrete transport/hardware storage, flashing and bootloader evidence |
| **Repository (equal-crate aggregate)** | **94** | **A** | **1,509/1,600; exact score remains conditional on the SHA's gates** |

## What “ceiling reached” meant

The v12 campaign reached its reviewed target of **1,509/1,600 points**.
A zero gap means that campaign's target was met. It does not mean further
improvement is impossible, that every roadmap item shipped, or that a model
has a permanent capability ceiling.

The earlier planning tables and interim scores are preserved in the
[immutable pre-publication record](https://github.com/Rullst/Rullst/blob/v12.0.0/docs/src/quality-scorecard.md).
Future work can earn different scores only after the policy, implementation
and evidence are reviewed together.

## Unpublished v13 privacy candidate

The 20 September policy update retains the sixteen existing ceilings and adds
`rullst-privacy` to the seventeen-package candidate inventory. Its initial ceiling
is **85/100**, below the unchanged 90-point RC planning floor and 95-point local
target. The report remains observational; these planning values grant no release
approval and are not a published v13 result.

The reviewed dimensions are API 19/20 (explicit optional contracts), verification
23/25 (restart, concurrency, real PostgreSQL and generated HTTP consumers),
security 19/20 (bounded, bound and fail-closed decisions), documentation 14/15
(scoped methods, setup and recovery obligations), and operations 10/20. Pending
installed-distribution acceptance, first registry publication/ownership and
operator recovery evidence withhold operations credit. Live providers and
verified guardianship remain outside the implemented scope. The workspace
and generated CLI test matrix conditions security/operations credit; the local Verus pilot
is separate evidence and does not increase this report's score.

## Maintaining the score

The September 22 package-preparation candidate adds Supervision and Media to
the nineteen-package scorecard inventory while retaining every earlier ceiling.
Each new package starts with the same conservative **85/100 ceiling** as Privacy:
API 19, verification 23, security 19, documentation 14 and operations 10.
The 90-point RC planning floor and 95-point local target remain unchanged.
These are reviewed policy limits, not awarded scores or independent audits.

Supervision's evidence covers its optional domain contracts, shared-local
SQLite state, minimized browser observations and generated LMS consumer. It
does not verify guardianship, infer cheating as fact or control other device
applications. Media's evidence covers the Bunny lifecycle, protocol failures,
browser upload, durable recovery and an extracted consumer; actual Bunny/CDN
interoperability and application entitlement policy remain outside that evidence.
Both withhold operations credit for ordinary-release archive acceptance, first
registry registration and deployment recovery. Their current hosted gates must
pass before the report can award the applicable dimensions. Labs remains outside
the publication scorecard and retains its separate isolation acceptance.

For every relevant change, retain the commit and workflow result, identify
which gates and evidence changed, and report capability progress separately.
New source does not inherit the stable release's score automatically.

Provider acceptance, physical devices, store publication, fiscal authorization,
independent review and production operations remain separately scoped evidence.
Use the [capability status](capability-status.md),
[capability ledger](capability-ledger.md), and
[release audit](v12-release-audit.md) for those boundaries.
