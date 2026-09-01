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
concurrent in-memory plus durable SQLite restart/corruption contract. Failed,
cancelled, or skipped applicable gates suppress
the dimensions they prove; the report is still generated so a red push cannot
hide its note.

Documentation/DX evidence also includes a Cargo-aware aggregation of the 50
public tutorials. It consumes the Markdown files directly during the normal
all-feature doctest run, so a green workspace test proves the standalone Rust
examples compile on that SHA; explicitly contextual fragments remain visible as
ignored and do not count as compiled examples.

Grades use the following fixed bands: A+ 97–100, A 90–96, B 80–89, C 70–79,
D 60–69, and F below 60.

The v12 RC quality objective is **no crate below B**, with the central Core and
ORM crates at A. A+ is an evidence threshold, not a value to assign by intent.
The audited ceilings below now satisfy that floor after dedicated macro
contracts and negative compiler cases plus an IoT durable-counter boundary with
restart/failure proof. This does not pre-approve a commit: the exact SHA still
earns those ceilings only when every constraining gate succeeds.

## Audited green-gate ceilings — 1 September 2026

These are the maximum current scores when every referenced Rust CI gate passes.
They are not presumed results for a new commit; the exact per-SHA artifact
applies the real gate outcomes and includes the full finding for every row.

| Crate | Ceiling | Grade | Principal remaining evidence boundary |
| :--- | ---: | :---: | :--- |
| `rullst-core` | 91 | A | Distributed deployment and host authorization |
| `rullst-orm` | 91 | A | Cross-store semantics, disaster recovery and remaining operational adapters |
| `rullst-security` | 89 | B | Durable SIEM, external audit and certification |
| `rullst-connect` | 88 | B | Durable/distributed token lifecycle and provider conformance |
| `rullst` | 88 | B | Inherits uneven maturity from opt-in domain crates |
| `rullst-auth` | 87 | B | WebAuthn conformance and durable shared revocation/devices |
| `rullst-mail` | 87 | B | Inbox/provider evidence, opaque-file inspection and distributed operations |
| `rullst-messaging` | 86 | B | Remote protocols, replication, encrypted-at-rest operations and provider fault matrices |
| `cargo-rullst` | 85 | B | Complete generated-project matrix across every blueprint |
| `rullst-ai` | 84 | B | Live adaptive evals, provider loops and external retrievers |
| `rullst-studio` | 83 | B | Distributed observability and remote inspectors |
| `rullst-capital` | 82 | B | Gateway parity and live fiscal homologation |
| `rullst-orm-macros` | 82 | B | Runtime protocol semantics remain evidence owned by `rullst-orm` |
| `rullst-nexus` | 81 | B | Host tenant policy, durable audit and production operations |
| `rullst-macros` | 81 | B | Experimental browser/runtime integration remains bounded |
| `rullst-iot` | 81 | B | Concrete hardware storage, transport, flashing and bootloader evidence |
| **Repository (equal-crate aggregate)** | **85** | **B** | **Exact score remains conditional on the SHA's gates** |

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
