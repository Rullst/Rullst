# Rullst CI/CD and Verification Contract

This document describes what the repository's automation currently executes. It
is not evidence that a workflow has passed for a particular commit. A green
claim must always point to the GitHub Actions run, commit SHA, logs, and produced
artifacts.

Last source-level review: **2026-08-26**.

## Status language

| Status | Meaning |
| :--- | :--- |
| **Blocking** | A failing command fails that workflow run. Branch protection still determines whether the check is required for merging. |
| **Automated evidence** | The workflow runs automatically, but part of its result is external, uploaded, or deliberately non-blocking. |
| **Informational** | The workflow is explicitly advisory or manual and must not be described as a release gate. |
| **Roadmap** | The idea is preserved, but this repository does not yet provide reproducible evidence for it. |

The distinction matters: Kani, Miri, mutation testing, or a scanner can be very
valuable without proving that the entire framework is panic-free, race-free,
memory-safe, or compliant with a regulation.

## Required local and release baseline

The contributor baseline from `AGENTS.md` is:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all
```

The main CI uses the stricter all-target Clippy form and checks formatting
without modifying files:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

`ci.yml` also checks each ORM strict database feature in isolation, exercises
the runtime-only Core and minimal umbrella boundaries, runs PostgreSQL/MySQL
integration tests on Linux, and tests the all-feature workspace on Linux,
macOS, and Windows. A dedicated job checks the declared MSRV, Rust 1.96.0.

## Phase 4 release-engineering status

| Goal from `gpt.md` | Current status | Assessment |
| :--- | :--- | :--- |
| Trifecta with all features | **Implemented** | CI and tag release both run format, all-target/all-feature Clippy, and all-feature tests. |
| Strict DB features in isolation | **Partial** | `strict-postgres`, `strict-mysql`, and `strict-sqlite` compile independently. Provider-specific runtime tests for every strict mode remain worthwhile. |
| Honest blocking/informational labels | **Implemented** | Unsafe and Wasm checks are blocking; Kani, Miri, mutation testing, and udeps explicitly say they are informational. |
| Cover every fuzz target | **Implemented in workflow** | `fuzzing.yml` has one manual matrix entry for each of the 40 targets in the ten fuzz manifests. This records configuration, not a successful six-hour run. |
| Package all crates before publishing | **Implemented in workflow** | The tag-only release validates versions, packages all publishable workspace crates, hashes and attests the archives, then publishes in dependency order. |
| Unified evidence bundle per tag | **Implemented in workflow** | The tag-scoped bundle contains `Cargo.lock`, Cargo metadata, CycloneDX 1.5 SBOM, Cargo Audit JSON, `deny.toml`, bounded compliance evidence, governed advisory exceptions, commit/tag context, and checksums. The bundle and `.crate` archives are included in build-provenance attestation. |
| Align manifest, changelog, tag, registry, and notes | **Partial** | The release validates `vMAJOR.MINOR.PATCH` against publishable manifest versions. Changelog state and registry/release-note consistency are not automatically verified. **Worth implementing before calling 12.0.0 released.** |

## Important evidence boundaries

### Zero-panics and unsafe Rust

`zero-panics.yml` denies Clippy's unwrap, expect, panic, todo, and unimplemented
lints for published runtime libraries, procedural-macro engines, CLI production
targets, generated runtime templates, and the Wasm Core path. Tests are excluded
where assertion panics are test semantics.

`unsafe-policy.yml` compiles production libraries and binaries with
`-Dunsafe-code`. The only reviewed file-level exceptions are the Radar OS probe
and dynamic-library loader, and the workflow fails if that allowlist changes.
This is an enforced boundary, not a claim that all dependencies contain no
unsafe Rust.

### Coverage

`coverage.yml` runs LLVM coverage over workspace all-features tests plus the
PostgreSQL/MySQL matrix and uploads LCOV to Codecov. `codecov.yml` configures a
90% project target with 1% tolerance, but the upload action uses
`fail_ci_if_error: false` and excludes examples, CLI, macro crates, benchmarks,
and test files. Therefore “90% configured target” is accurate; “90% of the whole
repository is enforced” is not.

### Formal, dynamic, and stress analysis

- Kani and Miri are manual and `continue-on-error`; their results are research
  evidence scoped to the harnesses/packages that actually execute.
- Mutation testing is manual, split into eight shards, and intentionally
  non-blocking while results are uploaded.
- `cargo-udeps` is weekly/manual and explicitly non-blocking.
- TSan and ASan run daily/manual across eleven library packages. There is no
  MSan job in the current sanitizer workflow.
- The ZAP baseline is manual. It exercises the blog example, not every possible
  Rullst application or deployment.
- Property tests and benchmarks are scheduled/manual evidence. Benchmark alerts
  use a 20% threshold; they are not a promise against every nanosecond-level
  regression.

### Fuzzing and OSS-Fuzz

The manual `fuzzing.yml` matrix covers all **40** declared libFuzzer targets:
Core 12, ORM 5, Security 7, Connect 3, Mail 4, AI 3, IoT 3, Capital 1, Nexus 1,
and Studio 1.

The `oss-fuzz/projects/rullst` directory is a local integration draft. It is not
proof of upstream acceptance, continuous ClusterFuzz execution, or coverage of
all 40 targets; its helper build must be completed and validated against the
official OSS-Fuzz repository before submission. **The integration is worth
finishing, but a “100% first-pass acceptance” promise is not meaningful and
should not be made.**

### Supply chain and release provenance

All third-party GitHub Actions references are pinned to full commit SHAs. RustSec
exceptions are limited by `deny.toml` and documented with owners, controls, and
expiry dates in `docs/src/security-advisory-exceptions.md`.

`release.yml` is tag-only. It verifies source, validates the exact semantic tag
against every publishable crate, packages before the first publish, and creates
a tag-bound evidence bundle containing the lockfile, Cargo metadata, CycloneDX
1.5, Cargo Audit JSON, dependency policy, bounded compliance evidence, advisory
exceptions, commit context, and checksums. The `.crate` archives and evidence
receive a GitHub build-provenance attestation, while the official generic SLSA generator produces
release provenance. This does **not** by itself establish project-wide SLSA
Level 3 certification, Sigstore Cosign binary signing, or regulatory compliance.

## Workflow inventory (32 definitions)

Durations are intentionally omitted because runner load, cache state, and the
dependency graph make static estimates unreliable.

| Workflow | Trigger | Mode | Actual scope |
| :--- | :--- | :--- | :--- |
| [`ai-sentinel-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ai-sentinel-pr.yml) | pull requests | Automated evidence | Generates bounded CLI audit, compliance report, and CycloneDX SBOM artifacts; no certification claim. |
| [`audit.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/audit.yml) | main/dev push and PR, daily, manual | Blocking | Cargo Audit with the governed exception list. |
| [`bench.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/bench.yml) | main push, weekly, manual | Blocking run | Eight benchmark groups with 20% regression alerts and gh-pages history. |
| [`cargo-deny.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/cargo-deny.yml) | main/dev push and PR, weekly | Blocking | Advisory, license, ban, and source policy from `deny.toml`. |
| [`ci.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) | main/dev push, main PR | Blocking | Format, all-target/all-feature Clippy, multi-OS tests, DB feature boundaries, and MSRV. |
| [`codeql.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml) | main push/PR, weekly | Blocking run | Rust CodeQL after an all-target/all-feature workspace check. |
| [`corpus-sync.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/corpus-sync.yml) | weekly, manual | Informational | Attempts corpus minimization and uploads results; individual cmin failures are tolerated. |
| [`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml) | main/dev push and PR | Automated evidence | LLVM LCOV generation and non-blocking Codecov upload. |
| [`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml) | manual | On-demand | OWASP ZAP baseline against the blog example. |
| [`e2e-smoke.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/e2e-smoke.yml) | main/dev push, main PR, manual | Blocking | Boots the release blog example and checks HTTP, headers, form flow, and SQLite persistence. |
| [`fuzzing.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/fuzzing.yml) | manual | On-demand | Forty libFuzzer matrix jobs, each capped below six hours. |
| [`iot-integration.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/iot-integration.yml) | main/dev/develop push and PR, manual | Blocking | Host IoT tests, signed OTA invariants, and one Cortex-M no-std build; no hardware claim. |
| [`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml) | manual | Informational | Bounded Kani research harnesses; failures do not fail the workflow job. |
| [`machete.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/machete.yml) | main push/PR | Blocking | Unused dependency scan with configured exceptions. |
| [`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml) | manual | Informational | Miri package matrix with randomized layouts; failures are tolerated. |
| [`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml) | manual | Informational | Eight cargo-mutants shards with uploaded results. |
| [`no_std-build.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/no_std-build.yml) | main/dev/develop push and PR, manual | Blocking | Builds `rullst-iot` for three bare-metal targets; this is compile evidence, not hardware execution. |
| [`pages.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pages.yml) | main push, manual | Deploy | Builds and deploys project sites to GitHub Pages. |
| [`pqc-compliance.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pqc-compliance.yml) | relevant main/dev changes, weekly | Blocking | Signed OTA and Vault tests, RustSec audit, and simulator-boundary checks; explicitly no PQC/HSM certification. |
| [`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml) | weekly, manual | Blocking run | Release-mode property and workspace tests with configured case counts. |
| [`release.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml) | exact-looking version tags | Release | Tag validation, full verification, package-all, evidence bundle, checksums, attestations, dependency-order publish, and release provenance. |
| [`sanitizers.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml) | daily, manual | Blocking run | TSan and ASan library matrices on nightly Rust. |
| [`scorecards.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/scorecards.yml) | main push, weekly | Automated evidence | OpenSSF Scorecard analysis and SARIF/artifact upload; not SLSA certification. |
| [`security-audit.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/security-audit.yml) | main/dev push and PR, weekly, manual | Blocking | A second Cargo Audit schedule using the governed exception list. |
| [`semver.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/semver.yml) | main push/PR | Blocking | SemVer checks for publishable libraries; see the workflow for the exact package list. |
| [`spellcheck.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/spellcheck.yml) | main push/PR | Blocking | Repository typo scan. |
| [`tangleguard.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/tangleguard.yml) | main push, PR, manual | Blocking | Architecture findings fail the run. |
| [`trufflehog.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/trufflehog.yml) | main/dev push and PR, weekly | Blocking | Verified-secret scan over the configured Git history range. |
| [`udeps.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/udeps.yml) | weekly, manual | Informational | Nightly cargo-udeps signal; command failures are tolerated. |
| [`unsafe-policy.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/unsafe-policy.yml) | main push/PR | Blocking | Denies new production unsafe code and validates the reviewed exception allowlist. |
| [`wasm-matrix.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/wasm-matrix.yml) | main/dev push, main PR | Blocking | Compiles Core and macro crates for `wasm32-unknown-unknown` and `wasm32-wasip1`. |
| [`zero-panics.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/zero-panics.yml) | main/dev push, main PR | Blocking | Panic-family Clippy lints plus generated-code regression checks for published runtime targets. |

## Preserved next-generation roadmap

These ideas remain valuable, but are not current guarantees:

| Idea | Status and recommendation |
| :--- | :--- |
| Loom and Shuttle concurrency exploration | **Not implemented — worth implementing** for the small shared-state primitives that have explicit concurrency invariants. Do not apply them indiscriminately to the whole workspace. |
| `cargo-vet` dependency review | **Not implemented — worth implementing** once review ownership and audit criteria are defined; an empty policy file would add ceremony without assurance. |
| `cargo-careful` and zero-allocation assertions | **Not implemented — worth targeted experiments.** Allocation claims need stable benchmarks and explicit hot paths before becoming gates. |
| PGO and BOLT | **Not implemented — defer until production profiles exist.** Fixed throughput-gain percentages must not be promised in advance. |
| Chaos testing with `fail-rs` | **Not implemented — worth implementing** around queues, database retries, and provider timeouts after deterministic failure contracts exist. |
| AFL.rs/honggfuzz differential fuzzing | **Not implemented — valuable after the 40 libFuzzer targets have healthy corpora and triage ownership.** |
| Sigstore Cosign signing | **Not implemented.** Consider it for separately distributed binaries/containers; current `.crate` provenance and checksums should remain the immediate priority. |
| Absolute “100% pure Rustls” mandate | **Not established and not recommended as a marketing absolute.** Enforce an audited TLS dependency policy based on supported platforms and threat model instead. |
| Complete upstream OSS-Fuzz integration | **Partial draft — worth finishing.** Validate every intended target with `helper.py build_fuzzers` and `check_build`, then submit upstream; do not imply acceptance before merge. |

The goal of this roadmap is stronger, reproducible evidence—not a larger number
of badges or absolute claims that no finite test suite can establish.
