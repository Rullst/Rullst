# Quality scorecard

Rullst generates an evidence-bound quality scorecard for every push to `main`
and every pull request. The report is attached to the corresponding **Rust CI**
run as `quality-scorecard-<commit SHA>` and is also written to that run's job
summary.

The permanent source of a run score is therefore the commit plus its workflow
run, not a mutable badge. The versioned expert-audit ceilings live in
`.github/quality-scorecard-policy.json`; CI can reduce them when required
evidence fails, but a green run cannot inflate them.

## What the score measures

| Dimension | Weight | Evidence |
| :--- | ---: | :--- |
| API and architecture | 20 | Audited explicitness, cohesion and public-boundary quality; awarded only while format/Clippy, feature and MSRV gates pass |
| Verification depth | 25 | Audited test depth constrained by the cross-platform all-feature workspace result |
| Security and failure design | 20 | Audited fail-closed/error/secret boundary constrained by Clippy and the applicable specialist gate |
| Documentation and DX | 15 | Audited user guidance, examples, feature/migration clarity and evidence links |
| Operations and release | 20 | Audited durability/live/recovery/release maturity constrained by feature, MSRV and specialist evidence |

Specialist evidence includes database/Redis live matrices, AI evals, the threat
minimum, release local-access negatives, provider matrices, and Messaging's
wire/trace, encrypted SQLite and ORM outbox crash-replay cases. Failed,
cancelled, or skipped applicable gates suppress
the dimensions they prove; the report is still generated so a red push cannot
hide its note.

Documentation/DX evidence also includes a Cargo-aware aggregation of the 52
public tutorials. It consumes the Markdown files directly during the normal
all-feature doctest run, so a green workspace test proves the standalone Rust
examples compile on that SHA; explicitly contextual fragments remain visible as
ignored and do not count as compiled examples.

Grades use the following fixed bands: A+ 97–100, A 90–96, B 80–89, C 70–79,
D 60–69, and F below 60.

The v12 RC quality objective is **A (90) or better for every crate except
`rullst-iot`, whose approved floor is B (80)**. A+ is an evidence threshold,
not a value to assign by intent. This owner-approved gate is deliberately
stricter than the earlier all-B floor and reopens bounded implementation work
before the feature freeze. It does not pre-approve a commit: the exact SHA
still earns each ceiling only when every constraining gate succeeds.

## Current audited green-gate scores — 3 September 2026

These are the maximum current scores when every referenced Rust CI gate passes.
They are not presumed results for a new commit; the exact per-SHA artifact
applies the real gate outcomes and includes the full finding for every row.
They are also not the highest scores that future repository-owned work can
earn. Here, **current audited score** means the score supported by code and evidence
already present; **local campaign ceiling** means a planning target that must
still be earned. Keeping those columns separate prevents a desired score from
being published as an achieved one.

| Crate | Current audited score | Grade | Principal remaining evidence boundary |
| :--- | ---: | :---: | :--- |
| `rullst-core` | 91 | A | Distributed deployment and host authorization |
| `rullst-orm` | 91 | A | Cross-store semantics, disaster recovery and remaining operational adapters |
| `rullst-security` | 90 | A | External SIEM delivery, independent audit and certification |
| `rullst-connect` | 90 | A | Durable transaction/key operations, distributed refresh and provider conformance |
| `rullst` | 90 | A | Inherits bounded maturity from opt-in domain crates |
| `rullst-auth` | 95 | A | Shared ceremonies, multi-host state, refresh workflow and normative WebAuthn conformance |
| `rullst-mail` | 95 | A | Authoritative malware/CDR inspection, multi-host operations and inbox/provider evidence |
| `rullst-messaging` | 96 | A | Remote protocols/replication, full metadata encryption and provider operations |
| `cargo-rullst` | 95 | A | Production deployment, provider accounts and real-application acceptance |
| `rullst-ai` | 90 | A | Streaming/cancellation, live adaptive evals, distributed audit delivery, provider loops and external retrievers |
| `rullst-studio` | 83 | B | Distributed observability and remote inspectors |
| `rullst-capital` | 93 | A | Live authorization, authoritative outbox/reconciliation and homologation |
| `rullst-orm-macros` | 95 | A | Compiler/ecosystem compatibility beyond the tested matrix |
| `rullst-nexus` | 95 | A | Host identity/domain policy, global/custom-route authorization, immutable audit delivery and production operations |
| `rullst-macros` | 94 | A | Real browser/network ecosystems and host identity policy remain external |
| `rullst-iot` | 83 | B | Concrete transport/hardware storage, flashing and bootloader evidence |
| **Repository (equal-crate aggregate)** | **92** | **A** | **1,466/1,600; exact score remains conditional on the SHA's gates** |

## Measured gap to the v12 quality gate

At the audited scores above, Core, ORM, Security, Connect, Auth, Mail,
Messaging, AI, Capital, Nexus, both macro crates, `cargo-rullst` and the umbrella
meet A, while IoT meets its approved B exception. One crate remains below its
required grade. Its gap totals **7 audited points**. This is not a count of tasks and
cannot be closed by editing the policy: every point requires a commensurate
change in implementation, verification, security/failure design,
documentation/DX, or operational evidence.

If Studio reaches exactly 90 while the other current scores remain unchanged,
the equal-crate aggregate becomes 1,473/1,600, or **92/A**.

| Crate | Current | Gap to required grade | Next evidence cluster to audit |
| :--- | ---: | ---: | :--- |
| `rullst-studio` | 83/B | +7 | Distributed trace ingestion and authenticated remote-inspector boundaries |

## Maximum-local v12 campaign

The release floor is not the stopping target. The table below records the
provisional highest score that the current campaign can responsibly pursue
with repository-owned implementation, deterministic fixtures, local services,
CI and documentation. These targets do **not** alter the scorecard policy and
must not appear as achieved scores until their evidence is implemented and
green on the exact commit.

The campaign is scoped to the 15 non-IoT crates, v12 quality, and the
historically promised `[x]` capabilities. IoT remains audited at its accepted
83/B evidence but is outside the remaining ceiling work. The campaign does not
pull every open v13 idea into the RC. External
provider acceptance, app-store/device testing, fiscal homologation, independent
audit and production operation remain external even when a bounded
implementation earns a high A.

Eight of the 15 active crates have now reached their audited local target:
`rullst-macros`, `rullst-orm-macros`, `rullst-messaging`, `rullst-capital`,
`rullst-mail`, `rullst-auth`, `rullst-nexus`, and `cargo-rullst`. Seven remain
in the ceiling campaign.

| Crate | Current audited | Provisional local ceiling | Points remaining | Repository-owned evidence cluster | External boundary retained |
| :--- | ---: | ---: | ---: | :--- | :--- |
| `rullst-core` | 91/A | 96/A | 5 | Graceful/distributed runtime contracts, authorization composition and recovery evidence | Production topology and host domain authorization |
| `rullst-orm` | 91/A | 96/A | 5 | Cross-store semantics, recovery rehearsal and remaining bounded adapter matrices | Vendor operations and application tenant policy |
| `rullst-security` | 90/A | 96/A | 6 | SIEM rotation/acknowledgement contracts, broader threat negatives and recovery/SBOM evidence | Independent audit, certification and real SOC operation |
| `rullst-connect` | 90/A | 95/A | 5 | Transactional token-generation CAS, remaining revocation fixtures and lifecycle recovery | Live-provider conformance and IdP operations |
| `rullst` | 90/A | 96/A | 6 | Deeper cross-domain runtime composition and recovery contracts | Maturity inherited from external provider/device evidence |
| `rullst-auth` | 95/A | 95/A | 0 | Bounded shared local revocation/device lifecycle, restart and counter-CAS evidence complete for this campaign | Shared ceremonies, multi-host replication, refresh workflow and normative WebAuthn conformance |
| `rullst-mail` | 95/A | 95/A | 0 | Bounded inspection, durable shared-local suppression and minimized terminal observations complete for this campaign | Authoritative malware/CDR inspection, provider webhook conformance, multi-host operations, inbox placement, DNS reputation and live-provider acceptance |
| `rullst-messaging` | 96/A | 96/A | 0 | Encrypted local durability, canonical codec/trace and ORM outbox crash-replay contracts complete for this campaign | Remote broker operation, replication, full metadata encryption and cloud acceptance |
| `cargo-rullst` | 95/A | 95/A | 0 | All 270 structural profiles, eight generated-test/runtime cases, five public-CLI feature-axis cases and v5/v6/v11 transactional upgrade/recovery fixtures complete for this campaign | Production deployment/account acceptance |
| `rullst-ai` | 90/A | 95/A | 5 | Cancellation/streaming, distributed authenticated audit delivery and adaptive evaluations | Non-compatible protocols need adapters; model-provider behavior and corpus quality remain external |
| `rullst-studio` | 83/B | 94/A | 11 | Distributed trace ingestion plus authenticated cache/query inspection boundaries | Production topology and operator authorization |
| `rullst-iot` | 83/B | 83/B | 0 | Approved B exception retained outside the 15-crate ceiling campaign | Physical hardware, flashing/bootloader, broker/device interoperability and certification |
| `rullst-capital` | 93/A | 93/A | 0 | Signed-environment binding and bounded HMAC-chained local fiscal command audit/recovery complete the local target | Live gateway acceptance, authoritative multi-writer outbox/reconciliation and official fiscal homologation |
| `rullst-orm-macros` | 95/A | 95/A | 0 | Fail-closed structured parser, 24 exact UI diagnostics and generated runtime cross-evidence complete for this campaign | Compiler/ecosystem compatibility beyond the tested matrix |
| `rullst-nexus` | 95/A | 95/A | 0 | Trusted-context tenant scope, transaction-coupled audit and bounded admin operation contracts complete for this campaign | Host identity/domain policy, global/custom-route authorization, immutable audit delivery and production operation |
| `rullst-macros` | 94/A | 94/A | 0 | Bounded grammar/diagnostics, native server route, versioned Wasm transport, CSRF composition and generated-project evidence complete for this campaign | Real compiler/browser/network ecosystem matrix and host identity policy beyond CI |
| **Repository** | **1,466/1,600 = 91.6 (rounded 92/A)** | **1,509/1,600 = 94.3/A** | **43** | **Every gain still requires reviewed evidence** | **A+ remains outside this local planning ceiling** |

On this planning scale, **97.2% of the eventual score total is already
evidenced and 2.8% remains**. That percentage describes point distance, not
elapsed effort: the remaining points are concentrated in integration,
durability, failure recovery, distributed composition and operational matrices
and are therefore more expensive than early API/documentation points.
`rullst-iot` is the only accepted campaign result below A; its approved B exception reflects missing
physical/device evidence rather than lowering the release gate for the other
15 crates. This table must be re-audited whenever implementation reveals a
stronger or weaker boundary.

The final point allocation may differ from these candidate clusters after code
review. External provider acceptance, fiscal homologation, device testing,
store publication, and independent audit must stay explicitly external even
when enough repository-owned evidence exists to reach A.

## What the score does not measure

The score is not:

- feature completeness or roadmap percentage;
- a claim that every crate has the same maturity;
- provider acceptance, device/store validation, fiscal homologation, or a
  security/compliance certification;
- a benchmark or proof that Rullst is better than another framework;
- a substitute for the exact release gates on the candidate SHA.

Those questions belong to the [capability status](capability-status.md), the
[capability ledger](capability-ledger.md), and the release evidence. Keeping
these axes separate prevents a well-tested bounded foundation from being
mistaken for a finished remote integration.

## Interpreting changes between pushes

A score should change only when its evidence changes. The per-push review will
call out:

1. the previous and current SHA;
2. repository score and changed crate rows;
3. the exact gate responsible for a gain or loss;
4. feature-completeness movement separately, when applicable.

No points are added for code volume, number of features, marketing claims, or
raw test count alone.
