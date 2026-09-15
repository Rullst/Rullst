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

## Maintaining the score

For every relevant change, retain the commit and workflow result, identify
which gates and evidence changed, and report capability progress separately.
New source does not inherit the stable release's score automatically.

Provider acceptance, physical devices, store publication, fiscal authorization,
independent review and production operations remain separately scoped evidence.
Use the [capability status](capability-status.md),
[capability ledger](capability-ledger.md), and
[release audit](v12-release-audit.md) for those boundaries.
