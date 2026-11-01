# Rullst CI/CD and Verification Contract

This document describes what the repository's automation currently executes. It
is not evidence that a workflow has passed for a particular commit. A green
claim must always point to the GitHub Actions run, commit SHA, logs, and produced
artifacts.

Last source-level review: **2026-09-20 UTC**, with later workflow changes
recorded through 13.0.0-alpha.4.

## Status language

| Status | Meaning |
| :--- | :--- |
| **Blocking** | A failing command fails that workflow run. Branch protection still determines whether the check is required for merging. |
| **Automated evidence** | The workflow runs automatically, but part of its result is external, uploaded, or deliberately non-blocking. |
| **Informational** | The workflow is explicitly advisory and must not be described as a release gate. A manual trigger alone does not make a strict candidate check informational. |
| **Roadmap** | The idea is preserved, but this repository does not yet provide reproducible evidence for it. |

The distinction matters: Kani, Miri, mutation testing, or a scanner can be very
valuable without proving that the entire framework is panic-free, race-free,
memory-safe, or compliant with a regulation.

## Branches

`main` carries v13 development; the transitional `v13` integration branch was
retired into it. `v12` was preserved from stable source `184bc1f7` on
21 September 2026 and has been frozen since its end of life on 16 October 2026;
it keeps its own copies of the workflows.

The default-branch Dependabot configuration targets Cargo (the root and the
eleven fuzz workspaces) and Actions on `main` only. Minor/patch Cargo changes
share a PR, the OpenTelemetry and utoipa families each have their own, and
Actions share one tools PR. Updates still require compatibility/MSRV and
protected CI. Old Dependabot PRs are closed after equivalent updates have
entered `main`, or with a comment when its policy excludes them.

## Mainline execution model

The development dashboard and its automatic status badges follow `main`.
Automatic workflows accept pushes and pull requests targeting `main`, `v12`
and `v13`, and expose `workflow_dispatch` where a safe rerun is useful.
Superseded runs are cancelled per workflow and ref, so rapid development does
not spend runner capacity proving an obsolete commit.

Release policy schema 4 binds v12 to `v12` and v13 to `main`; historical
schemas 2 and 3 keep their original bindings. Before release artifacts are
built, the tag, every inventory package version and the current protected
branch head must agree. Exact-source workflow admission and the crates.io
deployment approval remain mandatory. Fuzz receipts are limited to the
candidate's release line, including the source policy of reused runs.

### Rust CI by event

`ci.yml` treats the expensive operating-system matrix differently by event:

- **Draft pull requests** get Format and Clippy feedback.
- **Ready pull requests to `main`** run the fast Linux gate: the `workspace`
  and `cli-standard` shards, MSRV, strict database features, Redis/S3
  contracts, generated access, facade composition and AI evals; the ORM
  differential database job compares backends only when the pull request
  changes the ORM, its macros, the lockfile or `ci.yml` (otherwise it reports
  a pass without building). Line coverage, the SemVer registry comparison and
  CodeQL skip them; the SemVer workflow's public-API snapshot job runs.
- **The `full-matrix` label** on a pull request to `main` runs the complete
  Linux/macOS/Windows `Run Tests` matrix (seven shards per system) before
  merge, on the `labeled` event and every later push; other jobs keep their
  pull-request guards. Every pull-request event shares the pull request's
  concurrency group, so the newest run still reports every required check
  (`.github/test-ci-matrix.py` evaluates the expressions). Any other label
  re-runs the Linux gate, because a skipped job would report a passing check.
- **A push to `main`** runs every Linux shard, public feature boundaries, the
  threat-model minimum, coverage, SemVer and CodeQL. macOS and Windows
  evidence for `main` comes from the **nightly run** (05:30 UTC), which
  executes the complete matrix on the default branch.
- **Ready pull requests to `v12` or `v13`** keep the complete matrix, blocking
  line coverage, SemVer fan-out and CodeQL before merge.
- **Release candidates** use the manual full matrix when no successful
  full-matrix run points to the exact candidate tree.

Each operating system runs seven shards: the non-CLI workspace (with the
Blog, ERP, LMS, Portfolio and WebGPU examples), ordinary CLI targets, three
public-profile groups and two generated-starter groups; the partitions keep
every case. Hosted CI permits two nested compiler jobs for generated builds,
and each CLI shard fetches the locked registry inventory before its generated
applications compile offline.

The SHA-bound quality scorecard comes only from a ready pull request to a
maintained line, the nightly run or a manual full-matrix run, never from a
Linux-only run; a manual run on the final candidate produces the release
scorecard. Manual diagnostics may select one operating system and one shard
(for example `cli-updates`: discovery/cache/artifact tests, project
preparation and legacy upgrade fixtures); they skip unrelated jobs, produce no
scorecard and cannot replace the full `all`/`all` matrix that release
admission requires. Strict Clippy and format always run.

### Packaged distribution and generated-application jobs

The manual `all`/`all` matrix also packages the 20 release-inventory crates,
audits their contents and runs the release pipeline's archive-only consumer and
isolated CLI installation/blueprint checks. `Packaged distribution and
installed CLI` is a required exact-SHA release-admission job; it has no
registry credentials and never uploads to crates.io. The `platform=all`,
`shard=packaged-distribution` diagnostic runs only this check plus workspace
Clippy/format and cannot satisfy release admission.

`Generated VM deployment (docker)` and `(podman)` run
`.github/check-vm-deploy.py`: with the CLI installed from the packaged
archives, they run `cargo rullst deploy` for the Blank and SaaS starters,
follow the [VM guide](docs/src/vm-deployment.md) (`deploy:doctor`, the
generated `Dockerfile`, `compose.yaml` and `Caddyfile`, unmodified) and check
`/health` over HTTPS through Caddy, the HTTPS redirect, the unpublished app
port, the read-only root filesystem, capabilities and `no-new-privileges`,
Caddy's trusted-proxy address, `db:migrate`, a database-backed page (SaaS:
sign-up and sign-in with a Secure cookie), SQLite persistence across a
restart, the guide's backup command and the absence of panics. Podman runs
rootless with the unprivileged-port setting the guide documents. Full manual
runs, the nightly run and the `vm-deployment` diagnostic run both jobs; a
ready pull request runs them only when it changes
`cargo-rullst/src/generators/deploy.rs`, `generators/project/docker.rs`,
`blueprints/deploy.rs` or the check. They are not required release jobs, but a
failure fails the full manual run that admission requires. Real DNS, Let's
Encrypt, provider accounts, Arm images and reboots are not covered.

On Linux, the `cli-profiles-relational` shard runs
`.github/check-auth-starter-sql.py` against disposable PostgreSQL 15, MySQL 8.4
and MariaDB 11.4 servers: a Blank starter is migrated, given two rows, extended
with `cargo rullst auth` and migrated again. SQL checks the six added columns,
the unique `users_email_unique` index, the placeholder e-mails, the unchanged
starter schema and rows and the migration records; the application signs up an
account over HTTP (Argon2id hash); `db:rollback` must restore the starter
exactly and keep every row, and a second `db:migrate` must succeed. It runs on
`main` pushes, nightly, with the `full-matrix` label and in manual runs. The
API and SaaS starters and other server versions are not covered.

`First-hour documentation` runs `.github/check-first-hour.py`: in reading
order, it runs the blocks marked `<!-- first-hour: … -->` in the README
quickstart, the [getting-started page](docs/src/1-getting-started.md),
tutorials 01, 02, 03, 05 and 11 and the
[REST API quickstart](docs/src/tutorials/rest-api-quickstart.md), with the CLI
and crates from the packaged archives, and checks what each page says happens
(`.github/first_hour_checks.py`; grammar in `.github/first_hour_docs.py`). Its
triggers follow the deployment jobs (pull requests only when they change a
marked page, `cargo-rullst/src/` or the check); it is not a required release
job. `.github/test-first-hour.py` validates the markers offline on every push.

`Generated starters in browsers` runs `.github/check-starter-browsers.py`:
with the CLI built from the checkout, it generates the Blank and SaaS starters
(framework crates resolved from the checkout by
`.github/dast-generated-app.py`), builds them, applies their migrations and
starts each with its generated `.env` (development, offline billing).
`.github/starter-browsers/starter-browsers.mjs` then drives it in Chromium,
Firefox and WebKit with the Playwright and axe-core versions pinned (with
integrity hashes) by `.github/starter-browsers/package-lock.json`, installed
with `npm ci --ignore-scripts`; Playwright downloads the browser builds that
its version pins. The starter's own Content-Security-Policy stays in force
(the reports raised by the inline style Playwright injects for a WebKit
screenshot are counted apart in the summary).
Blocking: a console error, an uncaught page error, a failed same-origin
request or a CSP violation; an element that Tab does not reach, or whose
appearance focus does not change; focus lost after the Blank starter's HTMX
swap; a failed flow (the HTMX counter, and SaaS sign-up, sign-out and sign-in
through the real forms, each submitted with Enter, with the form token
matching the `rullst_csrf` cookie); an axe-core `serious` or `critical`
violation; a server panic. axe-core `moderate` and `minor` findings are listed
in the job summary. Pages: Blank `/`; SaaS `/pricing`, `/register`,
`/dashboard`, `/login` and `/password/forgot`. A defect in framework code is
versioned in `.github/starter-browsers/known-defects.json` with its owner and
reason: it is reported, not failed, and an entry that no longer occurs fails
the job until it is removed. Traces, screenshots and server logs are uploaded
when it fails. Its triggers follow the deployment jobs (pull requests only
when they change `cargo-rullst/src/blueprints/`, the project, auth or billing
generators, `rullst-macros/src/`, Core's security, server, config, `html` or
`htmx` sources, or the check); it is not a required release job. The API
starter (no HTML) is covered by the ZAP baseline instead. This is automated
evidence for the pages exercised, not WCAG conformance or screen-reader
testing.

### Native CLI artifacts and browser checks

Manual all-platform `all` and `cli-standard` selections call
`cli-artifacts.yml` for Linux x64, Windows x64, macOS ARM64 and macOS x64; both
executable entry points must run and report the candidate version before
files, digests and source/platform metadata are retained. These CI artifacts
have no release tag or installation authority, and exact full-candidate
admission requires all four jobs. The tag-only release calls the same builder,
attests the executables and manifests in build provenance and adds them as
release assets only after attestation and registry publication.

The Linux `workspace` shard sets `RULLST_UI_BROWSER_TESTS=1` (Node 24,
Chromium) and runs real Nexus and Portfolio-example HTML through
`.github/mobile-ui-browser-smoke.mjs`: drawer dismissal and focus, no-JS
behaviour, layouts at 320–1440px, long content and reduced motion, with CDN
resources blocked and the Nexus shell under the default production CSP.
Missing browser prerequisites fail the shard. This is not WebKit/Firefox,
device or WCAG evidence.

Scheduled events run from the default branch, so they follow `main` and are
not evidence for the frozen `v12` line. Tag publication has no manual button.

## Manual and periodic execution map

Every verification workflow except the PR-context-only `pr-security-evidence.yml` and `mutants-pr.yml`,
reusable `cli-artifacts.yml` and tag-only `release.yml` can be started from **Actions → select workflow
→ Run workflow**. A manual run checks the selected branch's current SHA; record
that SHA and the run URL before treating it as release evidence. The release
workflow intentionally has no button because its publication authority begins
only with an exact version tag.

Fuzz target jobs have one bounded exception to repeating expensive work on
an unchanged input surface: [verified evidence reuse](docs/src/fuzz-evidence.md).
The candidate still requires its own successful fuzz workflow; the admission
checker independently verifies every reused original job and source digest.
No other workflow gains cross-commit credit from this exception.

The workflows below are **started manually** for release evidence. `kani.yml`
and `miri.yml` also run weekly on `main` (next table), but release admission
counts only a dispatched run:

| Workflow | Evidence | RC interpretation |
| :--- | :--- | :--- |
| `dast-zap.yml` | OWASP ZAP baseline against a release blog showcase, a fresh generated REST API, a fresh generated SaaS starter (public pages, then its signed-in pages with a disposable account's session) and the LMS example (`examples/lms`, release-built from the workspace) | REST/SaaS/LMS warnings and failures block unless an exact rule ID is versioned as `INFO` with a local explanation in `.zap/`; those configs are passed explicitly to the pinned scanner and unlisted warnings remain live. The showcase is informational because it deliberately uses third-party presentation assets; reports and application logs are retained. This remains representative, not universal deployment coverage. |
| `fuzzing.yml` | All 42 declared libFuzzer targets from the validated shared inventory | **Required release evidence:** release mode validates all 42 targets and independently verifies reusable input-equivalent evidence; packages with remaining targets run locked compilation preflights and new 5.5-hour campaigns; target-specific corpora are restored and saved, while failure reproducers are retained. Dependency-lock drift fails preflight and campaign jobs. The proc-macro parser uses strict processes of at most 30 minutes sharing one corpus, which bounds sanitizer RSS without weakening the total budget. Original reused results must be at most seven days old, come from an ancestor on the same release line, and prove the complete budget. The final candidate still runs the evidence boundary and publication independently recomputes all 42 results. [Reuse policy](docs/src/fuzz-evidence.md). A strict five-minute single-target diagnostic accelerates correction feedback but is explicitly ineligible as release evidence. This is bounded evidence, not proof for every input. |
| `kani.yml` | Twenty named bounded formal harnesses in ten supported runtime/library packages | **Required release evidence for the declared harnesses:** every proof has an isolated strict matrix job. Rullst itself stays on stable Rust 1.99.1 with a Rust 1.96 MSRV; only the separately built Kani verifier uses its pinned `nightly-2026-08-01` compiler (`rustc 1.99.0-nightly`) because the latest stable Kani bundle's Rust 1.93 compiler cannot compile the framework. The proc-macro-only `rullst-macros` target remains unsupported by Kani and is covered by compile-pass/fail and generated-project evidence instead. |
| `miri.yml` | Randomized-layout Miri execution over 15 named pure-Rust/default-feature scopes | **Required release evidence for the declared scopes:** every selected scope is strict. This nightly-only interpreter uses pinned `nightly-2026-08-21` (`rustc 1.100.0-nightly`); it does not change the project's stable toolchain or MSRV. Native FFI, OS syscall, network/provider, umbrella re-export, and example-application boundaries are excluded explicitly rather than emitted as tolerated errors. Its informational cargo-careful jobs are not part of this evidence. |
| `mutants.yml` | A source-bound discovered inventory, eighty lossless shards with at most four running concurrently, their artifacts and a strict aggregate | Informational: a cheap all-feature `--list --json` preflight records the selected source's complete unique inventory before runners start; every shard then uses that release surface, and aggregation requires every reviewed candidate to receive exactly one classification before reporting the conservative caught percentage. A targeted mode retests one validated production Rust file after a correction, with explicit per-mutant test/build timeouts and an optional library-only test scope; it does not replace the complete campaign. Every shard has a pinned Redis service, so the `RULLST_TEST_REDIS_URL` contracts run for each mutant. A contract mode runs one owned-server wrapper's mutation mode, which mutates only that wrapper's PostgreSQL or Redis adapter files and runs only its contract tests against the server it owns; it checks its own outcome count instead of the workspace inventory and aggregate. Recovery modes accept only the repository-reviewed campaign policy, bind the source/run/branch/tool/inventory digest, bisect explicitly authorized failed fragments, reuse immutable successful artifacts and emit a content-addressed aggregate. Missed/time-out exit codes remain findings, while a broken baseline, incomplete artifact set, preflight/classification mismatch, invalid invocation or cargo-mutants internal failure fails the workflow. The v12 campaign completed 14,391/14,391 classifications in run `34761010296`; “pass” does not mean every mutant was killed. |

The v13 candidate also provides `verus.yml` as a manual-only pilot. Its source
linkage/proof/negative-control checks are strict when invoked, while the pilot
remains outside release admission. A new workflow must first be available in
GitHub's default-branch workflow registry before its manual button/dispatch can
be assumed available; prepared YAML is not hosted evidence.

These workflows are **periodic and manually runnable**:

| Cadence | Workflows | Mode |
| :--- | :--- | :--- |
| Daily | `ci.yml`, `sanitizers.yml`, `supply-chain.yml` | Supply Chain (Cargo Audit of the production and fuzz locks, Cargo Deny and the advisory-exception check) is blocking; the nightly `ci.yml` run executes the complete Linux/macOS/Windows matrix on `main`; TSan/ASan are blocking when executed. |
| Weekly | `bench.yml`, `codeql.yml`, `coverage.yml`, `crypto-containment.yml`, `documentation.yml`, `kani.yml`, `miri.yml`, `proptest.yml`, `scorecards.yml`, `trufflehog.yml`, `udeps.yml` | The inventory below identifies which results are blocking, automated evidence, or informational. Miri (Wednesday 01:17 UTC) and Kani (Thursday 01:47 UTC) run their full strict matrices so that a regression shows up within a week rather than at release admission. The Miri run also runs the informational `cargo careful test` jobs. |

All remaining workflows run on their push, pull-request or path filters and
expose a manual rerun. For an RC or stable checkpoint, use the automatic
mainline suite, then the exact-SHA manual admission set: full-platform
`ci.yml`, release-mode `fuzzing.yml`, `dast-zap.yml`, `kani.yml`, `miri.yml`,
`sanitizers.yml` and `proptest.yml`. The tag workflow checks those receipts
through `.github/release-required-workflows.json`; the fuzz receipt counts
only when all 42 target jobs and its evidence boundary succeeded. Admission
accepts only `workflow_dispatch` runs: a scheduled run is never a receipt,
and its own concurrency group (`-nightly` or `-weekly`) keeps it from
cancelling the dispatched run. Physical devices, store approval, live
provider accounts, external security review and human release approval
remain outside GitHub Actions.

## Required local and release baseline

The contributor baseline is the `AGENTS.md` trifecta (`cargo test`, `cargo
clippy` and `cargo fmt` with `--workspace --all-features`). The main CI uses
the stricter all-target Clippy form and checks formatting without modifying
files:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Rust CI disables incremental compilation and keeps content-addressed compiler
outputs in the GitHub Actions cache with the pinned `sccache` Action and
binary; the `cc` build dependency honours the same wrapper, so bundled DuckDB
C++ objects are reused. Same-repository pull requests write only their
isolated `refs/pull/.../merge` scope, fork pull requests are read-only, pushes
and manual runs on `main` populate entries for later pull requests, and the
tag-only verifier reads the cache strictly read-only. The setup action's Cargo
archive is disabled in these jobs: archiving mutable nested CLI targets once
produced false missing-directory annotations and about 9.56 GiB of duplicates
at GitHub's 10 GiB repository limit. The same cache serves coverage,
benchmarks (run sequentially on one runner), mutation and fuzz compilation and
the release-mode suite. A cache never substitutes for a test result; judge
acceleration by the hit ratio of a later compatible run, never by weakening
assertions. SemVer checks fan out from the machine-readable release order, one
published API per job.

The `Public API snapshot` job of `semver.yml` runs `.github/public-api.py
--check` (tested by `.github/test-public-api.py`). On the pinned
`nightly-2026-08-21` it builds the rustdoc JSON of each library package in the
release order, except the `cargo-rullst` CLI, with its
`[package.metadata.docs.rs]` features, renders every documented item into
`.github/public-api/<package>.txt` (`#[doc(hidden)]` items and blanket
implementations are not listed) and fails when the result differs from the
committed file, so an API change is reviewed as a diff. It runs on ready pull
requests to `main`, pushes and manual runs (`public_api_only` skips the
registry comparison). It is not a required status check, but a failure fails
the `semver.yml` push run that release admission requires.

CodeQL runs separate Rust and JavaScript/TypeScript jobs. The Rust job
compiles manually without the target cache, so the extractor observes the
exact SHA; the JavaScript job is buildless (`/language:javascript`).

`ci.yml` also checks each ORM strict database feature (PostgreSQL, MySQL,
SQLite) in isolation, the runtime-only Core and all 70 public umbrella
features in isolated additive graphs with manifest-drift detection, and the
portable database matrix on Linux. Feature-boundary rows and threat-model
negative tests each fan out into four deterministic strict shards. Rows with
no feature selected compile every package target; feature-selected rows
compile the isolated library graph. The umbrella's `cfg(doctest)` aggregation
reads all 48 public tutorial files, compiling or executing complete examples
and recording contextual fragments as ignored. Further contracts: the pinned
live Redis job (Core scheduling and cache inspection, ORM cache, invalidation,
rollback, post-commit observers and Scout ordering); the SQLite outbox
contract on all three systems, with the lifecycle repeated against
PostgreSQL, MySQL, MariaDB and strict SQLite; the Rust 1.96.0 MSRV job; and the
Linux provider matrix (Scout against a digest-pinned Meilisearch image,
pgvector L2 and cosine queries, Nexus's default Any/SQLite HTTP contract;
Algolia and Elasticsearch use local protocol fixtures).

The **ORM differential databases** job runs the same ORM scenarios
(`rullst-orm/tests/differential/`) through the public `#[derive(Orm)]`,
query-builder, `Schema`, `RowSnapshot` and `Orm::transaction` API, with each
backend's strict driver feature, on SQLite and on PostgreSQL 15.17, MySQL
8.4.13 and MariaDB 11.4.13 service containers (the digest-pinned images of the
auth-starter check). Each backend writes a normalized transcript (values, row
counts, ordering where the query specifies one, error kinds, transaction
outcomes); SQLite runs twice and must repeat itself exactly, and each server
transcript is compared with SQLite's. A difference passes only when
`expected.rs` documents it with a reason and the exact value, and a documented
difference that stops occurring fails too. The list records provider
semantics (PostgreSQL's constraint names, NULL ordering, case-sensitive
`LIKE`, NUL-free text and transaction abort after a failed raw statement;
MySQL/MariaDB's case- and accent-insensitive collations, MariaDB's trailing
spaces and MySQL 8's reserved `RANK`; non-transactional sequences;
microsecond timestamps; SQLite's unenforced `VARCHAR(n)`) and pins open ORM
defects, marked `Defect`, until they are fixed. `RULLST_DIFF_REQUIRE_ALL=1`
turns a missing server URL into a failure; the transcripts and comparison
reports are uploaded. It runs on pushes, nightly, in `all`/`workspace` manual
runs and on pull requests that change the ORM, and it is a release-admission
job (`.github/release-required-workflows.json`).

The quality scorecard combines versioned expert-audit ceilings with the actual
gate results of its run and keeps a 90-day artifact: a failed, skipped or
cancelled gate removes the dimensions it was meant to prove, and a green gate
cannot raise a crate above its ceiling. It is engineering-evidence reporting,
not certification ([methodology](docs/src/quality-scorecard.md)). The tag-only
packaged-distribution gate compiles a crates-only consumer of the extracted
`rullst` package with defaults disabled and every public feature enabled.

## Recommended branch-protection profiles

**`main` (v13 development)** requires the fast gate of 20 checks that every
ready pull request emits: Code Quality & Format; Run Tests (ubuntu-latest / workspace) and
(ubuntu-latest / cli-standard); Check MSRV (Rust 1.96.0); ORM strict-sqlite,
strict-postgres and strict-mysql; ORM differential databases; Redis live contracts; Generated release access
boundaries; Facade shared-local recovery composition; Versioned deterministic AI
evals; Build book and validate local links; Validate workflow syntax and
expressions; Enforce Zero Panics (published runtime targets); Enforce production
unsafe boundary; Check Dependencies (Licenses, Vulnerabilities); Scan for leaked
secrets; Enforce reviewed internal dependency graph; and Typo Check. It does not
require an up-to-date base: the full `main` push run and the nightly matrix
detect interactions between independently green pull requests, and each
additional rebase would otherwise repeat the whole gate. Cargo Deny's job in
`supply-chain.yml` is the required vulnerability gate; its other two jobs
have distinct names, so either could be required without a collision. The
macOS/Windows and CLI-profile shards, public feature boundaries, threat-model
minimum, coverage, SemVer and CodeQL run after merge, nightly or weekly. They
remain release-admission requirements.

**`v12` (frozen; end of life on 16 October 2026)** keeps the complete profile
of 46 checks with an up-to-date base: every Linux/macOS/Windows test shard,
public feature boundaries, the threat-model minimum, CodeQL, documentation and
workflow lint.

Do not configure a path-filtered, scheduled, manual, deployment or tag-only
workflow as a universal required check: an intentionally skipped workflow may
never create the check context (IoT and Vault Crypto Containment, for example,
blocks only when relevant paths change). Pages, benchmarks, fuzzing,
sanitizers, Kani, Miri, mutation testing, udeps, ZAP, Scorecard and release
provenance belong to deeper evidence or release policy. GitHub repository
rulesets remain the enforcement source; on 29 October 2026 they matched this
profile: the
[development-branch ruleset](https://github.com/Rullst/Rullst/rules/24175738)
on `main` and the
[maintained-branch ruleset](https://github.com/Rullst/Rullst/rules/24080901)
on `v12` (see [GOVERNANCE.md](GOVERNANCE.md#change-and-release-controls)).

### Observed v13 protection

Historical: on 20 September 2026 UTC the hosted `v13` protection was read
back with 43 required contexts (25 Linux runtime jobs, sixteen macOS/Windows
shards, documentation and workflow lint), a current base, required pull
requests with resolved conversations, administrator enforcement and no force
pushes. `v13` was later retired into `main`; the release policy now requires
25 workflow receipts (`.github/release-required-workflows.json`).

## Release-engineering status

| Release-engineering goal | Current status | Assessment |
| :--- | :--- | :--- |
| Trifecta with all features | **Implemented** | CI and tag release both run format, all-target/all-feature Clippy, and all-feature tests. |
| Strict DB features in isolation | **Implemented in workflow** | `strict-postgres`, `strict-mysql`, and `strict-sqlite` compile independently and each runs a backend-specific CRUD test with only the selected strict feature enabled. |
| Honest blocking/informational labels | **Implemented** | Unsafe and Wasm checks are blocking in continuous CI; the declared Kani, Miri and fuzzing scopes are strict release gates; mutation testing and udeps explicitly remain informational. |
| Cover every fuzz target | **Implemented in workflow** | `.github/fuzz-targets.json` is the inventory for the manual campaign and the workflow-lint and Rust CI lock checks. A blocking validator compares it with all eleven fuzz manifests and their 42 source files. This records configuration, not a successful six-hour run. |
| Package all crates before publishing | **Implemented in workflow** | The tag-only release validates versions, packages all publishable workspace crates, hashes and attests the archives, then publishes in dependency order. |
| Unified evidence bundle per tag | **Implemented in workflow** | The tag-scoped bundle contains `Cargo.lock`, Cargo metadata, CycloneDX 1.5 SBOM, Cargo Audit JSON, `deny.toml`, bounded compliance evidence, governed advisory exceptions, commit/tag context, and checksums. The bundle and `.crate` archives are included in build-provenance attestation. |
| Align manifest, changelog, tag, registry, and notes | **Implemented in workflow** | Preflight validates manifests and tag, extracts the exact dated changelog section into the GitHub release, and verifies registry checksums after publication. |

## Important evidence boundaries

### Zero-panics and unsafe Rust

`zero-panics.yml` denies Clippy's unwrap, expect, panic, todo and
unimplemented lints for published runtime libraries, procedural-macro engines,
CLI production targets, generated runtime templates and the Wasm Core path;
tests are excluded.

`unsafe-policy.yml` compiles production libraries and binaries with
`-Dunsafe-code`. The reviewed source allowlist holds the Radar OS probe, the
dynamic-library loader, the CLI's Windows cache/installation owner/DACL
boundary, Windows project access-policy preservation and macOS extended-ACL
inspection, and the workflow fails if it changes. Each unsafe call documents
pointer and handle ownership and lifetime: Windows descriptors are installed
atomically through owned handles, and ACE pointers and SID lengths are checked
before use; the macOS module only inspects an ACL. This is an enforced
boundary, not a claim that dependencies contain no unsafe Rust.

### Coverage

`coverage.yml` runs LLVM coverage over the all-feature and default workspace
profiles plus the live database matrix and uploads LCOV to Codecov with GitHub
OIDC. The all-feature pass uses pinned cargo-nextest only to schedule the same
tests concurrently (zero retries, four slots, at most two nested
generated-application builds, a 30-day JUnit record); Rust CI still runs
ordinary `cargo test`. The default-profile pass covers ORM, Studio, Nexus and
the facade, whose SQLite contracts the all-feature profile excludes. The
workflow rejects a summary below 90% for the repository or the framework
libraries before upload, `codecov.yml` requires 90% with zero tolerance for
those views and changed lines, and a failed upload fails the run. Examples,
benchmarks and test support are filtered; CLI and proc-macro code stays in the
blocking aggregate. The README shows the overall and `framework_libraries`
badges.

### Formal, dynamic, and stress analysis

- **Kani and Miri** are bounded evidence for the harnesses and scopes that
  execute. Both run weekly on `main`; release admission still requires a
  dispatched run on the exact candidate. Rullst stays on stable Rust 1.99.1
  with a Rust 1.96 MSRV. Kani builds reviewed upstream revision
  `8fcd6d90ed07b559e553ca8a92b95f2db69b2c78` with its own pinned
  `nightly-2026-08-01`, which predates a `compare_bytes` compiler crash seen
  with Kani's first `rustc 1.100.0-nightly` snapshot. Its twenty isolated
  harness jobs across ten packages prove pure decisions such as Vault key-ID
  policy, DLP buffer admission, ASCII-folded RASP matching, SRI asset limits,
  bounded Login Guard delay and CoAP option classification; they do not verify
  `zeroize`'s inline assembly, cryptographic implementations or whole
  concurrent middleware, and the proc-macro-only `rullst-macros` is
  unsupported. Miri pins `nightly-2026-08-21` and strictly runs 15 named
  pure-Rust/default-feature scopes; native `ring`, AWS-LC, SQLite, syscall and
  network paths, the facade and the Blog example rely on native CI,
  integration tests and sanitizers instead.
- **cargo-careful** (`miri.yml`, weekly and on dispatch, informational):
  `cargo careful test` 0.4.10 on the same pinned `nightly-2026-08-21` runs the
  library unit tests of `rullst-core`, `rullst-security` and `rullst-orm`
  against a standard library built with debug assertions and cargo-careful's
  extra UB checks, natively (so beyond Miri's interpreter boundary), with a
  45-minute budget per crate. Integration tests that start servers or need
  services are excluded; the proc-macro crates are host-only and cannot load
  the careful sysroot. Job-level `continue-on-error` keeps its result out of
  the workflow conclusion, so it never changes the Miri admission receipt.
- **Mutation testing** (`mutants.yml`, manual, informational): a
  `--list --json` preflight fixes the selected source's exact inventory, then
  up to 80 lossless shards (four at a time) run with `--all-features`
  (`.cargo/mutants.toml` applies the same policy locally) and a pinned Redis
  service for the `RULLST_TEST_REDIS_URL` contracts. The aggregate must
  classify every candidate exactly once and never counts a timeout as caught;
  a broken baseline, an internal error, a missing artifact or inventory drift
  fails the run. Targeted mode retests one production `.rs` file with a
  per-mutant test timeout (`timeout_seconds`, default 300 s, which also bounds
  the baseline), a 300 s build timeout, an 8 GiB address-space limit per test
  binary and an optional `shard=k/n` slice (n at most 16); `test_scope=lib`
  runs only library unit tests. The ORM query-cache and hash-scope Redis
  contracts are compiled out of the all-feature build;
  [contract mode](#owned-server-contract-mutation-runs) covers adapters that
  only a wrapper-owned server reaches. Full mode can measure an ancestor
  `source_sha`, and recovery follows a committed policy bound to the runs,
  source, tool version, shard set and inventory digest.
- **Fuzzing** is described under [Fuzzing and OSS-Fuzz](#fuzzing-and-oss-fuzz).
- **Branch coverage, `cargo-udeps`, TSan and ASan** share the pinned
  `nightly-2026-08-21`. Branch coverage and udeps (weekly/manual) are
  informational. TSan and ASan run daily/manual across twelve runtime/domain
  packages and block when they run; Messaging runs its integration contract
  so its concurrent state is exercised. There is no MSan job.
- **ZAP** (manual) scans a fresh REST API and SaaS starter generated by the
  real CLI (framework crates resolved from the checkout by
  `.github/dast-generated-app.py`) and the LMS example. The SaaS starter runs
  in production with placeholder Stripe settings; one baseline crawls its
  public pages, and a second uses the session of an account registered for the
  run (`ZAP_AUTH_HEADER`) to crawl the signed-in pages and submit forms with a
  valid CSRF token, with only POST sign-out and billing out of scope
  (`.zap/saas-starter-account.tsv`). These baselines fail on any warning and
  ignore no rule. The blog showcase is informational because of its relaxed
  CSP and third-party assets. The targets are representative, not every
  starter, role (Nexus administrators are not scanned), browser or deployment.
- **Property tests and benchmarks** are scheduled/manual. `proptest.yml` runs
  the all-feature release-mode suite in seven shards plus the ORM and Connect
  property contracts with 10,000 cases. Eight benchmark groups (nine Criterion
  binaries) raise non-blocking alerts at a 20% regression and feed the
  [public benchmark hub](https://rullst.github.io/Rullst/benches/).

### Pull-request mutation gate

`mutants-pr.yml` keeps the campaign's result from decaying between
campaigns: on every pull request to `main` that changes Rust, it mutates only
the changed lines and fails when a mutant survives. Resolve a red result
before merging, with a test that fails with the mutant or, when the mutant is
truly equivalent, with an entry in the
[accepted equivalents](#accepted-equivalent-mutants). Branch protection does not
require the check yet. `.github/mutants-pr-exclude.txt` lists the files it
skips: the workspace example applications, test-only modules and adapters
exercised only by wrappers that own a disposable PostgreSQL, MySQL or
authenticated Redis server. Those files remain the manual campaign's
responsibility, through its
[contract mode](#owned-server-contract-mutation-runs).

The CLI's integration suites build generated projects, too slowly to repeat
per mutant, so `.github/mutants-pr.sh` runs two passes: the workspace pass
tests each mutant with its package's whole suite and leaves `cargo-rullst`
out; the CLI pass mutates only the files in `.github/mutants-pr-cli.txt` and
runs the CLI's library unit tests (`cargo test --lib`). A CLI file joins that
list once a targeted `test_scope=lib` run leaves only accepted equivalents.

### Accepted equivalent mutants

The [mutation testing record](docs/src/mutation-testing.md) lists each
equivalent mutant with its justification and the status of every campaign.
The gate skips an entry only when its exact cargo-mutants name is also in
`.github/mutants-pr-equivalents.txt`.

### Owned-server contract mutation runs

Some adapters are reached only by ignored contract tests that a wrapper runs
against a disposable Docker server it owns, so a full or targeted run reports
their database-bound mutants as missed. `mode=contract` runs
`.github/check-<contract>.py --mutants REGEX [--shard k/n]` on a hosted runner
and uploads `mutants.out` as `mutants-out-contract`: the wrapper starts its
server, mutates only its adapter files and runs only the exercise phase of its
contract tests, one at a time. `.github/mutation_contract.py`, the Cargo
target runner, resets the server before every test binary (a recreated
database, no leftover roles, `ALTER SYSTEM RESET ALL`, or `FLUSHALL` for
Redis), because a mutant can leave a role, a disabled `fsync` or a full Redis
behind. Missed and timed-out mutants are findings; a failed or incomplete
baseline, a failed reset or an incomplete classification fails the run. A
mutant that a unit test of the same file catches can still be reported as
missed here; compare with a targeted run. One build directory serves every
mutant, compiled incrementally without sccache.

| `contract` | Mutated files | Contract tests |
| --- | --- | --- |
| `passkey-postgres` | `rullst-auth/src/auth/passkey/shared/manager.rs`, `shared/postgres/operations.rs` | `postgres_contract` |
| `auth-recovery-postgres` | `rullst-auth/src/recovery/connection.rs`, `store.rs`, `email_login/guard.rs`, `email_login/storage.rs`, `api_tokens/storage.rs`: the files with PostgreSQL-only branches | recovery, sessions, email login, API token (with HTTP) and the exercise phase of the three restart journeys; not the browser journey |
| `privacy-postgres` | `rullst-privacy/src/age_assurance/postgres.rs`, `consent/postgres.rs` | `shared_replay_contract`, `shared_consent_contract` |
| `mail-postgres` | `rullst-mail/src/suppression/postgres/**` | suppression contract and restart exercise |
| `recurring-postgres` | `rullst-messaging/src/schedules/**` (`schedules-postgres`) | recurring contract and restart exercise |
| `messaging-redis` | `rullst-messaging/src/redis_streams/**` | the password, TLS, fault, ordering, outbox and bounds contracts and the AOF restart exercise |
| `connect-sql` | `rullst-connect/src/challenge/sql/postgres.rs`, `sql/mysql.rs` (the wrapper also owns a MySQL server, which each contract resets by dropping its table) | `postgres_challenge_contract`, `mysql_challenge_contract` |
| `orm-sqlite` | `rullst-orm/src/**` and `rullst-orm-macros/src/**`, test files excluded; select files with `mutants_regex` (for example `^rullst-orm/src/pool/savepoint\.rs:`) | every `rullst-orm` test target, built with `drivers-all` (the SQLx `Any` pool) |
| `orm-polyglot` | the same files; select the optional-backend modules (`polyglot`, `scout`, `redis_data`, `query_cache`) with `mutants_regex` | every `rullst-orm` test target, built with `drivers-all` and the Redis, MongoDB, SurrealDB, Turso, Scout HTTP and Qdrant features, with an owned Redis server for the `RULLST_TEST_REDIS_URL` contracts |
| `orm-sql-servers` | the same files; select the PostgreSQL and MySQL/MariaDB branches with `mutants_regex` | `matrix_postgres`, `matrix_mysql` and `matrix_mariadb`, built with `drivers-all`; their testcontainers start a disposable server per test binary, and a server that cannot start fails the run |
| `orm-strict-postgres` | the same files | `matrix_postgres`, built with `strict-postgres` only, which also exercises the native enum types and typed binds of that pool |
| `orm-strict-mysql` | the same files | `matrix_mysql`, built with `strict-mysql` only |
| `orm-polyglot-servers` | the same files; select the optional backends' live paths with `mutants_regex` | the MongoDB, SurrealDB, document-recovery, Qdrant, Meilisearch, Redis data and Turso testcontainers matrices, built like `orm-polyglot` |

`orm-sqlite` owns no server. The all-feature build selects the strict
PostgreSQL pool and compiles the SQLite behaviour tests of `rullst-orm` out;
this contract builds the ORM with `drivers-all` and runs its whole suite on
SQLite for every mutant, including the code `#[derive(Orm)]` generates.
PostgreSQL-only branches stay with targeted runs. `orm-polyglot` adds the
optional backends (mocks and loopback servers) and runs the query-cache and
Redis hash-scope contracts against its own Redis; DuckDB stays with targeted
runs. `orm-sql-servers` measures dialect branches (placeholders, migration
locks, collations, payload types) against three real servers, so select only
those branches. `orm-strict-postgres` and `orm-strict-mysql` cover code that
the `Any` pool does not compile.

### Fuzzing and OSS-Fuzz

The manual `fuzzing.yml` matrix covers all **42** declared libFuzzer targets
(the frozen v12 line keeps 40): Core 12, ORM 5, Security 7, Connect 3, Mail 4,
AI 3, IoT 3, Capital 1, Nexus 1, Studio 1 and Privacy 2.
`.github/fuzz-targets.json` is validated against every `*/fuzz/Cargo.toml`,
its checked-in lockfile and source files before the campaign fans out, on the
verifier-only `nightly-2026-08-21`. Release mode verifies input-equivalent
earlier results ([reuse policy](docs/src/fuzz-evidence.md)); eleven locked
preflight jobs then compile the remaining targets before any 5.5-hour
campaign, and lock drift fails preflight and campaign jobs. The parser
campaign restarts its ASan process every 30 minutes with one corpus and the
full budget. Campaign jobs restore and save per-target corpora and keep
failure reproducers; reuse adds a verified relationship to the candidate
inputs and never claims a fresh execution. The five-minute single-target
diagnostic compiles only that target and is never release evidence.

The weekly `corpus-sync.yml` warm-up was removed in 13.0.0-alpha.4: the
repository cache runs above its 10 GiB limit, so no saved corpus survived to
the next run (the weekly runs of 11, 18 and 25 October 2026 and the release
campaign of 24 October all reported "Cache not found"), and its one-minute
runs never produced a reproducer. Campaign corpora face the same eviction.
The `oss-fuzz/projects/rullst` directory is a local draft, not proof of
upstream acceptance, ClusterFuzz execution or coverage of all 42 targets;
finishing the submission is on the
[verification roadmap](ROADMAP.md#verification-roadmap).

### Supply chain and release provenance

All direct third-party GitHub Actions references are pinned to full commit
SHAs, and Actionlint 1.7.7 in `workflow-lint.yml` runs from an immutable
container digest. A pinned composite action can still carry its own transitive
downloads, so blocking integrations are reviewed for that behaviour. RustSec
exceptions are limited by `deny.toml` and documented with owners, controls and
expiry dates in `docs/src/security-advisory-exceptions.md`. `scorecards.yml`
publishes OIDC-authenticated OpenSSF Scorecard results for the README badge;
the score is supply-chain evidence, not a security certification.

`release.yml` is tag-only. It verifies source, validates the exact semantic tag
against every publishable crate, packages before the first publish, and creates
a tag-bound evidence bundle containing the lockfile, Cargo metadata, CycloneDX
1.5, Cargo Audit JSON, dependency policy, bounded compliance evidence, advisory
exceptions, commit context, checksums, and release notes rendered from the
exact matching changelog section: the section's introduction and "Known
limitations", plus a link to the full section at the tag (the whole section
when it has no introduction). The `.crate` archives and evidence receive
GitHub's SHA-pinned build-provenance attestation. This does **not** by itself
establish a project-wide SLSA level, Sigstore Cosign binary signing, independent
review, or regulatory compliance.

`architecture.yml` is repository-owned and deterministic: it rejects any
internal dependency edge or optionality change not reflected in the reviewed
`crate-architecture-policy.json`. It replaced TangleGuard, whose action
downloaded an unversioned `latest` binary without a pinned checksum.

Keep CodeQL `init`, `analyze` and `upload-sarif` on one reviewed Action
revision; Dependabot groups these sub-actions, and the local pin validator
rejects mixed CodeQL revisions inside a workflow. Review the scanner bundle as
well as the wrapper SHA. `setup-rust-toolchain` 2.0.0 moved warning
enforcement to `CARGO_BUILD_WARNINGS`; the workflows keep
`rustflags: "-D warnings"` and `build-warnings: ""`, preserving the previous
compiler flags and compiler-cache inputs, and Clippy keeps its explicit
`-D warnings`. TruffleHog's composite Action pin does not pin its default
`latest` scanner image, so the workflow also pins the reviewed 3.97.4 image
digest; a scanner update must change that digest.

## Workflow inventory (34 definitions)

Durations are intentionally omitted because runner load, cache state, and the
dependency graph make static estimates unreliable.

| Workflow | Trigger | Mode | Actual scope |
| :--- | :--- | :--- | :--- |
| [`architecture.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/architecture.yml) | main/v12/v13 push and PR, manual | Blocking | Compares Cargo's publishable non-dev internal dependency graph with the reviewed `crate-architecture-policy.json`; unreviewed normal/build edges, removals, or optionality changes fail, while test-only dev-dependencies do not masquerade as production coupling. |
| [`bench.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/bench.yml) | main push, weekly, manual | Automated evidence | Eight published groups backed by nine Criterion binaries, with non-blocking 20% regression alerts and gh-pages data consumed by the benchmark hub. Scheduled runs use the repository default branch. |
| [`ci.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) | main/v12/v13 push and PR (Linux gate on PRs to main; full OS matrix with the `full-matrix` label), nightly full matrix, manual | Blocking plus observational report | Format, all-target/all-feature Clippy, seven-shard multi-OS tests including Cargo-aware doctests sourced from all 48 tutorials, four-way feature/threat partitions, the SQLite transactional outbox contract and Messaging concurrency suite, relational/polyglot live matrices, the ORM differential comparison of SQLite, PostgreSQL, MySQL and MariaDB, isolated strict-DB/feature boundaries, MSRV, the generated VM deployment built and started with Docker and rootless Podman (manual, nightly and deploy-changing pull requests), the first-hour documentation path run as written (manual, nightly and pull requests changing a marked page or the CLI), the generated Blank and SaaS starters driven in Chromium, Firefox and WebKit with keyboard, CSP and axe-core checks (manual, nightly and pull requests changing the starters, `html!` or Core's pages, headers or CSRF), and a ready-PR/manual full-matrix SHA-bound per-crate quality scorecard artifact. A targeted manual OS/shard run is diagnostic and cannot emit the full scorecard. |
| [`cli-artifacts.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/cli-artifacts.yml) | reusable call from full/CLI manual Rust CI or admitted tag release | Blocking caller job; required release evidence | Builds and runs both CLI entry points on four explicit native targets; stages bounded executables and source/version/platform/digest inventories before attestation. CI artifacts are diagnostic; only the tag pipeline adds separate provenance and release assets. No installation occurs. |
| [`codeql.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml) | main/v12/v13 push, v12/v13 PR, weekly, manual | Blocking run | Rust CodeQL after an all-target/all-feature workspace check, plus JavaScript/TypeScript analysis. |
| [`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml) | main/v12/v13 push, v12/v13 PR, weekly, manual | Blocking plus observational job | LLVM LCOV generation with a pinned, zero-retry, bounded-concurrency nextest scheduler and retained JUnit inventory; a focused default-SQLite pass for ORM/Studio/Nexus/the facade; exact local 90% floors; and blocking OIDC-authenticated Codecov upload. Scheduled/manual branch instrumentation is non-blocking and uses the pinned verifier-only nightly. |
| [`crypto-containment.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/crypto-containment.yml) | relevant main changes and PRs, weekly, manual | Blocking | Signed OTA and Vault tests, RustSec audit, and simulator-boundary checks; explicitly no PQC/HSM certification. Until 13.0.0-alpha.4 this was `pqc-compliance.yml`; the `pqc` and `hsm` modules it scans are deterministic simulator fixtures. |
| [`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml) | manual | Blocking generated targets plus informational showcase | Pins the ZAP image by digest, scans fresh release/migrated REST API, SaaS starter (public and signed-in) and LMS surfaces as blocking gates, scans the CDN-backed blog showcase informationally, and uploads separate reports plus application logs. |
| [`documentation.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/documentation.yml) | main/v12/v13 push and PR, weekly, manual | Blocking plus informational external scan | Builds the mdBook; validates landing/benchmark templates, project identity, the README workflow count, the inventory below and every workflow badge or link, local assets, pinned external chart scripts and all requested social links. Real Chromium checks desktop/390px/320px layout, keyboard/mobile navigation, clipboard success/denial, privacy disclosure, reduced motion, no-JS navigation, and absence of external landing requests/browser storage. This is a bounded browser contract, not WCAG certification. Also validates the 190-claim historical roadmap denominator and repository-local links. Scheduled/manual runs preserve an informational external-link report. |
| [`e2e-smoke.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/e2e-smoke.yml) | main/v12/v13 push and PR, manual | Blocking | Boots the release Blog application and checks HTTP, headers, CSRF form flow, SQLite persistence, and the persisted page parsed by real headless Chromium. |
| [`fuzzing.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/fuzzing.yml) | manual | Blocking release campaign or diagnostic | Release mode maintains all 42 targets through independently verified, at-most-seven-day input-equivalent evidence or fresh 5.5-hour jobs. A package's inputs are the crates it reaches through path dependencies, files they include from elsewhere and a few global files; a change reruns the packages whose inputs it touches (all targets for global inputs, none for docs, changelog, CLI or examples), and a package whose inputs cannot be bounded falls back to the whole repository. Selected packages retain locked preflights, corpus caching and failure reproducers. `force_full` always requests a new full campaign; the current-commit boundary and independent tag-admission check remain mandatory. Lock drift is always rejected. The parser restarts its ASan process at most every 30 minutes while preserving the budget. Single-target diagnostic mode runs for five minutes and the evidence boundary marks it ineligible for release. |
| [`iot-integration.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/iot-integration.yml) | main/v12/v13 push and PR, manual | Blocking | Host IoT tests, signed OTA invariants, and one Cortex-M no-std build; no hardware claim. |
| [`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml) | weekly (Thursday 01:47 UTC), manual | Blocking release scope; bounded evidence | Builds an immutable reviewed Kani snapshot with the verifier-only `nightly-2026-08-01` compiler, while Rullst stays on stable Rust 1.99.1 with a Rust 1.96 MSRV. It verifies twenty named bounded harnesses in isolated jobs across ten supported packages. Proof failures fail their matrix jobs; the proc-macro-only crate remains outside Kani's supported targets. |
| [`machete.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/machete.yml) | main/v12/v13 push and PR, manual | Blocking | Unused dependency scan with configured exceptions. |
| [`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml) | weekly (Wednesday 01:17 UTC), manual | Blocking release scope; bounded evidence | Pinned nightly-only Miri executes 15 named pure-Rust/default-feature scopes with randomized layouts without changing Rullst's stable toolchain or MSRV. Native FFI/syscall/network paths, the umbrella re-export facade, and the Blog example are explicit boundaries; selected-scope failures fail the workflow. Informational `cargo careful test` jobs (Core, Security, ORM library unit tests) run alongside with `continue-on-error` and never change the conclusion. |
| [`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml) | manual | Informational | A source-bound inventory preflight followed by eighty lossless pinned cargo-mutants 27.1.0 shards with four concurrent jobs over that source's all-feature workspace, each shard with a pinned Redis service for the `RULLST_TEST_REDIS_URL` contracts, one validated production-file diagnostic, one owned-server contract (`mode=contract`: a PostgreSQL or Redis wrapper's adapter mutants against the server it owns, or the ORM crates against the `rullst-orm` SQLite suite), or policy-bound exact-SHA recovery of reviewed failed fragments. Successful immutable artifacts may be reused, but the content-addressed aggregate still requires every reviewed candidate exactly once and carries source/run/branch/tool/inventory provenance. Findings stay informational, while baseline/tool/invocation failures, missing artifacts, incomplete classification and reviewed-inventory drift fail the run. The completed v12 receipt is run `34761010296`: 14,391/14,391 classified and a conservative 70.57% caught. |
| [`mutants-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants-pr.yml) | pull requests to main that change Rust | Blocking by policy; not a required check | Lists the pinned cargo-mutants 27.1.0 all-feature mutants on the Rust lines the pull request changes, skipping the globs in `.github/mutants-pr-exclude.txt` and every CLI file not listed in `.github/mutants-pr-cli.txt` (listed CLI files run against the CLI's library tests), and tests them in at most eight shards with a Redis service and the targeted-run timeouts and memory cap. A missed mutant fails the run; unviable and timed-out mutants do not. See [Pull-request mutation gate](#pull-request-mutation-gate). |
| [`no_std-build.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/no_std-build.yml) | main/v12/v13 push and PR, manual | Blocking | Builds `rullst-iot` for three bare-metal targets; this is compile evidence, not hardware execution. |
| [`pages.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pages.yml) | main push, manual | Deploy | Validates and deploys the current development landing page, local visual assets, mdBook and benchmark hub/dashboards to GitHub Pages while preserving history data fetched from `gh-pages`. |
| [`pr-security-evidence.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pr-security-evidence.yml) | pull requests | Automated evidence | Generates bounded CLI audit, compliance report, and CycloneDX SBOM artifacts; no certification claim. Until 13.0.0-alpha.4 this was `ai-sentinel-pr.yml`; it calls no AI service. |
| [`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml) | weekly, manual | Blocking run | Seven parallel release-mode workspace shards plus dedicated ORM and Connect property contracts with configured case counts. |
| [`release.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml) | exact-looking version tags | Release | Tag/changelog validation, exact-SHA admission of the full CI matrix (not repeated in the tag run), package-all, evidence bundle, checksums, GitHub build-provenance attestation, changelog-derived release notes, and dependency-order publication. |
| [`sanitizers.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml) | daily, manual | Blocking run | TSan and ASan library matrices on pinned `nightly-2026-08-21`; this verifier toolchain does not change Rullst's stable compiler or MSRV. |
| [`scorecards.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/scorecards.yml) | main push, weekly, manual | Automated evidence | OpenSSF Scorecard analysis and SARIF/artifact upload; not SLSA certification. |
| [`semver.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/semver.yml) | main/v12/v13 push, PR, manual | Blocking | Fans out one job per machine-readable release-order entry and compares each supported library API with the latest stable, non-yanked crates.io release of the same major version (`.github/check-semver.sh`; selection tested by `.github/test-semver-baseline-constraints.py`). Prereleases may change the API, so a line without a stable release has no baseline: each 13.0 prerelease job reports that with a notice and compares nothing, and the 13.x comparison starts by itself once 13.0.0 is published. A tree whose version still equals that release is checked as the next minor release (breaking changes fail, deprecations pass); a bumped version is checked for the change it declares. The frozen `v12` branch runs its own copy of the workflow and script. Never-published packages and proc-macro/binary API surfaces unsupported by `cargo-semver-checks` are reported explicitly. The registry comparison skips pull requests to `main`; the `Public API snapshot` job also runs on them and compares each library's documented API with its listing in `.github/public-api/`. |
| [`spellcheck.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/spellcheck.yml) | main/v12/v13 push and PR, manual | Blocking | Repository typo scan. |
| [`supply-chain.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/supply-chain.yml) | main/v12/v13 push and PR, daily, manual | Blocking | Three jobs, which until 13.0.0-alpha.4 were `cargo-deny.yml`, `audit.yml` and `security-audit.yml`. `Check Dependencies (Licenses, Vulnerabilities)` runs Cargo Deny's advisory, license, ban and source policy from `deny.toml` (a required check on `main`). `Cargo Audit (production and fuzz locks)` audits the production lock and all eleven fuzz-package locks with one advisory-database fetch. `Advisory exception governance` checks that the active advisory IDs in the ledger, `deny.toml` and every workflow's `--ignore`/`--audit-ignore` flags agree, and that each ledger row has an owner and an unexpired date. No advisory is ignored today. The daily scheduled run uses its own concurrency group, so it never cancels the `main` push run that release admission requires. The `v12` lock audit was removed at the 12.x end of life (16 October 2026); the frozen `v12` branch keeps its own copies of the old workflows. Before Cargo Deny, `Check Dependencies (Licenses, Vulnerabilities)` runs `.github/check-public-dependencies.py` (tested by `.github/test-check-public-dependencies.py`), which keeps every [public dependency](docs/src/compatibility-policy.md#public-dependencies) on the 13.x line in `.github/public-dependencies.json`. |
| [`trufflehog.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/trufflehog.yml) | main/v12/v13 push and PR, weekly, manual | Blocking | Verified-secret scan over the configured Git history range. |
| [`udeps.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/udeps.yml) | weekly, manual | Informational | `cargo-udeps` signal on pinned `nightly-2026-08-21`; command failures are tolerated. |
| [`unsafe-policy.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/unsafe-policy.yml) | main/v12/v13 push and PR, manual | Blocking | Denies new production unsafe code and validates the reviewed exception allowlist. |
| [`verus.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/verus.yml) | manual-only v13 candidate | Strict pilot, outside release admission | Verifies the source-linked age-policy method predicate with pinned Verus/Rust/Z3 and requires three postcondition-failing controls. Uploads fingerprints, tool identities and resource measurements; hosted acceptance/promotion pending. |
| [`wasm-matrix.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/wasm-matrix.yml) | main/v12/v13 push and PR, manual | Blocking | Compiles Core, the public `rullst` facade (without features, with its defaults and with the CLI-generated application feature set) and macros for `wasm32-unknown-unknown` and `wasm32-wasip1`. |
| [`workflow-lint.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/workflow-lint.yml) | main/v12/v13 push and PR, manual | Blocking | Validates the shared fuzz inventory, then Actionlint checks workflow syntax, GitHub expressions and embedded shell using an immutable container digest. |
| [`zero-panics.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/zero-panics.yml) | main/v12/v13 push and PR, manual | Blocking | Panic-family Clippy lints plus generated-code regression checks for published runtime targets. |

### Informational workflows reviewed for 13.0.0-alpha.4

Each informational workflow's run history was read for 13.0.0-alpha.4, asking
whether a finding ever led to a commit and whether another workflow already
covers it. `corpus-sync.yml` was removed (see
[Fuzzing and OSS-Fuzz](#fuzzing-and-oss-fuzz)); these two stay:

- `mutants.yml` stays: it measures the mutation campaigns (MAINTAINING.md section 6), admits CLI files to `.github/mutants-pr-cli.txt`, and its targeted runs are dispatched and acted on routinely.
- `udeps.yml` stays: it is the only check of unused dev- and build-dependencies, because `cargo machete` reads only `[dependencies]`. Its weekly report listed the same findings from September 2026; 13.0.0-alpha.4 removed the three unused ones (`insta` in `cargo-rullst`, `proptest` in `rullst` and `loom` in `rullst-connect`). The remaining `rullst-orm` and `rullst-studio` entries are test dependencies the all-feature build compiles out, so any other entry is a new finding.

## Verification efficiency

These mechanisms shorten feedback without weakening the evidence boundary.
Affected-crate execution and general cross-commit reuse of release evidence
are **not enabled**; verified fuzz evidence reuse is the one bounded exception.

- **Verification-plan observation.** `workflow-lint.yml` records a
  `rullst.verification-plan.v1` report from `.github/plan-verification.py`,
  built from committed Git objects and TOML metadata without running Cargo or
  tests, over every dependency edge kind. Renames, deletions, symlinks,
  unknown paths, policy or lockfile changes, critical crates, generators,
  potentially executable Markdown and unsupported graphs keep the full
  recommendation; `may_skip_checks` is always false and no job consumes it.
- **Site-only admission.** `admit-site-only.py` and `plan-ci-scope.sh` skip
  the Rust CI runtime jobs only for a `main` or `v13` push that changes
  nothing but `docs/home_template.html`, `docs/site.css` or `docs/site.js`,
  when the immediately preceding commit has a successful full Linux push CI on
  the same branch within 72 hours with all 24 expected runtime jobs. Anything
  else (a skipped, unknown or failed job, missing history, a policy, symlink
  or mode change, a stale receipt) keeps full checks; receipts are never
  chained, and the helpers run from the preceding commit. Admitted pushes
  still build and validate the site; they are not release evidence.
- **Cheaper installs and preflights.** Documentation and Pages install a
  pinned prebuilt mdBook. `check-fuzz-locks.sh` resolves all eleven fuzz
  graphs with `cargo metadata --locked` before workspace Clippy (`--no-deps`
  would accept stale path-dependency locks). `check-package-reproducibility.sh`
  packages the two macro crates from two paths with different file times and
  umask and fails on different digests
  ([reproducible crate archives](docs/src/reproducible-builds.md)).
- **Threat-model runner.** Evidence rows are validated before compilation and
  each unique exact test runs once in its own all-feature process on a hash
  shard; a pass needs a successful exit and a libtest log with exactly one
  named, passed, non-ignored test. The LMS mapping (`ACADEMY-02`) runs in the
  LMS example (`rullst-lms-example`, `--lib`) without a nested build.
- **Timing reports.** `.github/report-ci-timings.py` keeps skipped jobs
  visible without a duration, fails on malformed or negative intervals in jobs
  that ran, and reports waiting separately from execution.

The invariants remain: 90% coverage floors, enforced negative authorization
and tenant-isolation tests, and no timeout or crash tolerated to shorten a
run. Change the automation, its policy tests and this document together. The
September 2026 measurements are in this file's git history; further CI
acceleration must meet the conditions in the
[verification roadmap](ROADMAP.md#verification-roadmap).

Read-only local commands (the planner inspects commits, not uncommitted files):

```bash
python3 .github/plan-verification.py --base main --head HEAD
python3 .github/test-plan-verification.py
python3 .github/test-report-ci-timings.py
python3 .github/test-admit-site-only.py
python3 .github/test-ci-scope.py
python3 .github/test-exact-rust-test.py
python3 .github/test-threat-model-runner.py
python3 .github/test-generated-evidence.py
python3 .github/test-fuzz-preflight.py
gh api --paginate --slurp \
  'repos/Rullst/Rullst/actions/runs/34980693742/attempts/1/jobs?per_page=100' \
  | python3 .github/report-ci-timings.py --top 10
```

## Verification roadmap

Planned assurance experiments (Loom/Shuttle, `cargo-vet` and others) are
tracked in the [verification roadmap](ROADMAP.md#verification-roadmap).
Differential database testing, the cross-browser and accessibility checks
and `cargo-careful` moved from it to the jobs described above.
