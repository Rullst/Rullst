# Rullst CI/CD and Verification Contract

This document describes what the repository's automation currently executes. It
is not evidence that a workflow has passed for a particular commit. A green
claim must always point to the GitHub Actions run, commit SHA, logs, and produced
artifacts.

Last source-level review: **2026-09-04**.

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

## Mainline execution model

The v12 dashboard and its automatic status badges are pinned to `main`. The
continuous workflows accept pushes to `main` and pull requests targeting it,
and expose `workflow_dispatch` where a safe rerun is useful. Superseded runs of
these workflows are cancelled per workflow and ref so rapid development does
not spend runner capacity proving an obsolete commit.

GitHub executes `schedule` events from the repository's default branch, so
scheduled and continuous v12 evidence now share the active `main` source line.
Tag publication remains deliberately unavailable through a manual button.

## Manual and periodic execution map

Every verification workflow except the PR-context-only `ai-sentinel-pr.yml`
and tag-only `release.yml` can now be started from **Actions → select workflow
→ Run workflow**. A manual run checks the selected branch's current SHA; record
that SHA and the run URL before treating it as release evidence. The release
workflow intentionally has no button because its publication authority begins
only with an exact version tag.

The workflows below run **only when requested manually**:

| Workflow | Evidence | RC interpretation |
| :--- | :--- | :--- |
| `dast-zap.yml` | OWASP ZAP baseline against a release blog showcase plus fresh generated REST API and complete LMS applications | REST/LMS warnings and failures block unless an exact rule ID is versioned as `INFO` with a local explanation in `.zap/`; those configs are passed explicitly to the pinned scanner and unlisted warnings remain live. The showcase is informational because it deliberately uses third-party presentation assets; reports and application logs are retained. This remains representative, not universal deployment coverage. |
| `fuzzing.yml` | All 40 declared libFuzzer targets from the validated shared inventory | Every matrix job must finish without a crash for the configured time budget; target-specific corpora are restored and saved, while failure reproducers are retained. The proc-macro parser keeps the same 5.5-hour budget in eleven strict 30-minute processes sharing one corpus, which bounds sanitizer RSS without weakening crash detection. This is bounded evidence, not proof for every input. |
| `kani.yml` | Twenty named bounded formal harnesses in ten supported runtime/library packages | Informational release evidence, but each proof has an isolated strict matrix job. Rullst itself stays on stable Rust 1.98.1 with a Rust 1.96 MSRV; only the separately built Kani verifier uses its pinned `nightly-2026-08-01` compiler (`rustc 1.99.0-nightly`) because the latest stable Kani bundle's Rust 1.93 compiler cannot compile the framework. The proc-macro-only `rullst-macros` target remains unsupported by Kani and is covered by compile-pass/fail and generated-project evidence instead. |
| `miri.yml` | Randomized-layout Miri execution over 15 named pure-Rust/default-feature scopes | Informational release evidence, but every selected scope is strict. This nightly-only interpreter uses pinned `nightly-2026-08-21` (`rustc 1.100.0-nightly`); it does not change the project's stable toolchain or MSRV. Native FFI, OS syscall, network/provider, umbrella re-export, and example-application boundaries are excluded explicitly rather than emitted as tolerated errors. |
| `mutants.yml` | Eight mutation-testing shards and their artifacts | Informational: review survived/timed-out mutants and the measured score. “Pass” does not honestly mean every possible mutant was killed. |

These workflows are **periodic and manually runnable**:

| Cadence | Workflows | Mode |
| :--- | :--- | :--- |
| Daily | `audit.yml`, `sanitizers.yml` | Cargo Audit is blocking; TSan/ASan are blocking when executed. |
| Weekly | `bench.yml`, `cargo-deny.yml`, `codeql.yml`, `corpus-sync.yml`, `coverage.yml`, `documentation.yml`, `pqc-compliance.yml`, `proptest.yml`, `scorecards.yml`, `security-audit.yml`, `trufflehog.yml`, `udeps.yml` | The inventory below identifies which results are blocking, automated evidence, or informational. Corpus sync warms and minimizes the same 40 validated target corpora with bounded parallelism. |

All remaining test/build workflows run on the documented push, pull-request or
path filters and also expose a manual rerun. For an RC checkpoint, first use the
automatic mainline suite, then manually run the five manual-only workflows and
any periodic/platform matrix whose latest successful run does not point to the
same candidate SHA. Physical devices, store approval, live provider accounts,
external security review and human release approval remain outside GitHub
Actions.

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

`ci.yml` also compiles and exercises each ORM strict database feature in
isolation (PostgreSQL, MySQL, and SQLite), exercises the runtime-only Core and
all 45 public umbrella features in isolated additive graphs with automatic
manifest-drift detection, runs the portable database matrix on Linux, and
tests the all-feature workspace on Linux, macOS, and Windows. The umbrella's
`cfg(doctest)` aggregation reads all 50 public tutorial files directly, so that
same command discovers the versioned Rust blocks, compiles or executes complete
examples, and records explicitly contextual fragments as ignored instead of pretending
they are standalone programs. Its pinned live Redis job also proves that
scheduled Core jobs are not claimed early, that Core cache inspection returns
bounded metadata without values, plus ORM
cache hit/TTL/recovery, tenant/table invalidation, rollback preservation,
process-local post-commit observers and Scout commit ordering. A separate
SQLite outbox contract runs on all three operating systems and covers
atomicity, conflicting idempotency keys, claim races, lease expiry,
retry and dead-letter; the relational matrix repeats the core outbox lifecycle
against PostgreSQL, MySQL, MariaDB and strict SQLite. A dedicated job checks
the declared MSRV, Rust 1.96.0. The Linux provider matrix also runs the
feature-gated Scout adapter against a digest-pinned Meilisearch image; Algolia
and Elasticsearch use bounded local protocol fixtures because no hosted
provider account is part of CI. The same matrix runs typed, parameterized L2
and cosine queries against a digest-pinned PostgreSQL + pgvector image. It also
runs Nexus's default Any/SQLite HTTP contract explicitly, because the global
all-feature graph intentionally selects a strict database profile and excludes
that materialized tenant/audit target. Coverage separately merges the default
workspace pass, so those routes contribute real executed-line evidence.

After those Rust CI jobs finish, an observational job always emits a
SHA-bound per-crate quality scorecard into the workflow summary and a 90-day
artifact. The score combines versioned expert-audit ceilings with the actual
gate results; a failed/skipped/cancelled gate can remove the dimensions it was
meant to prove, while a green gate cannot inflate a crate beyond its audited
ceiling. This is engineering-evidence reporting, not capability completion or
certification. See the [scorecard methodology](docs/src/quality-scorecard.md).

Rows with no feature selected compile every package target. Feature-selected
rows compile the isolated library graph; feature-enabled tests, examples, and
benchmarks remain covered by the workspace and specialist jobs. This avoids
pulling unrelated development dependencies into every boundary while retaining
real integration coverage.

The tag-only packaged-distribution gate reads the complete feature set from the
extracted `rullst` package manifest and compiles that crates-only consumer with
defaults disabled and every public feature enabled. A partial hand-maintained
feature allowlist therefore cannot make a monorepo-only integration appear
release-ready.

## Recommended `main` branch-protection profile

Require every job emitted by the following workflows before merging into
`main`: Rust CI, GitHub Actions Lint, Documentation, End-to-End Smoke Tests, Cargo Audit,
Security Audit, Cargo Deny, CodeQL, Test Coverage, Cargo Machete, SemVer Checks,
Spellcheck, Crate Architecture Policy, TruffleHog, Unsafe Policy, WebAssembly Matrix, Zero
Panics, no-std Build, IoT Integration, and PR Security Evidence.

Do not configure a path-filtered, scheduled, manual, deployment, or tag-only
workflow as a universal required check: an intentionally skipped workflow may
never create the check context. In particular, IoT Cryptography Containment is
blocking when relevant paths change, and the Omni desktop, Android and iOS
compile workflows are blocking only when the Omni generator boundary changes.
Pages, benchmarks, fuzzing, sanitizers,
Kani, Miri, mutation testing, udeps, ZAP, Scorecard, and release provenance
belong to deeper evidence or release policy. GitHub repository rulesets remain
the enforcement source; this document records the recommended profile and does
not claim that the hosted setting is already enabled.

## Phase 4 release-engineering status

| Goal from `gpt.md` | Current status | Assessment |
| :--- | :--- | :--- |
| Trifecta with all features | **Implemented** | CI and tag release both run format, all-target/all-feature Clippy, and all-feature tests. |
| Strict DB features in isolation | **Implemented in workflow** | `strict-postgres`, `strict-mysql`, and `strict-sqlite` compile independently and each runs a backend-specific CRUD test with only the selected strict feature enabled. |
| Honest blocking/informational labels | **Implemented** | Unsafe and Wasm checks are blocking; Kani, Miri, mutation testing, and udeps explicitly say they are informational. |
| Cover every fuzz target | **Implemented in workflow** | `.github/fuzz-targets.json` is the shared inventory for the manual campaign and corpus maintenance. A blocking validator compares it with all ten fuzz manifests and their 40 source files. This records configuration, not a successful six-hour run. |
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

`coverage.yml` runs LLVM coverage over workspace all-features and default
profiles plus the live database matrix, then uploads LCOV to Codecov using
GitHub OIDC rather than a long-lived upload secret. It also retains exact JSON
and text line summaries for 30 days so a passing upload cannot be confused
with the coverage percentage. `codecov.yml` requires at least 90%, with zero
tolerance, for the whole repository, the framework-library path set, and
changed lines; failure to upload LCOV also fails the workflow. The report
filters examples, benchmarks, auxiliary test support, and separate test files.
CLI and proc-macro code therefore remains part of the blocking repository
aggregate and is additionally visible as informational components. Their
stronger semantic evidence still comes from materialized scaffolds and
compile-pass/compile-fail contracts. The README exposes both the public overall
badge and the separate `framework_libraries` badge rather than substituting the
higher component result for the repository total.

### Formal, dynamic, and stress analysis

- Kani and Miri are manual research evidence scoped to the harnesses/packages
  that actually execute. Rullst itself remains pinned to stable Rust 1.98.1
  and keeps Rust 1.96 as its declared MSRV. Kani builds reviewed upstream
  revision `8fcd6d90ed07b559e553ca8a92b95f2db69b2c78` with the verifier's own pinned
  `nightly-2026-08-01` compiler (`rustc 1.99.0-nightly`) into a bundle and
  installer, then treats proof failures in twenty
  isolated harness jobs across ten supported packages as real matrix failures.
  That revision intentionally predates a `compare_bytes` compiler crash
  reproduced with Kani's first `rustc 1.100.0-nightly` snapshot. The workflow
  does not patch manifests, bypass MSRV data, or change Rullst's stable
  toolchain. Kani cannot verify the proc-macro-only `rullst-macros` target.
  Miri, which only runs on nightly Rust, pins `nightly-2026-08-21`
  (`rustc 1.100.0-nightly`) and strictly executes 15 named
  pure-Rust/default-feature scopes. Its matrix excludes native `ring`, AWS-LC,
  SQLite, OS-syscall and network/provider execution that Miri cannot interpret;
  native CI, integration tests and sanitizers remain the applicable evidence
  for those paths. The `rullst` umbrella re-export facade and Blog example add
  no separate interpreter scope. A selected-scope failure fails the run. The
  Kani Security harnesses prove pure production decisions such as Vault key-ID
  character policy, DLP buffer admission, ASCII-folded RASP matching, SRI asset
  limits and bounded Login Guard delay; the IoT matrix also proves the complete
  CoAP option-component classification. They do not claim that Kani verifies
  `zeroize`'s unsupported inline assembly, cryptographic implementations or the
  entire concurrent middleware implementations.
- Mutation testing is manual, split into eight shards, and intentionally
  non-blocking while results are uploaded.
- Fuzzing and corpus maintenance pin `nightly-2026-08-21` instead of following
  a moving nightly alias. This verifier-only toolchain does not change the
  framework's stable Rust 1.98.1 toolchain or its Rust 1.96 MSRV. The parser
  campaign restarts its ASan process every 30 minutes while retaining one
  corpus and the full 5.5-hour target budget, preventing instrumentation RSS
  accumulation from masquerading as a parser crash.
- Branch coverage, `cargo-udeps`, TSan and ASan share the reviewed
  `nightly-2026-08-21` analysis snapshot instead of following a moving nightly
  alias. The first two remain observational/informational; sanitizer failures
  remain blocking whenever their daily/manual matrix executes.
- `cargo-udeps` is weekly/manual and explicitly non-blocking.
- TSan and ASan run daily/manual across twelve runtime/domain packages;
  Messaging runs its integration contract so its concurrent state is actually
  exercised rather than reporting a zero-test library pass. There is no MSan
  job in the current sanitizer workflow.
- The manual ZAP workflow materializes, release-builds and migrates a fresh
  REST API and complete LMS through the real CLI. Both baselines fail on any
  warning/failure, preserve INFO observations and use no ignored rules. The
  release blog showcase is scanned separately but remains informational because
  its documented presentation boundary deliberately uses a relaxed CSP and
  third-party assets. Its rules retain those external-asset findings for review
  and reduce only evidenced token/state signals or escaped showcase reflections
  to INFO; they do not hide findings with `IGNORE`. These three targets are representative evidence, not
  coverage of every blueprint, authenticated role, browser, proxy or deployment.
- Property tests and benchmarks are scheduled/manual evidence. The eight
  published benchmark groups, backed by nine Criterion binaries, emit
  non-blocking alerts at a 20% regression and feed the
  [public benchmark hub](https://rullst.github.io/Rullst/benches/); they are not
  a promise against every nanosecond-level regression.

### Fuzzing and OSS-Fuzz

The manual `fuzzing.yml` matrix covers all **40** declared libFuzzer targets:
Core 12, ORM 5, Security 7, Connect 3, Mail 4, AI 3, IoT 3, Capital 1, Nexus 1,
and Studio 1. The checked-in `.github/fuzz-targets.json` is validated against
every `*/fuzz/Cargo.toml` and source file before either the manual campaign or
weekly corpus job can fan out. Both jobs use versioned per-target caches;
campaign failures retain their exact reproducer, and the weekly job performs a
bounded warm-up before minimizing and uploading each actual corpus. A clean
run remains evidence only for its exact SHA, target, corpus, toolchain and time
budget.

The `oss-fuzz/projects/rullst` directory is a local integration draft. It is not
proof of upstream acceptance, continuous ClusterFuzz execution, or coverage of
all 40 targets; its helper build must be completed and validated against the
official OSS-Fuzz repository before submission. **The integration is worth
finishing, but a “100% first-pass acceptance” promise is not meaningful and
should not be made.**

### Supply chain and release provenance

All direct third-party GitHub Actions references in this repository's workflow
files are pinned to full commit SHAs. A pinned composite action can still carry
its own transitive downloads or references, so blocking integrations must also
be reviewed for that behavior. RustSec exceptions are limited by `deny.toml`
and documented with owners, controls, and expiry dates in
`docs/src/security-advisory-exceptions.md`.

`scorecards.yml` runs the pinned OpenSSF Scorecard action on `main` pushes and
weekly, uploads SARIF to GitHub code scanning, and publishes OIDC-authenticated
results to the public Scorecard API so the README badge follows the latest
completed analysis. The numeric score is supply-chain evidence, not a security
certification.

`release.yml` is tag-only. It verifies source, validates the exact semantic tag
against every publishable crate, packages before the first publish, and creates
a tag-bound evidence bundle containing the lockfile, Cargo metadata, CycloneDX
1.5, Cargo Audit JSON, dependency policy, bounded compliance evidence, advisory
exceptions, commit context, and checksums. The `.crate` archives and evidence
receive a GitHub build-provenance attestation, while the official generic SLSA generator produces
release provenance. This does **not** by itself establish project-wide SLSA
Level 3 certification, Sigstore Cosign binary signing, or regulatory compliance.

`workflow-lint.yml` validates all workflow syntax, GitHub expressions, and
embedded shell with Actionlint 1.7.7. Its container is pinned to an immutable
linux/amd64 digest, just like third-party GitHub Actions are pinned to full
commit SHAs.

`architecture.yml` is repository-owned and deterministic. It rejects any
internal dependency edge or optionality change that is not reflected in the
reviewed `crate-architecture-policy.json`. The earlier TangleGuard integration
was removed because its composite action downloaded an unversioned `latest`
binary without a repository-pinned checksum, which was unsuitable for a
blocking supply-chain gate.

## Workflow inventory (37 definitions)

Durations are intentionally omitted because runner load, cache state, and the
dependency graph make static estimates unreliable.

| Workflow | Trigger | Mode | Actual scope |
| :--- | :--- | :--- | :--- |
| [`ai-sentinel-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ai-sentinel-pr.yml) | pull requests | Automated evidence | Generates bounded CLI audit, compliance report, and CycloneDX SBOM artifacts; no certification claim. |
| [`architecture.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/architecture.yml) | main push and PR, manual | Blocking | Compares Cargo's publishable non-dev internal dependency graph with the reviewed `crate-architecture-policy.json`; unreviewed normal/build edges, removals, or optionality changes fail, while test-only dev-dependencies do not masquerade as production coupling. |
| [`audit.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/audit.yml) | main push and PR, daily, manual | Blocking | Cargo Audit with the governed exception list. |
| [`bench.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/bench.yml) | main push, weekly, manual | Automated evidence | Eight published groups backed by nine Criterion binaries, with non-blocking 20% regression alerts and gh-pages data consumed by the benchmark hub. Scheduled runs use the repository default branch. |
| [`cargo-deny.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/cargo-deny.yml) | main push and PR, weekly, manual | Blocking | Advisory, license, ban, and source policy from `deny.toml`. |
| [`ci.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) | main push and PR, manual | Blocking plus observational report | Format, all-target/all-feature Clippy, multi-OS tests including Cargo-aware doctests sourced from all 50 tutorials, the SQLite transactional outbox contract and Messaging concurrency suite, relational/polyglot live matrices, isolated strict-DB/feature boundaries, MSRV, and an always-generated SHA-bound per-crate quality scorecard artifact. |
| [`codeql.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml) | main push and PR, weekly, manual | Blocking run | Rust CodeQL after an all-target/all-feature workspace check. |
| [`corpus-sync.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/corpus-sync.yml) | weekly, manual | Informational | Validates the shared 40-target inventory, restores each real target corpus, performs a bounded warm-up, minimizes it and uploads the result; individual target failures are retained but tolerated. |
| [`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml) | main push and PR, weekly, manual | Blocking plus observational job | LLVM LCOV generation and blocking OIDC-authenticated Codecov upload; scheduled/manual branch instrumentation is non-blocking and uses the pinned verifier-only nightly. |
| [`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml) | manual | Blocking generated targets plus informational showcase | Pins the ZAP image by digest, scans fresh release/migrated REST API and complete LMS surfaces as blocking gates, scans the CDN-backed blog showcase informationally, and uploads separate reports plus application logs. |
| [`documentation.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/documentation.yml) | main push and PR, weekly, manual | Blocking plus informational external scan | Builds the mdBook, validates the landing/benchmark HTML templates and local site assets, checks landing JavaScript syntax, and rejects broken repository-local links. Scheduled/manual runs also upload a non-blocking external-link report because third-party availability and rate limits are not deterministic contribution gates. |
| [`e2e-smoke.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/e2e-smoke.yml) | main push and PR, manual | Blocking | Boots the release blog example and checks HTTP, headers, form flow, and SQLite persistence. |
| [`fuzzing.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/fuzzing.yml) | manual | On-demand | Forty validated libFuzzer matrix jobs, each capped below six hours, with per-target corpus caching and failure reproducers. The parser's single matrix job restarts its ASan-instrumented process every 30 minutes while sharing the same corpus and preserving the full 5.5-hour campaign budget. |
| [`iot-integration.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/iot-integration.yml) | main push and PR, manual | Blocking | Host IoT tests, signed OTA invariants, and one Cortex-M no-std build; no hardware claim. |
| [`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml) | manual | Informational release evidence; proof jobs are strict | Builds an immutable reviewed Kani snapshot with the verifier-only `nightly-2026-08-01` compiler, while Rullst stays on stable Rust 1.98.1 with a Rust 1.96 MSRV. It verifies twenty named bounded harnesses in isolated jobs across ten supported packages. Proof failures fail their matrix jobs; the proc-macro-only crate remains outside Kani's supported targets. |
| [`machete.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/machete.yml) | main push and PR, manual | Blocking | Unused dependency scan with configured exceptions. |
| [`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml) | manual | Informational release evidence; selected scopes are strict | Pinned nightly-only Miri executes 15 named pure-Rust/default-feature scopes with randomized layouts without changing Rullst's stable toolchain or MSRV. Native FFI/syscall/network paths, the umbrella re-export facade, and the Blog example are explicit boundaries; selected-scope failures fail the workflow. |
| [`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml) | manual | Informational | Eight pinned cargo-mutants 27.1.0 shards with uploaded results. |
| [`no_std-build.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/no_std-build.yml) | main push and PR, manual | Blocking | Builds `rullst-iot` for three bare-metal targets; this is compile evidence, not hardware execution. |
| [`omni-android.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/omni-android.yml) | relevant main changes and PRs, manual | Blocking when triggered | Generates a fresh deterministic Omni shell, initializes Android and compiles an unsigned aarch64 debug APK. It does not test a physical device, Play testing, signing, privacy declarations or store acceptance. |
| [`omni-desktop.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/omni-desktop.yml) | relevant main changes and PRs, manual | Blocking when triggered | Generates a fresh deterministic HTTPS-backed shell and checks its Tauri crate on Linux, macOS and Windows. It does not build/sign every installer or exercise a GUI/WebView session. |
| [`omni-ios.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/omni-ios.yml) | relevant main changes, manual | Blocking | Generates a fresh deterministic Omni iOS shell on macOS and compiles it for the runner's simulator architecture. It does not test a physical device, signing, privacy declarations, TestFlight or App Store acceptance. |
| [`pages.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pages.yml) | main push, manual | Deploy | Validates and deploys the unreleased v12 landing page, local visual assets, mdBook and benchmark hub/dashboards to GitHub Pages while preserving history data fetched from `gh-pages`. |
| [`pqc-compliance.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pqc-compliance.yml) | relevant main changes, weekly, manual | Blocking | Signed OTA and Vault tests, RustSec audit, and simulator-boundary checks; explicitly no PQC/HSM certification. |
| [`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml) | weekly, manual | Blocking run | Release-mode property and workspace tests with configured case counts. |
| [`release.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml) | exact-looking version tags | Release | Tag validation, full verification, package-all, evidence bundle, checksums, attestations, dependency-order publish, and release provenance. |
| [`sanitizers.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml) | daily, manual | Blocking run | TSan and ASan library matrices on pinned `nightly-2026-08-21`; this verifier toolchain does not change Rullst's stable compiler or MSRV. |
| [`scorecards.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/scorecards.yml) | main push, weekly, manual | Automated evidence | OpenSSF Scorecard analysis and SARIF/artifact upload; not SLSA certification. |
| [`security-audit.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/security-audit.yml) | main push and PR, weekly, manual | Blocking | Cross-checks active advisory IDs and expiry metadata across the ledger, Cargo Deny, and scanner workflows, then independently reruns Cargo Audit. |
| [`semver.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/semver.yml) | main push and PR, manual | Blocking | Compares each supported, already-published library API with its exact latest non-yanked crates.io baseline. Never-published packages and proc-macro/binary API surfaces unsupported by `cargo-semver-checks` are reported explicitly. |
| [`spellcheck.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/spellcheck.yml) | main push and PR, manual | Blocking | Repository typo scan. |
| [`trufflehog.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/trufflehog.yml) | main push and PR, weekly, manual | Blocking | Verified-secret scan over the configured Git history range. |
| [`udeps.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/udeps.yml) | weekly, manual | Informational | `cargo-udeps` signal on pinned `nightly-2026-08-21`; command failures are tolerated. |
| [`unsafe-policy.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/unsafe-policy.yml) | main push and PR, manual | Blocking | Denies new production unsafe code and validates the reviewed exception allowlist. |
| [`wasm-matrix.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/wasm-matrix.yml) | main push and PR, manual | Blocking | Compiles Core, the public `rullst` facade and macros for `wasm32-unknown-unknown` and `wasm32-wasip1`. |
| [`workflow-lint.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/workflow-lint.yml) | main push and PR, manual | Blocking | Validates the shared fuzz inventory, then Actionlint checks workflow syntax, GitHub expressions and embedded shell using an immutable container digest. |
| [`zero-panics.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/zero-panics.yml) | main push and PR, manual | Blocking | Panic-family Clippy lints plus generated-code regression checks for published runtime targets. |

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
