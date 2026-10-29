# Verification Dashboard

The live status of Rullst's continuous gates on the `main` branch, and what
each one actually checks. Badges report the latest matching run; they are not
a certification or a deployment guarantee. [CI/CD and security workflows](workflows.md)
documents the exact scope of all 37 workflow definitions and the
branch-protection profiles.

Scheduled events use the repository's default branch, so scheduled and
continuous evidence refer to `main`; the frozen v12 line has its own branch.

## Continuous and change-aware gates

| Gate | `main` status | Actual scope |
| :--- | :---: | :--- |
| **Rust CI** | [![Rust CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=flat-square&label=Rust%20CI)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain) | Format, all-target/all-feature Clippy, a Linux gate on pull requests, every Linux shard after merge and the complete Linux/macOS/Windows matrix nightly; Cargo-aware doctests sourced from every tutorial and the main guides, strict DB boundaries, feature boundaries, generated-code checks, and MSRV 1.96.0. |
| **Declared MSRV** | [![MSRV 1.96.0](https://img.shields.io/badge/MSRV-1.96.0-f74c00?style=flat-square&logo=rust)](compatibility-policy.md) | Every publishable manifest declares Rust 1.96.0 and CI runs an explicit workspace all-feature check with that toolchain. |
| **GitHub Actions lint** | [![Workflow Lint](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/workflow-lint.yml?branch=main&style=flat-square&label=Workflow%20Lint)](https://github.com/Rullst/Rullst/actions/workflows/workflow-lint.yml?query=branch%3Amain) | Validates workflow syntax, expressions, embedded shell, and full-SHA third-party Action pins. |
| **Documentation** | [![Documentation](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/documentation.yml?branch=main&style=flat-square&label=Docs)](https://github.com/Rullst/Rullst/actions/workflows/documentation.yml?query=branch%3Amain) | Builds the mdBook and rejects broken local links and anchors; scheduled/manual runs also preserve an informational external-link report. |
| **End-to-end smoke** | [![E2E](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?branch=main&style=flat-square&label=E2E)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml?query=branch%3Amain) | Boots the release blog example and verifies HTTP, security headers, form flow, and SQLite persistence. |
| **Codecov — whole repository** | [![Whole-repository coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg)](https://codecov.io/gh/Rullst/Rullst) | The badge reports the current branch aggregate. The enforced repository floor is **≥90%** with zero tolerance. For reference, the [LLVM run](https://github.com/Rullst/Rullst/actions/runs/34895523751) on the v12.0.0 release commit `eb11f892` measured **90.3220%** (79,813/88,365 lines) before Codecov upload. |
| **Codecov — framework libraries** | [![Framework library coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries)](https://codecov.io/gh/Rullst/Rullst) | Runtime libraries are enforced separately at **≥90%**. CLI and proc-macro components stay separately visible; [Coverage CI](https://github.com/Rullst/Rullst/actions/workflows/coverage.yml?query=branch%3Amain) uploads their real LCOV evidence with OIDC. |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?branch=main&style=flat-square&label=RustSec)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml?query=branch%3Amain) | RustSec advisory scan with only governed, expiring exceptions; the daily run also audits the `v12` locks. |
| **Security exception governance** | [![Security Governance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/security-audit.yml?branch=main&style=flat-square&label=Exception%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/security-audit.yml?query=branch%3Amain) | Cross-checks scanner allowlists against the owner/expiry ledger, then independently reruns Cargo Audit. |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?branch=main&style=flat-square&label=Cargo%20Deny)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml?query=branch%3Amain) | Advisory, license, ban, and source policy. |
| **CodeQL SAST** | [![CodeQL](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?branch=main&style=flat-square&label=CodeQL)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml?query=branch%3Amain) | Rust semantic analysis after an all-target/all-feature build, plus JavaScript/TypeScript analysis. |
| **OpenSSF Scorecard** | [![OpenSSF Scorecard](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square)](https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst) | The badge renders the score from the official public Scorecard JSON report; the pinned [Scorecard workflow](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) publishes OIDC-authenticated results on each `main` push and weekly. A score is evidence, not a security certification. [Evidence and improvements](openssf-scorecard.md). |
| **Cargo Machete** | [![Machete](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?branch=main&style=flat-square&label=Machete)](https://github.com/Rullst/Rullst/actions/workflows/machete.yml?query=branch%3Amain) | Unused direct dependency detection. |
| **SemVer checks** | [![SemVer](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?branch=main&style=flat-square&label=SemVer)](https://github.com/Rullst/Rullst/actions/workflows/semver.yml?query=branch%3Amain) | Supported library APIs are compared with exact latest non-yanked registry baselines; never-published packages and unsupported proc-macro/binary surfaces are reported explicitly. |
| **Zero-panics policy** | [![Zero Panics](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?branch=main&style=flat-square&label=Zero%20Panics)](https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml?query=branch%3Amain) | Denies panic-family operations in published production targets and generated runtime templates. |
| **Unsafe boundary** | [![Unsafe Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?branch=main&style=flat-square&label=Unsafe%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml?query=branch%3Amain) | Denies new production unsafe code outside the reviewed OS/FFI allowlist. |
| **Secret scanning** | [![TruffleHog](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/trufflehog.yml?branch=main&style=flat-square&label=Secrets)](https://github.com/Rullst/Rullst/actions/workflows/trufflehog.yml?query=branch%3Amain) | Verified-secret scan across the configured Git history range. |
| **Spellcheck** | [![Spellcheck](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/spellcheck.yml?branch=main&style=flat-square&label=Spellcheck)](https://github.com/Rullst/Rullst/actions/workflows/spellcheck.yml?query=branch%3Amain) | Repository-wide typo detection. |
| **Crate architecture policy** | [![Architecture](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/architecture.yml?branch=main&style=flat-square&label=Architecture)](https://github.com/Rullst/Rullst/actions/workflows/architecture.yml?query=branch%3Amain) | Compares the real publishable-crate dependency graph with a versioned, reviewed repository policy. |
| **WebAssembly matrix** | [![Wasm](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/wasm-matrix.yml?branch=main&style=flat-square&label=Wasm)](https://github.com/Rullst/Rullst/actions/workflows/wasm-matrix.yml?query=branch%3Amain) | Compiles Core, the public facade and macros for browser Wasm and WASI Preview 1. |
| **Bare-metal `no_std` matrix** | [![no_std](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/no_std-build.yml?branch=main&style=flat-square&label=no_std)](https://github.com/Rullst/Rullst/actions/workflows/no_std-build.yml?query=branch%3Amain) | Builds IoT helpers for Cortex-M and RISC-V targets; this is compile evidence, not hardware testing. |
| **IoT integration** | [![IoT](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/iot-integration.yml?branch=main&style=flat-square&label=IoT)](https://github.com/Rullst/Rullst/actions/workflows/iot-integration.yml?query=branch%3Amain) | Host tests, signed OTA invariants, and a Cortex-M build. |
| **IoT crypto containment** | [![IoT Crypto](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/pqc-compliance.yml?branch=main&style=flat-square&label=Crypto%20Boundary)](https://github.com/Rullst/Rullst/actions/workflows/pqc-compliance.yml?query=branch%3Amain) | Path-aware signed OTA, Vault, advisory, and simulator-boundary checks; no PQC/HSM certification claim. |
| **PR security evidence** | [![PR-only evidence](https://img.shields.io/badge/trigger-PR--only-2563eb?style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/ai-sentinel-pr.yml?query=event%3Apull_request) | Pull-request-only bounded IDOR/RBAC heuristics and CycloneDX SBOM artifact. It intentionally has no continuous `main` status. |
| **PR mutation diff** | [![PR-only evidence](https://img.shields.io/badge/trigger-PR--only-2563eb?style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/mutants-pr.yml?query=event%3Apull_request) | Pull-request-only cargo-mutants run over the changed Rust lines; a missed mutant fails it. It intentionally has no continuous `main` status. |

## Deep evidence

Deep or irreversible workflows are intentionally not presented as continuously
green `main` gates:

| Workflow | Trigger and enforcement |
| :--- | :--- |
| [Benchmark regression](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Weekly, `main` push, or manual; eight published groups backed by nine Criterion benchmark binaries emit non-blocking alerts at a 20% regression. |
| [Property testing](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Weekly/manual release-mode invariant testing with 10,000 configured cases. |
| [TSan and ASan](https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml) | Daily/manual package matrices on a pinned verifier-only nightly. |
| [Fuzzing](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) / [corpus minimization](https://github.com/Rullst/Rullst/actions/workflows/corpus-sync.yml) | Forty-two manual libFuzzer jobs, one per declared target, required as release evidence; weekly/manual corpus maintenance is informational. |
| [OWASP ZAP](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Manual baseline over three release surfaces: the generated REST API and the LMS example are blocking with no ignored alerts; the deliberately CDN-backed blog showcase remains an explicitly informational boundary. |
| [Kani](https://github.com/Rullst/Rullst/actions/workflows/kani.yml), [Miri](https://github.com/Rullst/Rullst/actions/workflows/miri.yml), [mutation testing](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml), [cargo-udeps](https://github.com/Rullst/Rullst/actions/workflows/udeps.yml) | Manual or scheduled research signals: selected Kani/Miri scopes are strict, while mutation and unused-dependency findings remain explicitly informational. |
| [v13 Verus pilot](https://github.com/Rullst/Rullst/blob/main/.github/workflows/verus.yml) | Optional production-linked age-policy proof with pinned tooling and three negative controls. Hosted registration/acceptance remains pending; no framework-wide correctness claim. |
| [GitHub Pages](https://github.com/Rullst/Rullst/actions/workflows/pages.yml) | Deploys development documentation from `main`; it is not a code-quality gate. |
| [Release and provenance](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Exact version tags only: exact-SHA admission of the full CI matrix, package-all, evidence bundle, checksums, GitHub build-provenance attestation, changelog-derived release notes, and ordered crates.io publication. This does **not** claim a project-wide SLSA level or independent certification. |

## Related evidence

- [How the repository is maintained](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md):
  workstation rules, CI gates, fuzzing evidence and release admission.
- [Capability Status & Vision Decisions](capability-ledger.md) preserves
  ambitious features that are partial or not implemented, with an explicit
  recommendation and rationale for each one.
- [Simple Capability Status](capability-status.md) and the
  [per-commit quality scorecard](quality-scorecard.md) keep feature progress
  separate from SHA-bound engineering evidence.
- The [v12 release audit](v12-release-audit.md) links the platform, property,
  Miri, Kani, fuzzing, sanitizer and publication results of the last stable
  release.
