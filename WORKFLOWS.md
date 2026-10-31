# Rullst CI/CD and Verification Contract

This document describes what the repository's automation currently executes. It
is not evidence that a workflow has passed for a particular commit. A green
claim must always point to the GitHub Actions run, commit SHA, logs, and produced
artifacts.

Last source-level review: **2026-09-20 UTC**.

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

## Branch transition

On 21 September 2026, `v12` was preserved from stable source `184bc1f7`.
The hosted `main` and `v12` protections were read back with the same 43 required
GitHub Actions checks as `v13`, an up-to-date base, administrator enforcement,
conversation resolution and disabled force pushes/deletion. This is a settings
observation, not a successful test receipt. Promotion of v13 source to `main`
requires a normally admitted PR; existing tags, packages and running feature
campaigns are retained. `v13` remains transitional while that admission runs.

The default-branch Dependabot configuration targets Cargo (the root and the
eleven fuzz workspaces) and Actions on `main` only; `v12` receives no updates
after its end of life on 16 October 2026. Minor/patch Cargo changes share a PR,
the OpenTelemetry and utoipa families each have their own, and Actions share one
tools PR. Updates still require compatibility/MSRV and protected CI; group
membership is not approval. Old Dependabot PRs are closed after equivalent
updates have entered `main`, or with a comment when its policy excludes them.

## Mainline execution model

The development dashboard and its automatic status badges follow `main`. The
declared automatic release workflows accept pushes and pull requests targeting
`main`, stable maintenance `v12`, and transitional integration `v13`,
and expose `workflow_dispatch` where a safe rerun is useful. Superseded runs of
these workflows are cancelled per workflow and ref so rapid development does
not spend runner capacity proving an obsolete commit.

Release policy schema 4 binds v12 to `v12` and v13 to `main`. Historical
schemas 2 and 3 retain their original bindings. Before
building release artifacts, the tag, every inventory package version and the
current protected branch head must agree. Exact-source workflow admission and
the crates.io deployment approval remain mandatory. Fuzz receipts are limited
to the candidate's release line, including the source policy of reused runs.

`ci.yml` deliberately treats the expensive operating-system matrix differently.
Format and Clippy continue to give feedback on draft pull requests. A ready pull
request to `main` runs the fast Linux gate: the `workspace` and `cli-standard`
shards, MSRV, strict database features, Redis/S3 contracts, generated access,
facade composition and AI evals. Line coverage, the SemVer registry comparison
and CodeQL skip pull requests to `main`; the SemVer workflow's public-API
snapshot job runs on ready pull requests to `main`. The resulting `main` push runs every Linux
shard, public feature boundaries, the threat-model minimum, coverage, SemVer and
CodeQL. A nightly scheduled run (05:30 UTC) executes the complete
Linux/macOS/Windows matrix on the default branch. A maintainer can add the
`full-matrix` label to a pull request to `main` to run that complete
Linux/macOS/Windows test matrix (all seven shards on each system) before merge:
the `labeled` event starts it, and every later push re-runs it while the label
stays. The label changes only the `Run Tests` matrix; feature boundaries, the
threat-model minimum, the scorecard and the release-only packaging jobs keep
their pull-request guards. Every pull-request event, `labeled` included, uses
the pull request's concurrency group, so the newest run replaces the older one
and still reports every required check (`.github/test-ci-matrix.py` evaluates
the workflow's expressions to check this). Adding any other label re-runs the
same Linux gate rather than skipping it, because a skipped job would report a
passing required check. Ready pull requests to the
maintained `v12` line and transitional `v13` keep the complete matrix, blocking
line coverage, SemVer fan-out and CodeQL before merge. Any matrix can be
requested manually. Each operating system
executes seven parallel shards: the non-CLI workspace (which includes the Blog,
ERP, LMS, Portfolio and WebGPU examples), ordinary CLI targets, three public-profile groups
and two generated-starter groups.
The basic/relational/polyglot and foundation/product partitions retain every original
case while bounding the longest Windows and macOS jobs. No test is omitted;
this changes wall-clock scheduling rather than the assertions being executed.
Hosted CI permits two nested compiler jobs for the otherwise serial generated
profile/blueprint builds; constrained local runs retain their one-job default.
Each CLI
shard fetches the locked registry inventory before its generated applications
prove that they compile without network access. After a
reviewed commit is merged, the automatic `main` or `v13` push runs Linux only;
for `main`, macOS and Windows evidence comes from the nightly run. Release
candidates must use the manual full matrix when no successful full-matrix run
points to the exact candidate tree.

The SHA-bound quality scorecard is generated only by a ready pull request to a
maintained line, the nightly run or a manual full-matrix run. It is deliberately
skipped on Linux-only runs (automatic branch pushes and pull requests to
`main`), because they cannot honestly award cross-platform verification credit. Run `ci.yml` manually on a final release-branch candidate to
produce the exact-SHA release scorecard. Manual diagnostic runs may select one
operating system and one test shard; those deliberately do not produce a
full-matrix scorecard and do not replace final-candidate evidence.
The manual `cli-updates` diagnostic selects discovery/cache/artifact tests,
isolated project preparation/verification and legacy upgrade process fixtures.
It does not replace the complete `cli-standard` shard in release matrices.
Manual CLI-only shards also skip unrelated ORM/Redis/feature/threat/eval/facade
and MSRV jobs, keeping correction runs bounded. Manual `workspace` and `all`
selections, automatic runs and ready PRs retain those jobs. Strict workspace
Clippy/format still runs for every diagnostic; release admission continues to
require every job from the full `all`/`all` matrix.

The manual `all`/`all` matrix additionally packages the 20 release-inventory
crates, including `rullst-labs`, audits their contents and
uses the release pipeline's archive-only consumer and isolated CLI
installation/blueprint checks. `Packaged distribution and
installed CLI` is a required exact-SHA release-admission job. It runs without
registry publication credentials and never uploads to crates.io. Automatic
development runs and other diagnostic subsets deliberately skip this expensive
job. The explicit `platform=all`, `shard=packaged-distribution` diagnostic runs
this same archive/installed-CLI check and workspace Clippy/format, without
repeating the OS test matrix or native artifact builds. It emits no full-matrix
scorecard and cannot satisfy complete release admission; the final `all`/`all`
and tag pipelines retain their existing package verification.

`Generated VM deployment (docker)` and `(podman)` run
`.github/check-vm-deploy.py` (about ten minutes each): they install the CLI
from the packaged archives and, for the Blank and SaaS starters, run
`cargo rullst deploy`, copy the project as the
[VM guide](docs/src/vm-deployment.md) does, check `.env.production` with
`deploy:doctor`, build the image with the generated `Dockerfile` and start the
generated `compose.yaml` and `Caddyfile`. They check `/health` over HTTPS
through Caddy (verified against Caddy's local CA for `localhost`), the
HTTP-to-HTTPS redirect, that the app port is not published, the read-only root
filesystem, capabilities and `no-new-privileges` of both containers (read from
`/proc`), Caddy's trusted-proxy address, `db:migrate`, a database-backed page
(SaaS: registration and sign-in with a Secure session cookie), that the SQLite
database stays in its volume across `docker compose restart` (Podman: `down`
and `up -d`, because podman-compose's `restart` can leave Caddy stopped), the
guide's backup command and that no container logged a panic. The release
crates enter the build context as packaged archives through
`[patch.crates-io]`; the generated deployment files run unmodified. Docker uses
the runner's daemon; Podman runs rootless through `podman compose` with
podman-compose and the `net.ipv4.ip_unprivileged_port_start=80` setting the
guide documents. Full manual runs, the nightly run and the manual
`vm-deployment` diagnostic run both jobs; a ready pull request runs them only
when it changes `cargo-rullst/src/generators/deploy.rs`,
`generators/project/docker.rs`, `blueprints/deploy.rs` or the check (otherwise
the job succeeds without building), and branch pushes skip them, so the push
inventory that site-only admission reads is unchanged. They are not listed in
the release policy's required jobs, but a failure fails the full manual run
that admission requires. Real DNS, Let's Encrypt, provider accounts, Arm
images and reboots are not covered.

On Linux, the `cli-profiles-relational` shard then runs
`.github/check-auth-starter-sql.py` with the CLI it built. For PostgreSQL 15,
MySQL 8.4 and MariaDB 11.4, each in a disposable container bound to loopback,
it generates a Blank starter, points its `.env` at the server, runs
`cargo rullst db:migrate`, inserts two rows (a name; default timestamps), runs
`cargo rullst auth`, mounts the sign-up routes as Tutorial 11 describes and runs
`db:migrate` again. SQL against the server then checks the six added columns
(types, nullability and defaults), the unique `users_email_unique` index and
that it rejects a duplicate, the `user-<id>@no-email.invalid` placeholders
without a password, that the starter's columns, indexes and rows are otherwise
unchanged, and the batch's migration records and `password_reset_tokens`
table. The built application must sign up an account over HTTP (`/dashboard`
opens, the row holds an Argon2id hash). `db:rollback` must restore the
starter's columns, indexes, tables and migration records exactly and keep
every row, and a second `db:migrate` must apply the batch again. The generated
builds reuse the shard's target directory and the workspace `Cargo.lock`
offline. The shard runs on `main` pushes, nightly, with the `full-matrix`
label and in manual runs (the manual `cli-profiles-relational` selection with
`ubuntu-latest` runs it alone); a pull request to `main` without the label does
not. The API and SaaS starters, other server versions and existing deployments'
data are not covered.

`First-hour documentation` runs `.github/check-first-hour.py`: it follows the
path the documentation gives a newcomer, running the shell blocks and applying
the code blocks marked `<!-- first-hour: … -->` in the README quickstart, the
[getting-started page](docs/src/1-getting-started.md), tutorials
[01](docs/src/tutorials/01-hello-world.md),
[02](docs/src/tutorials/02-cli-generators.md),
[03](docs/src/tutorials/03-active-record-crud.md),
[05](docs/src/tutorials/05-migrations-and-seeds.md) and
[11](docs/src/tutorials/11-authentication-system.md) (the last four in one
`shop` project) and the [REST API quickstart](docs/src/tutorials/rest-api-quickstart.md),
in reading order. The marker grammar is in `.github/first_hour_docs.py`; what
each page says happens (files written, `/health`, the welcome page, the API
starter's bearer-protected `POST /api/messages`, migrations, rollback and
seeding in SQLite, sign-up and sign-in after `cargo rullst auth`) is checked by
`.github/first_hour_checks.py`, keyed by page and step. The CLI is installed
from the packaged archives, and a `[patch.crates-io]` in the scratch
directory's `.cargo/config.toml` resolves the unpublished release crates to
them, so every project is built as written; only `cargo add rullst@=<version>`
is replaced by the line it writes while that version is not on crates.io.
`cargo rullst new` without `--default` answers the wizard with Enter in a
pseudo-terminal, and `cargo run`/`cargo rullst dev` run in the background until
the next block stops them with SIGINT, as Ctrl+C would. A failure is annotated
on the page's line. The trigger rules are the deployment job's: full manual
runs, the nightly run and the `first-hour` diagnostic run it; a ready pull
request runs it only when it changes a marked page, `cargo-rullst/src/` or the
check, and branch pushes skip it. It is not a required release job.
`.github/test-first-hour.py` (in the Documentation workflow) validates the
markers, the documented install versions and the edits offline on every push.
Pages outside the first hour, external accounts (Turso, Stripe, mail
providers), PostgreSQL/MySQL servers and the browser itself are not covered.

Manual all-platform `all` and `cli-standard` selections also call the native CLI
artifact builder for Linux x64, Windows x64, macOS ARM64 and macOS x64. The
committed target inventory selects explicit runner labels. Both executable entry
points must run and report the candidate version before bounded files, digests
and source/platform metadata are retained. These ordinary CI artifacts have no
release tag and no installation or publisher-verification authority. Exact
full-candidate admission requires all four native jobs; a diagnostic CLI shard
still does not replace the full matrix.

The tag-only release also calls this builder after protected release-branch admission.
The separate attestation job verifies downloaded checksums and includes native
executables/manifests in build provenance without executing source or binaries.
The GitHub release job adds those assets only after attestation and registry
publication succeed. This prepares distribution, not an installer; no new
installation command or platform recovery claim is implied.

The existing Linux `workspace` shard also requires
`RULLST_UI_BROWSER_TESTS=1` with Node 24 and Chromium. It feeds HTML from the
real Nexus renderer and the Portfolio example's view (`examples/portfolio`) into
the bounded `.github/mobile-ui-browser-smoke.mjs` contract: drawer
dismissal/focus/no-JS behavior, responsive layout at 320–1440px, long content
and reduced motion. Missing browser prerequisites fail that shard. Other platforms still run the render assertions and the complete
existing project matrix; they do not claim Chromium evidence. Local runs opt in
with the same flag and an absolute `RULLST_UI_BROWSER_SCRIPT` path. CDN resources
are blocked in this UI fixture. The Nexus run serves the shell with the default
production CSP and its real same-origin assets, and fails on a CSP violation;
this is not live-provider, WebKit/Firefox,
hardware-device, or WCAG certification. No extra Rust compilation matrix is added.

GitHub executes `schedule` events from the repository's default branch, so
scheduled checks follow v13 development on `main`. Stable `v12` receives
its own push/PR checks and explicit manual campaigns; a scheduled `main` run
is not stable-line evidence.
Tag publication remains deliberately unavailable through a manual button.

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
| `kani.yml` | Twenty named bounded formal harnesses in ten supported runtime/library packages | **Required release evidence for the declared harnesses:** every proof has an isolated strict matrix job. Rullst itself stays on stable Rust 1.99.0 with a Rust 1.96 MSRV; only the separately built Kani verifier uses its pinned `nightly-2026-08-01` compiler (`rustc 1.99.0-nightly`) because the latest stable Kani bundle's Rust 1.93 compiler cannot compile the framework. The proc-macro-only `rullst-macros` target remains unsupported by Kani and is covered by compile-pass/fail and generated-project evidence instead. |
| `miri.yml` | Randomized-layout Miri execution over 15 named pure-Rust/default-feature scopes | **Required release evidence for the declared scopes:** every selected scope is strict. This nightly-only interpreter uses pinned `nightly-2026-08-21` (`rustc 1.100.0-nightly`); it does not change the project's stable toolchain or MSRV. Native FFI, OS syscall, network/provider, umbrella re-export, and example-application boundaries are excluded explicitly rather than emitted as tolerated errors. |
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
| Weekly | `bench.yml`, `codeql.yml`, `coverage.yml`, `crypto-containment.yml`, `documentation.yml`, `kani.yml`, `miri.yml`, `proptest.yml`, `scorecards.yml`, `trufflehog.yml`, `udeps.yml` | The inventory below identifies which results are blocking, automated evidence, or informational. Miri (Wednesday 01:17 UTC) and Kani (Thursday 01:47 UTC) run their full strict matrices so that a regression shows up within a week rather than at release admission. |

All remaining test/build workflows run on the documented push, pull-request or
path filters and also expose a manual rerun. For an RC or stable checkpoint,
first use the automatic mainline suite, then run the exact-SHA manual admission
set: full-platform `ci.yml`, release-mode `fuzzing.yml`, `dast-zap.yml`,
`kani.yml`, `miri.yml`, `sanitizers.yml` and `proptest.yml`. The tag workflow checks those receipts through
`.github/release-required-workflows.json`; the fuzz receipt is accepted only
when all 42 named target jobs and its evidence boundary succeeded. A scheduled
run of `ci.yml`, `sanitizers.yml`, `kani.yml`, `miri.yml` or `proptest.yml` is
never a receipt, even on the candidate commit: admission accepts only
`workflow_dispatch` runs, and each scheduled run has its own concurrency group
(`-nightly` or `-weekly` suffix), so it cannot cancel, or be cancelled by, the
dispatched admission run on `main`. Physical
devices, store approval, live provider accounts, external security review and
human release approval remain outside GitHub Actions.

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

Rust CI disables Cargo incremental compilation and uses the pinned `sccache`
Action and binary to store content-addressed compiler outputs in GitHub Actions
cache. The current `cc` build dependency also honors the same Rust compiler
wrapper, so compatible bundled DuckDB C++ objects can be reused. Same-repository
pull requests can populate only their GitHub-isolated `refs/pull/.../merge`
cache scope, making a failed-job rerun useful without modifying the trusted
default-branch namespace. Fork pull requests remain read-only. Pushes and
explicit manual runs on `main` populate entries reusable by later pull
requests. Cache contents never substitute for a test result.
The tag-only verifier uses the same namespace strictly read-only, so it can
reuse an exact trusted compiler output but cannot alter the cache while
creating release artifacts; every release command and assertion still runs.

The setup action's job-scoped Cargo archive is disabled in these compiler-cache
jobs, so neither the raw workspace `target` tree nor duplicate registry bundles
compete with compiler objects. CLI integration tests create and remove nested
Cargo targets, and archiving whole mutable trees previously produced false
missing-directory annotations, duplicated roughly 9.56 GiB across twenty
active main caches, and caused eviction churn at GitHub's default 10 GiB
repository limit. Cargo may redownload registry sources on a fresh runner; this
small network cost is preferable to storing the same registry/target archive
under many job-specific keys. A first run on a new cache namespace is still a
cold build; evaluate acceleration using the reported cache hit ratio and a
later compatible run, never by weakening or omitting assertions.

The same content-addressed approach accelerates LLVM coverage, benchmark,
mutation/fuzz compilation and the scheduled release-mode regression suite.
Benchmarks remain
sequential on one runner so comparisons do not mix host variance. Coverage
deliberately remains one report job because splitting it without a reviewed
profile-data merge could change the repository percentage. Its second
default-feature pass is limited to ORM, Studio and the public facade: those are
the packages with default-SQLite tests excluded by the mutually exclusive
all-feature graph, so unrelated workspace tests are not repeated. The release-mode
workspace is safe to split because every shard
returns an ordinary test result, while the two source locations that actually
use `proptest!` still receive their configured 10,000-case runs. SemVer checks
fan out from the machine-readable release order and validate one published API
per job, so adding or removing a release package cannot silently drift from the
matrix.

The `Public API snapshot` job of `semver.yml` runs `.github/public-api.py
--check` (after its unit tests, `.github/test-public-api.py`). It builds the
rustdoc JSON of each library package in the release order, except the
`cargo-rullst` CLI implementation, with the features and rustdoc arguments of
its `[package.metadata.docs.rs]` table, on the pinned `nightly-2026-08-21`.
It renders every documented item, signature, field, variant, implementation
and auto trait into `.github/public-api/<package>.txt` and fails when the result
differs from the committed file. Items hidden with `#[doc(hidden)]` and blanket
implementations are not listed. A pull request that changes a public API
commits the regenerated listing, so the change is reviewed as a diff. The job
runs on ready pull requests to `main`, on pushes and on manual runs; a manual
run with `public_api_only` skips the registry comparison. It is not a required
status check, but a failure fails the `semver.yml` push run that release
admission requires.

The all-feature coverage pass uses pinned cargo-nextest to schedule the same
discovered unit and integration tests concurrently. It performs no retry and a
flaky retry could not be normalized into success. A repository profile fixes
four global slots and permits at most two integration tests that launch nested
generated-application Cargo builds, preventing compiler fan-out from trading
latency for memory or disk exhaustion. The exact per-test JUnit result and
duration record is retained for 30 days. This runner is a coverage scheduler,
not a replacement for Rust CI: the required multi-platform shards continue to
execute the complete inventory with ordinary `cargo test`, preserving both
traditional libtest process semantics and the materialized-project gates.

CodeQL runs separate Rust and JavaScript/TypeScript analysis jobs. The Rust
job's Cargo target cache is disabled so the
extractor observes compilation for the exact SHA instead of inheriting a fresh
artifact from another run; the analysis database itself is not interchangeable
with ordinary test shards. Rust CodeQL's faster buildless mode is intentionally
not used because manual compilation gives the extractor the stronger generated
code boundary needed by this release. The JavaScript job uses buildless analysis
and retains the `/language:javascript` category to compare against existing
mainline results without leaving that language unexamined.

`ci.yml` also compiles and exercises each ORM strict database feature in
isolation (PostgreSQL, MySQL, and SQLite), exercises the runtime-only Core and
all 70 public umbrella features in isolated additive graphs with automatic
manifest-drift detection, runs the portable database matrix on Linux, and
tests the complete all-feature workspace in seven parallel shards on Linux,
macOS, and Windows. Feature-boundary rows and threat-model negative tests also
fan out into four deterministic strict shards each; their matrix job remains a
single blocking dependency for the quality scorecard. Each threat-model shard
primes the reviewed lockfile before its deliberately offline generated-project
checks, so it does not inherit a hidden source-cache dependency from another
job. The umbrella's
`cfg(doctest)` aggregation reads all 48 public tutorial files directly, so that
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

After a ready-PR or manual full-matrix Rust CI run finishes, an observational
job emits a SHA-bound per-crate quality scorecard into the workflow summary and
a 90-day artifact. The score combines versioned expert-audit ceilings with the
actual gate results; a failed/skipped/cancelled gate can remove the dimensions
it was meant to prove, while a green gate cannot inflate a crate beyond its
audited ceiling. This is engineering-evidence reporting, not capability completion or
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

## Recommended branch-protection profiles

**`main` (v13 development)** requires the fast gate of 19 checks that every
ready pull request emits: Code Quality & Format; Run Tests (ubuntu-latest / workspace) and
(ubuntu-latest / cli-standard); Check MSRV (Rust 1.96.0); ORM strict-sqlite,
strict-postgres and strict-mysql; Redis live contracts; Generated release access
boundaries; Facade shared-local recovery composition; Versioned deterministic AI
evals; Build book and validate local links; Validate workflow syntax and
expressions; Enforce Zero Panics (published runtime targets); Enforce production
unsafe boundary; Check Dependencies (Licenses, Vulnerabilities); Scan for leaked
secrets; Enforce reviewed internal dependency graph; and Typo Check. It does not
require an up-to-date base: the full `main` push run and the nightly matrix
detect interactions between independently green pull requests, and each
additional rebase would otherwise repeat the whole gate. Cargo Deny's job in
`supply-chain.yml` is the required vulnerability gate. Until 13.0.0-alpha.4 the
two Cargo Audit workflows both emitted an ambiguous `audit` context; their jobs
now have distinct names (`Cargo Audit (production and fuzz locks)`, `Advisory
exception governance`), so either could be required without a collision. The
macOS/Windows and CLI-profile shards, public feature boundaries, threat-model
minimum, coverage, SemVer and CodeQL run after merge, nightly or weekly. They
remain release-admission requirements.

**`v12` (frozen; end of life on 16 October 2026)** keeps the complete profile
of 46 checks with an up-to-date base: every Linux/macOS/Windows test shard,
public feature boundaries, the threat-model minimum, CodeQL, documentation and
workflow lint.

Do not configure a path-filtered, scheduled, manual, deployment, or tag-only
workflow as a universal required check: an intentionally skipped workflow may
never create the check context. In particular, IoT and Vault Crypto Containment is
blocking when relevant paths change.
Pages, benchmarks, fuzzing, sanitizers,
Kani, Miri, mutation testing, udeps, ZAP, Scorecard, and release provenance
belong to deeper evidence or release policy. GitHub repository rulesets remain
the enforcement source; this document records the recommended profile. On
29 October 2026 the hosted rulesets matched it: the
[development-branch ruleset](https://github.com/Rullst/Rullst/rules/24175738)
on `main` and the
[maintained-branch ruleset](https://github.com/Rullst/Rullst/rules/24080901)
on `v12` (see [GOVERNANCE.md](GOVERNANCE.md#change-and-release-controls)).

### Observed v13 protection

On 20 September 2026 UTC, the hosted `v13` branch protection was enabled and
read back from the GitHub API. It requires 43 existing GitHub Actions contexts:
the 25 Linux runtime jobs, all sixteen corresponding macOS/Windows test shards,
documentation and workflow lint. Contexts are bound to GitHub Actions, the base
must be current, a pull request and resolved conversations are required, and
administrators are included. Force pushes and branch deletion are disabled.
No additional human approving review is required. These are hosted settings at
that observation, not a guarantee that an administrator cannot change them.

This initial merge profile does not replace the release policy's 28 complete
workflow receipts, manual native/security jobs, package inspection or protected
crates.io deployment approval. The additional automatic security workflows
are enabled on v13 by this preparation change; the profile above names only
checks already emitted by the integration candidate. The `v13` branch was later
retired into `main`, and the release policy now requires 25 workflow receipts
(`.github/release-required-workflows.json`).

## Phase 4 release-engineering status

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

`zero-panics.yml` denies Clippy's unwrap, expect, panic, todo, and unimplemented
lints for published runtime libraries, procedural-macro engines, CLI production
targets, generated runtime templates, and the Wasm Core path. Tests are excluded
where assertion panics are test semantics.

`unsafe-policy.yml` compiles production libraries and binaries with
`-Dunsafe-code`. The exact reviewed source allowlist contains the Radar OS probe,
dynamic-library loader, the CLI's Windows cache/installation owner/DACL boundary,
Windows project access-policy preservation and macOS extended-ACL inspection.
Both inner and outer unsafe-lint attributes enter the source inventory.
The macOS module borrows an open descriptor,
inspects the first ACL entry and frees the returned allocation; it never edits
an ACL. Its native regression adds an ACL and verifies that replacement rejects
without discarding it. Windows descriptors are installed atomically and
validated through owned handles. Each unsafe call documents pointer/handle
ownership and lifetime; the workflow fails if the source allowlist changes.
Windows ACL counts come from `GetAclInformation` into owned output structures;
ACE pointers must be non-null before creating bounded borrowed views. SID lengths
are checked before OS validation and no borrowed pointer outlives its descriptor.
This is an enforced boundary, not a claim that all dependencies contain no
unsafe Rust.

### Coverage

`coverage.yml` runs LLVM coverage over workspace all-features and default
profiles plus the live database matrix, then uploads LCOV to Codecov using
GitHub OIDC rather than a long-lived upload secret. It also retains exact JSON
and text line summaries for 30 days so a passing upload cannot be confused
with the coverage percentage. The all-feature pass uses pinned cargo-nextest
only to run the same discovered tests concurrently, with zero retries, bounded
nested-Cargo concurrency and a retained JUnit execution record. Rust CI still
runs ordinary `cargo test` across Linux, macOS and Windows. The default-profile pass explicitly includes
ORM, Studio, Nexus and the umbrella facade so their real SQLite contracts are
not hidden by mutually exclusive all-feature database profiles. Before upload,
the workflow independently rejects an LLVM summary below 90% for either the
whole repository or the governed framework-library paths. `codecov.yml` also
requires at least 90%, with zero tolerance, for those views and changed lines;
failure to upload LCOV also fails the workflow. The report
filters examples, benchmarks, auxiliary test support, and separate test files.
CLI and proc-macro code therefore remains part of the blocking repository
aggregate and is additionally visible as informational components. Their
stronger semantic evidence still comes from materialized scaffolds and
compile-pass/compile-fail contracts. The README exposes both the public overall
badge and the separate `framework_libraries` badge rather than substituting the
higher component result for the repository total.

### Formal, dynamic, and stress analysis

- Kani and Miri are bounded evidence scoped to the harnesses/packages that
  actually execute. Both run weekly on `main` so that a regression appears
  between releases; release admission still requires a dispatched run on the
  exact candidate commit. Rullst itself remains pinned to stable Rust 1.99.1
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
- Mutation testing is manual, split into 80 lossless shards with at most four
  running concurrently, and intentionally informational. A new campaign discovers
  its selected source's inventory; the historical 14,391-candidate receipt is not
  a permanent limit for later sources. Before the expensive matrix starts, a fail-fast `--list --json`
  preflight verifies the exact unique candidate set; the final aggregate must
  match that reviewed list, not merely its count. Full mode optionally accepts
  a complete `source_sha` that must be an ancestor of the workflow commit, allowing
  a repaired controller to measure an immutable published release. Provenance
  distinguishes the measured source from the workflow source. Targeted and
  recovery modes reject that override; recovery still enforces its reviewed
  historical digest. The hosted command makes
  `--all-features` explicit and
  `.cargo/mutants.toml` applies the same feature policy locally; the ignored
  legacy root configuration and its exclusions were not silently activated.
  Targeted mode accepts exactly one tracked production `.rs` path so a
  correction can be retested without restarting the complete workspace
  campaign. Each targeted run uses its own per-file concurrency group, an
  explicit per-mutant test timeout (`timeout_seconds`, default 300 s, which
  also bounds the unmutated baseline) and a 300 s build timeout, so one hanging
  mutant cannot consume the job budget. Its optional `test_scope=lib` runs only
  the package's library unit tests (`cargo test --lib`) to avoid rebuilding
  generated projects in slow integration suites; that narrower scope can report
  a mutant as missed when only an integration test would catch it, and such a
  mutant still gets a unit test. Each targeted test binary runs under an
  8 GiB address-space limit (`prlimit` as the Cargo test runner), so a
  mutant that allocates without bound aborts its own tests instead of
  exhausting the runner. An optional `shard=k/n` slice (n at most 16)
  splits a file that is too slow for one job; the preflight inventory and the
  aggregate then cover exactly that slice. Every full and targeted shard has
  the pinned Redis service of `ci.yml`'s Redis job and sets
  `RULLST_TEST_REDIS_URL`, so the Security rate-limit and Core cache
  inspection Redis contracts run in the baseline and for every mutant (Core
  queue scheduling and the ORM Redis data matrix start their own
  testcontainer). The ORM query-cache and hash-scope Redis contracts need the
  non-strict pool, so the all-feature build compiles them out and no mutation
  run reaches them. [Contract mode](#owned-server-contract-mutation-runs) covers the
  adapters that only a wrapper-owned server reaches. Recovery is governed by a committed, versioned policy binding the
  originating and continuation runs, attempts, branches, workflow identity,
  measured source SHA, cargo-mutants version, exact shard set and inventory
  digest. It bisects only authorized incomplete fragments and combines them
  with successful immutable artifacts. The content-addressed mixed aggregate
  still has to match the complete reviewed inventory exactly once. Exit statuses for missed and timed-out mutants remain findings;
  baseline, usage and internal failures do not get normalized into green jobs.
  The aggregate also fails closed when an artifact is absent, a shard is
  incomplete or the reviewed full inventory drifts; its conservative
  percentage never treats a timeout as caught.
- Fuzzing pins `nightly-2026-08-21` instead of following
  a moving nightly alias. This verifier-only toolchain does not change the
  framework's stable Rust 1.99.1 toolchain or its Rust 1.96 MSRV. Before a
  release campaign starts, eleven strict preflight jobs compile every target so a
  stale import or broken fuzz manifest fails in minutes rather than alongside
  hours of valid campaigns. A five-minute one-target diagnostic is correction
  feedback only; the evidence-boundary job refuses to call it release evidence.
  Each of the eleven fuzz packages has a checked-in dependency lock; inventory
  validation requires it, and locked metadata plus a post-command drift check
  prevents preflight or campaign jobs from silently resolving a
  different dependency graph.
  The parser
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
  REST API and a fresh SaaS starter through the real CLI, and release-builds
  and migrates the LMS example from the workspace (the LMS starter moved to
  `examples/lms` in 13.0.0-alpha.4). The generated manifests name the
  framework crates as a published CLI writes them, and
  `.github/dast-generated-app.py` resolves them from the checkout through
  `[patch.crates-io]`, so the build is the same before and after a stable
  release. The SaaS starter runs in production with placeholder Stripe
  settings (production refuses mock billing; no checkout is reached). One
  baseline crawls its public pages (pricing, sign-in, registration, password
  reset); its form submissions fail the CSRF check because ZAP does not return
  the `Secure` cookie over plain HTTP. A second baseline sends the session
  cookie of an account registered for the run with every request
  (`ZAP_AUTH_HEADER`), so it crawls the dashboard and password-change pages and
  submits the forms with a valid CSRF token. `.zap/saas-starter-account.tsv`
  puts only the POST-only sign-out and billing commands out of scope (signing
  out everywhere would revoke the scanning session; checkout and portal open a
  provider session), and the session must still open the dashboard after the
  scan. These baselines fail on any
  warning/failure, preserve INFO observations and use no ignored rules. The
  release blog showcase is scanned separately but remains informational because
  its documented presentation boundary deliberately uses a relaxed CSP and
  third-party assets. Its rules retain those external-asset findings for review
  and reduce only evidenced token/state signals or escaped showcase reflections
  to INFO; they do not hide findings with `IGNORE`. These four targets are representative evidence, not
  coverage of every starter, authenticated role (Nexus administrators are not
  scanned), browser, proxy or deployment.
- Property tests and benchmarks are scheduled/manual evidence. The property
  workflow preserves the complete all-feature release-mode regression suite in
  seven parallel shards and separately runs the ORM and Connect property
  contracts with 10,000 generated cases. The eight published benchmark groups,
  backed by nine Criterion binaries, emit
  non-blocking alerts at a 20% regression and feed the
  [public benchmark hub](https://rullst.github.io/Rullst/benches/); they are not
  a promise against every nanosecond-level regression.

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

The CLI's integration suites build generated projects, which is too slow to
repeat for every mutant, so `.github/mutants-pr.sh` runs two passes. The
workspace pass tests each mutant with its package's whole test suite and
leaves `cargo-rullst` out; the CLI pass mutates only the files listed in
`.github/mutants-pr-cli.txt` and tests them with the CLI's library unit tests
(`cargo test --lib`), the scope of a targeted `mutants.yml` run with
`test_scope=lib`. A CLI file joins the list once such a run leaves no missed
mutant apart from accepted equivalents; the other CLI files are not mutated by
the gate.

### Accepted equivalent mutants

The mutants that no test can tell apart in the all-feature Linux build, each
with a one-line justification, and the status of every mutation campaign are
recorded in the [mutation testing record](docs/src/mutation-testing.md). The
pull-request gate skips an entry only when its exact cargo-mutants name is also
listed in `.github/mutants-pr-equivalents.txt`.

### Owned-server contract mutation runs

Some adapters are reached only by ignored contract tests that a wrapper runs
against a disposable Docker server it owns, so a full or targeted run reports
their database-bound mutants as missed. `mode=contract` measures them: it runs
`.github/check-<contract>.py --mutants REGEX [--shard k/n]` on a hosted runner
and uploads `mutants.out` as `mutants-out-contract`. The wrapper starts its
server, mutates only its adapter files and runs only the first (exercise)
phase of its contract tests, one at a time, never the restart phase.
`.github/mutation_contract.py` is the Cargo target runner for those tests: it
resets the server before every test binary (a recreated database of the same
name, no leftover runtime roles, `ALTER SYSTEM RESET ALL`, or `FLUSHALL` for
Redis), because a mutant can stop a test halfway and leave a role, a disabled
`fsync` or a full Redis behind. Missed and timed-out mutants are findings; a
failed baseline, a baseline that did not run every contract test, a failed
reset or an incomplete classification fails the run. Because only the contract
tests run, a mutant that a unit test of the same file catches can still be
reported as missed: compare it with a targeted run of that file. One build
directory serves every mutant, so the job compiles incrementally, without
sccache (which refuses incremental builds), and rebuilds a mutant in about a
second.

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

`orm-sqlite` owns no server. Targeted and full runs build the workspace with
every feature, which selects the strict PostgreSQL pool and compiles the
SQLite behaviour tests of `rullst-orm` out, so they report the mutants that
only a database statement reaches as missed. This contract builds the ORM
with `drivers-all` instead and runs its whole suite on SQLite for every
mutant, including the code `#[derive(Orm)]` generates; the testcontainers
matrices are skipped. Code compiled only with every feature, such as the
PostgreSQL-only branches, is measured by the targeted runs. `orm-polyglot`
adds the optional backends, whose tests use mocks and loopback servers, and
runs the query-cache and Redis hash-scope contracts against a Redis server it
owns; DuckDB, whose bundled build is large, stays with targeted runs. `orm-sql-servers`
measures the dialect branches (placeholders, migration locks, collations and
payload types) against real servers; every mutant waits for three of them, so
select only those branches. `orm-strict-postgres` and `orm-strict-mysql` run
one of those matrices in the build of a strict pool, for code that the `Any`
pool does not compile.

The MySQL statement files `rullst-capital/src/quota/sql/mysql.rs` and
`rullst-ai/src/ai/memory/sql/mysql.rs` hold only string constants, so
cargo-mutants generates no mutant for them. The privacy URL policy in
`postgres_connection.rs` and the capacity check have unit tests. So do the
mail adapter's clock, event-time, stored-time and URL checks, the privacy
choice-code decoding, and, in `passkey-postgres` and
`auth-recovery-postgres`, most branches of `manager.rs` and the recovery
files: a contract run reports those mutants as missed, and a targeted run
of the file catches them.

### Fuzzing and OSS-Fuzz

The v13 manual `fuzzing.yml` matrix covers all **42** declared libFuzzer targets
(the immutable v12 line retains 40):
Core 12, ORM 5, Security 7, Connect 3, Mail 4, AI 3, IoT 3, Capital 1, Nexus 1,
Studio 1 and Privacy 2. The checked-in `.github/fuzz-targets.json` is validated against
every `*/fuzz/Cargo.toml`, corresponding lockfile and source file before the
manual campaign can fan out. Release mode verifies input-equivalent historical
results and compiles the remaining targets in package-level preflight jobs
before starting any long campaign. Campaign jobs restore and save versioned
per-target corpora in the Actions cache and share one content-addressed
compiler-cache namespace; campaign failures retain their exact reproducer.
The weekly `corpus-sync.yml` warm-up was removed in 13.0.0-alpha.4: the
repository cache runs above its 10 GiB limit, so no saved corpus survived to
the next weekly run or to a release campaign (the weekly runs of 11, 18 and 25
October 2026 and the release campaign of 24 October all reported "Cache not
found"), and its one-minute runs never produced a reproducer. The same
eviction applies to the corpora the campaign saves. A clean
run retains its original source SHA, target, corpus, toolchain and time budget;
reuse adds a separately verified relationship to the candidate inputs and
never claims a fresh execution. Diagnostic mode is limited to one exact inventory target for five
minutes and is never counted as the complete release gate.

The `oss-fuzz/projects/rullst` directory is a local integration draft. It is not
proof of upstream acceptance, continuous ClusterFuzz execution, or coverage of
all 42 targets; its helper build must be completed and validated against the
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
exceptions, commit context, checksums, and release notes rendered from the
exact matching changelog section: the section's introduction and "Known
limitations", plus a link to the full section at the tag (the whole section
when it has no introduction). The `.crate` archives and evidence receive
GitHub's SHA-pinned build-provenance attestation. This does **not** by itself
establish a project-wide SLSA level, Sigstore Cosign binary signing, independent
review, or regulatory compliance.

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

### Reviewed Action updates on the v12 maintenance line

Keep CodeQL `init`, `analyze`, and `upload-sarif` on one reviewed Action
revision. Dependabot groups these sub-actions, and the local pin validator
rejects mixed CodeQL revisions inside a workflow. Review the scanner bundle
change as well as the wrapper SHA; the 4.38.0 update selects CodeQL 2.27.0.

`setup-rust-toolchain` 2.0.0 changes warning enforcement from `RUSTFLAGS` to
`CARGO_BUILD_WARNINGS` (Cargo 1.97+). The v12 migration explicitly retains
`rustflags: "-D warnings"` and `build-warnings: ""`, preserving the previous
compiler flags, strict warning behavior and compiler-cache inputs. This does
not disable Clippy's explicit `-D warnings`. The MSRV job still uses a separate
toolchain installer and Rust 1.96.0. Adopting the new Cargo warning mechanism
is a separate measured migration, not an implicit side effect of updating an
Action. See the [upstream migration notes](https://github.com/actions-rust-lang/setup-rust-toolchain/releases/tag/v2.0.0).

TruffleHog's composite Action pin does not pin its default `latest` scanner
image. The workflow therefore also specifies the reviewed 3.97.4 multi-platform
image digest. Future scanner updates must review and change that digest;
Dependabot updating the wrapper alone is insufficient. The scan's existing
scope and verified-secret failure policy remain unchanged. These automation
updates do not change published crate versions or constitute a v12.0.1 release.

## Workflow inventory (34 definitions)

Durations are intentionally omitted because runner load, cache state, and the
dependency graph make static estimates unreliable.

| Workflow | Trigger | Mode | Actual scope |
| :--- | :--- | :--- | :--- |
| [`architecture.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/architecture.yml) | main/v12/v13 push and PR, manual | Blocking | Compares Cargo's publishable non-dev internal dependency graph with the reviewed `crate-architecture-policy.json`; unreviewed normal/build edges, removals, or optionality changes fail, while test-only dev-dependencies do not masquerade as production coupling. |
| [`bench.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/bench.yml) | main push, weekly, manual | Automated evidence | Eight published groups backed by nine Criterion binaries, with non-blocking 20% regression alerts and gh-pages data consumed by the benchmark hub. Scheduled runs use the repository default branch. |
| [`ci.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) | main/v12/v13 push and PR (Linux gate on PRs to main; full OS matrix with the `full-matrix` label), nightly full matrix, manual | Blocking plus observational report | Format, all-target/all-feature Clippy, seven-shard multi-OS tests including Cargo-aware doctests sourced from all 48 tutorials, four-way feature/threat partitions, the SQLite transactional outbox contract and Messaging concurrency suite, relational/polyglot live matrices, isolated strict-DB/feature boundaries, MSRV, the generated VM deployment built and started with Docker and rootless Podman (manual, nightly and deploy-changing pull requests), the first-hour documentation path run as written (manual, nightly and pull requests changing a marked page or the CLI), and a ready-PR/manual full-matrix SHA-bound per-crate quality scorecard artifact. A targeted manual OS/shard run is diagnostic and cannot emit the full scorecard. |
| [`cli-artifacts.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/cli-artifacts.yml) | reusable call from full/CLI manual Rust CI or admitted tag release | Blocking caller job; required release evidence | Builds and runs both CLI entry points on four explicit native targets; stages bounded executables and source/version/platform/digest inventories before attestation. CI artifacts are diagnostic; only the tag pipeline adds separate provenance and release assets. No installation occurs. |
| [`codeql.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml) | main/v12/v13 push, v12/v13 PR, weekly, manual | Blocking run | Rust CodeQL after an all-target/all-feature workspace check, plus JavaScript/TypeScript analysis. |
| [`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml) | main/v12/v13 push, v12/v13 PR, weekly, manual | Blocking plus observational job | LLVM LCOV generation with a pinned, zero-retry, bounded-concurrency nextest scheduler and retained JUnit inventory; a focused default-SQLite pass for ORM/Studio/Nexus/the facade; exact local 90% floors; and blocking OIDC-authenticated Codecov upload. Scheduled/manual branch instrumentation is non-blocking and uses the pinned verifier-only nightly. |
| [`crypto-containment.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/crypto-containment.yml) | relevant main changes and PRs, weekly, manual | Blocking | Signed OTA and Vault tests, RustSec audit, and simulator-boundary checks; explicitly no PQC/HSM certification. Until 13.0.0-alpha.4 this was `pqc-compliance.yml`; the `pqc` and `hsm` modules it scans are deterministic simulator fixtures. |
| [`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml) | manual | Blocking generated targets plus informational showcase | Pins the ZAP image by digest, scans fresh release/migrated REST API, SaaS starter (public and signed-in) and LMS surfaces as blocking gates, scans the CDN-backed blog showcase informationally, and uploads separate reports plus application logs. |
| [`documentation.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/documentation.yml) | main/v12/v13 push and PR, weekly, manual | Blocking plus informational external scan | Builds the mdBook; validates landing/benchmark templates, project identity, the README workflow count, the inventory below and every workflow badge or link, local assets, pinned external chart scripts and all requested social links. Real Chromium checks desktop/390px/320px layout, keyboard/mobile navigation, clipboard success/denial, privacy disclosure, reduced motion, no-JS navigation, and absence of external landing requests/browser storage. This is a bounded browser contract, not WCAG certification. Also validates the 190-claim historical roadmap denominator and repository-local links. Scheduled/manual runs preserve an informational external-link report. |
| [`e2e-smoke.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/e2e-smoke.yml) | main/v12/v13 push and PR, manual | Blocking | Boots the release Blog application and checks HTTP, headers, CSRF form flow, SQLite persistence, and the persisted page parsed by real headless Chromium. |
| [`fuzzing.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/fuzzing.yml) | manual | Blocking release campaign or diagnostic | Release mode maintains all 42 targets through independently verified, at-most-seven-day input-equivalent evidence or fresh 5.5-hour jobs. A package's inputs are the crates it reaches through path dependencies, files they include from elsewhere and a few global files; a change reruns the packages whose inputs it touches (all targets for global inputs, none for docs, changelog, CLI or examples), and a package whose inputs cannot be bounded falls back to the whole repository. Selected packages retain locked preflights, corpus caching and failure reproducers. `force_full` always requests a new full campaign; the current-commit boundary and independent tag-admission check remain mandatory. Lock drift is always rejected. The parser restarts its ASan process at most every 30 minutes while preserving the budget. Single-target diagnostic mode runs for five minutes and the evidence boundary marks it ineligible for release. |
| [`iot-integration.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/iot-integration.yml) | main/v12/v13 push and PR, manual | Blocking | Host IoT tests, signed OTA invariants, and one Cortex-M no-std build; no hardware claim. |
| [`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml) | weekly (Thursday 01:47 UTC), manual | Blocking release scope; bounded evidence | Builds an immutable reviewed Kani snapshot with the verifier-only `nightly-2026-08-01` compiler, while Rullst stays on stable Rust 1.99.0 with a Rust 1.96 MSRV. It verifies twenty named bounded harnesses in isolated jobs across ten supported packages. Proof failures fail their matrix jobs; the proc-macro-only crate remains outside Kani's supported targets. |
| [`machete.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/machete.yml) | main/v12/v13 push and PR, manual | Blocking | Unused dependency scan with configured exceptions. |
| [`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml) | weekly (Wednesday 01:17 UTC), manual | Blocking release scope; bounded evidence | Pinned nightly-only Miri executes 15 named pure-Rust/default-feature scopes with randomized layouts without changing Rullst's stable toolchain or MSRV. Native FFI/syscall/network paths, the umbrella re-export facade, and the Blog example are explicit boundaries; selected-scope failures fail the workflow. |
| [`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml) | manual | Informational | A source-bound inventory preflight followed by eighty lossless pinned cargo-mutants 27.1.0 shards with four concurrent jobs over that source's all-feature workspace, each shard with a pinned Redis service for the `RULLST_TEST_REDIS_URL` contracts, one validated production-file diagnostic, one owned-server contract (`mode=contract`: a PostgreSQL or Redis wrapper's adapter mutants against the server it owns, or the ORM crates against the `rullst-orm` SQLite suite), or policy-bound exact-SHA recovery of reviewed failed fragments. Successful immutable artifacts may be reused, but the content-addressed aggregate still requires every reviewed candidate exactly once and carries source/run/branch/tool/inventory provenance. Findings stay informational, while baseline/tool/invocation failures, missing artifacts, incomplete classification and reviewed-inventory drift fail the run. The completed v12 receipt is run `34761010296`: 14,391/14,391 classified and a conservative 70.57% caught. |
| [`mutants-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants-pr.yml) | pull requests to main that change Rust | Blocking by policy; not a required check | Lists the pinned cargo-mutants 27.1.0 all-feature mutants on the Rust lines the pull request changes, skipping the globs in `.github/mutants-pr-exclude.txt` and every CLI file not listed in `.github/mutants-pr-cli.txt` (listed CLI files run against the CLI's library tests), and tests them in at most eight shards with a Redis service and the targeted-run timeouts and memory cap. A missed mutant fails the run; unviable and timed-out mutants do not. See [Pull-request mutation gate](#pull-request-mutation-gate). |
| [`no_std-build.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/no_std-build.yml) | main/v12/v13 push and PR, manual | Blocking | Builds `rullst-iot` for three bare-metal targets; this is compile evidence, not hardware execution. |
| [`pages.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pages.yml) | main push, manual | Deploy | Validates and deploys the current development landing page, local visual assets, mdBook and benchmark hub/dashboards to GitHub Pages while preserving history data fetched from `gh-pages`. |
| [`pr-security-evidence.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pr-security-evidence.yml) | pull requests | Automated evidence | Generates bounded CLI audit, compliance report, and CycloneDX SBOM artifacts; no certification claim. Until 13.0.0-alpha.4 this was `ai-sentinel-pr.yml`; it calls no AI service. |
| [`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml) | weekly, manual | Blocking run | Eight parallel release-mode workspace shards plus dedicated ORM and Connect property contracts with configured case counts. |
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

## Verification efficiency — v12 maintenance and v13

**Priority: first, before the planned 12.1.0 updater and concentrated v13 product
development.** Compatible policy/tooling can serve both release lines after
review; it does not require a new framework capability release. The draft/ready split,
seven OS shards, Linux-only post-merge repetition, mutation recovery and
targeted diagnostic modes described above already exist. Measurement and
change-impact **observation tooling now exists on v13**. A narrowly scoped
development-only site admission path is being validated below; affected-crate
execution and general cross-commit release-evidence reuse are **not enabled**.

The `ci.yml`, `documentation.yml` and `workflow-lint.yml` development triggers
include v13, as do those of the other branch-filtered workflows except the
`main`-only benchmark, Pages, Scorecard and IoT cryptography workflows; these three jobs are not the whole release gate. The workflow-lint job retains a source/policy-bound
`rullst.verification-plan.v1` observation. It uses committed Git objects and TOML
metadata without executing Cargo, build scripts, macros or project tests. All
normal/optional/target/build/dev dependency edges participate in its reverse
closure. Renames, deletions, symlinks, unknown paths, policy/lockfile changes,
critical crates, generators and potentially executable Markdown retain the full
recommendation. Missing history or unsupported graph shapes also fall back to
full. `may_skip_checks` is always false and no workflow consumes this report to
skip a job. Candidate package lists are not a complete test/feature matrix.

The separate `admit-site-only.py` / `plan-ci-scope.sh` path can avoid repeating
Rust CI only for a **development `main` or transitional `v13` push** whose committed changes exclusively touch
`docs/home_template.html`, `docs/site.css` or `docs/site.js`. It requires the
immediately preceding commit to have a successful full Linux push CI in this
repository within 72 hours, on the same branch, with all 24 expected runtime jobs completed
successfully. Run/attempt/source/repository/branch identity and the entire job
inventory are checked. A skipped runtime job, unknown job, failure, missing
history, unavailable API, changed policy, symlink, mode change or stale receipt
retains full runtime checks. The source-equivalence check also prevents an
untested intermediate code change from being hidden by a later CSS-only push.

The helpers are loaded from the preceding committed source and executed with
isolated Python imports. A candidate cannot replace its own admission helper;
the shell's error path and every runtime job guard also retain full execution
if scope planning fails. Admitted pushes still build the book and run static
site validation and real Chromium tests, plus the separate documentation and
workflow checks. They do not produce a quality scorecard or qualify as release
evidence. PRs, `v12`, manual matrices, fuzzing, mutation campaigns and release
admission are unchanged. The first implementation deliberately does **not**
chain presentation-only receipts or search arbitrarily old baselines; a missing
immediate full baseline incurs a normal run.

Hosted rollout evidence on September 15, 2026: [full Linux push CI 35000185080](https://github.com/Rullst/Rullst/actions/runs/35000185080)
at `42dd0545` passed all 25 runtime jobs in 20m57s creation-to-completion.
The following [presentation-only CI 35002518137](https://github.com/Rullst/Rullst/actions/runs/35002518137)
at `3229b762` passed scope admission and fresh site validation in 43s, with
runtime jobs skipped as intended. Separate documentation and workflow checks
also passed. These are two observed Rust CI elapsed times, not a universal
speedup, a sum of all workflow durations, or permission to reuse release evidence.

Documentation and Pages builds install the same pinned mdBook version from its
prebuilt release instead of recompiling mdBook on each runner. Book, link and
browser checks remain enabled. Dependency changes still require runtime CI.

Before workspace Clippy, `check-fuzz-locks.sh` resolves all eleven fuzz dependency
graphs with `cargo metadata --locked` and the campaign's explicit Linux target.
It does not compile or execute fuzzers. This catches stale fuzz lockfiles after
workspace dependency updates without waiting for a long campaign build. Both
campaign preflights use the same full graph resolution; `--no-deps` is forbidden
because a real offline Cargo regression demonstrates that it accepts stale
path-dependency locks. Five tests cover that failure, the complete inventory,
early failure, argument rejection and CI wiring. Neither the forty-two-target
inventory nor campaign duration or release admission is reduced.

After formatting, `check-package-reproducibility.sh` packages `rullst-macros`
and `rullst-orm-macros` twice with `--no-verify`: once from the CI checkout and
once from a fresh worktree of the same commit at another path, with new file
times and umask `077`, each into an empty target directory. Different SHA-256
digests fail the job. It compiles nothing; the release preflight still compares
every admitted archive. See [reproducible crate archives](docs/src/reproducible-builds.md).

The threat-model runner now validates all evidence rows before downloads or
compilation and runs each unique exact test once, instead of first invoking
Cargo again to list that target. The current 66 evidence mappings, 50 threat
IDs and 65 unique tests therefore need one Cargo test invocation per unique
test instead of two, excluding each shard's retained locked fetch. Every
selected test still runs in a separate all-feature process on its deterministic
hash shard. Both a successful process exit and a bounded libtest log showing
exactly one named, passed, non-ignored test are required. An ignored or missing
test can no longer produce a false pass from libtest's zero exit status.

The LMS threat mapping (`ACADEMY-02`; v13 retired the other Academy mappings
with the complete Academy scaffold) selects
`security_tests::another_learners_lessons_are_refused` in the LMS example
(`rullst-lms-example`, `--lib`). Since 13.0.0-alpha.4 the LMS starter is that
workspace example, so the exact test needs no nested generated build; the
workspace shard also runs it with the example's other tests. The normal
foundation and product shards exercise the four reviewed generated
configurations: Blank, the hot-reload JSON API, hot-reload SaaS and SaaS's
release build.

Before 13.0.0-alpha.4, the mapping materialized the generated LMS starter
(`materialized_lms_executes_security_contracts`). That narrowing addressed an observed bottleneck: in [Rust CI 35007899011](https://github.com/Rullst/Rullst/actions/runs/35007899011),
the old all-blueprint wrapper consumed 958.22 seconds inside threat shard 0,
whose complete job took 26m12s. That gate needed LMS evidence, not another
execution of the seven other project configurations. Hosted threat jobs now
use the same bounded two-job nested compiler setting as the normal generated
matrix. Removing warm listing calls alone saves overhead, not half the
compilation time. The complete optimized Linux run passed all 25 required
runtime jobs, plus documentation and workflow validation.

| Measurement | [Before: 35011900532](https://github.com/Rullst/Rullst/actions/runs/35011900532) | [After: 35016405011](https://github.com/Rullst/Rullst/actions/runs/35016405011) |
| :--- | ---: | ---: |
| Required runtime jobs successful | 25/25 | 25/25 |
| Threat-model shard 0 execution | 28m34s | 10m22s |
| Generated-project threat wrapper | 1,063.37 seconds | 227.75 seconds |
| Summed measured runner time | 196.68 minutes | 147.55 minutes |
| Whole Rust CI elapsed time | 43m43s | 61m12s |

These source revisions are `0b3588a4` and `d0898516`. Execution work decreased,
but some jobs waited up to 51m40s between creation and start, so the whole
workflow took longer. Waiting can include orchestration and runner availability.
Cache warmth and overlapping workflows differ; this is not a controlled
benchmark or a fixed speedup guarantee. Do not increase shard counts blindly:
runner contention can outweigh shorter individual jobs. Complete and validate
one feedback batch before pushing another when practical; no required check is
cancelled merely to improve the displayed elapsed time.

The timing reporter leaves skipped jobs visible but does not assign them an
execution duration. GitHub can synthesize reversed timestamps for an unexecuted
job. Malformed timestamps, contradictory executed steps and negative intervals
in jobs that actually ran still fail. This observation-only correction does not
change site admission or release-evidence rules. The compatible v12 backport
must earn its own platform evidence; a v13 run is not a main acceptance receipt.

Six parser tests include a real compiled Rust harness; five runner tests check
the full exact inventory, shard partition and failure paths. Four generated
scheduling tests compile the actual selectors and wrappers with a recorded
verifier, proving the four-case partition without building applications
locally. That recorder is policy
evidence only: real generated application tests and full CI remain required.
These changes do not enable general fuzz-result reuse or relax release gates.

Single-target fuzz diagnostics now compile only the requested target in their
package preflight, rather than every other target in that package. The exact
target/package pair is checked again before Cargo; unknown, ambiguous or
option-like targets fail before compilation. Release mode still compiles every
target in all eleven packages before its forty-two 5.5-hour campaigns. Five
scheduling tests cover all forty-two diagnostic selections, the complete package inventory,
invalid inputs and compiler failure propagation. No sanitizer, instrumentation,
corpus or campaign duration is weakened, and diagnostics remain ineligible as
release evidence. Fuzzing and mutation campaigns remain manual, and Miri and
Kani run weekly besides their dispatched admission runs; an ordinary push does
not start any of them.

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

Initial timing observation: [Rust CI run 34980693742, attempt 1](https://github.com/Rullst/Rullst/actions/runs/34980693742)
reported head `792c1d554465c016abfdc223cbcaf7d94ec8c0c5` and 42 completed jobs.
The longest job was Windows/workspace at 30.05 minutes (27.83 in the combined
build/test step); the longest creation-to-start wait was 17.50 minutes.
Those waits can include orchestration, not just runner queues. The summed
386.72 runner-minutes are neither elapsed workflow time nor a billing estimate.
This one run is a baseline, not a measured speedup, cold/warm comparison or
evidence that compilation and test execution have been timed separately.

| Order | Improvement | Acceptance evidence |
| :--- | :--- | :--- |
| 1 | Measure the existing bottlenecks before changing scheduling | Record queue, restore, native compilation, Rust compilation and test durations separately, by OS and cold/warm cache. Compare equivalent inventories and runner conditions. Do not promise a fixed speedup. |
| 2 | Classify documentation, site, runtime, generator, dependency and verification-policy changes | Run the relevant book/link/browser/ledger tests for presentation changes. Markdown containing executable examples still needs its doctests. Unknown paths, renames, deletions and classifier failures fall back to broad verification; `.github/**` is never automatically a documentation-only exemption. |
| 3 | Select affected crates and reverse dependencies for development feedback | Unit-test the selection policy against runtime, macros, manifests, lockfiles, build scripts, generated templates, shared fixtures and security controls. Prove no required test or feature combination disappears before enabling skips. Keep full/manual and broad scheduled/release modes. |
| 4 | Reduce duplicate compilation and balance shards using measurements | Isolate trusted and untrusted caches; key compatible artifacts by toolchain, target, profile, features and dependency inputs. Preserve every case and assertion, report cache misses honestly and measure total runner time as well as elapsed time. |
| 5 | Reuse expensive verification only under an explicit admission policy | Bind receipts to source inputs, dependency locks, tool versions, flags, inventory, artifacts and policy. Failed, partial, diagnostic, expired or mismatched evidence cannot become a release pass. Keep conservative reruns for security-relevant or uncertain impact. |

The invariants remain unchanged: repository and library coverage floors stay at
90%; negative authorization and tenant-isolation tests remain enforced; no
timeout/crash becomes a tolerated success merely to shorten a run. A reduction
in redundant work must not be presented as stronger security proof.

Until a reviewed implementation changes the relevant contract, the current
`AGENTS.md` local baseline and exact-source release admission rules still apply.
Update automation, policy tests and this document together; a roadmap paragraph
alone must never authorize skipping a required check. Carry compatible fixes
from v12 to v13, without importing unrelated next-major features into v12.

## Preserved next-generation roadmap

These ideas remain valuable, but are not current guarantees:

| Idea | Status and recommendation |
| :--- | :--- |
| Verus proofs for selected production contracts | **Locally verified v13 candidate; hosted acceptance pending.** `verus.yml` is manual-only, with a pinned archive/compiler/solver, syntax-linked `AgePolicy::permits` projection, one verified predicate, three mandatory postcondition-failing controls and source/resource receipts. Normal CLI CI checks the source linkage without installing Verus. The pilot remains outside release admission; Auth, Capital, age evidence/time/storage and promotion remain separate work. See the [measured scope and assumptions](docs/src/verus-roadmap.md). No additional v12.1 gate. |
| Loom and Shuttle concurrency exploration | **Not implemented — worth implementing** for the small shared-state primitives that have explicit concurrency invariants. Do not apply them indiscriminately to the whole workspace. |
| `cargo-vet` dependency review | **Not implemented — worth implementing** once review ownership and audit criteria are defined; an empty policy file would add ceremony without assurance. |
| `cargo-careful` and zero-allocation assertions | **Not implemented — worth targeted experiments.** Allocation claims need stable benchmarks and explicit hot paths before becoming gates. |
| PGO and BOLT | **Not implemented — defer until production profiles exist.** Fixed throughput-gain percentages must not be promised in advance. |
| Chaos testing with `fail-rs` | **Not implemented — worth implementing** around queues, database retries, and provider timeouts after deterministic failure contracts exist. |
| AFL.rs/honggfuzz differential fuzzing | **Not implemented — valuable after the 42 libFuzzer targets have healthy corpora and triage ownership.** |
| Cross-platform CI acceleration | **Partial — first engineering priority, also applicable to compatible v12 maintenance.** The inherited v12 baseline has eight `cargo test` shards per OS and isolated fixture targets. Coverage separately uses nextest; the ordinary CI runner has not migrated to it. Preserve workspace, feature, generated-project, outbox, doctest and live-provider contracts while measuring further scheduling/cache improvements. Speed alone must never reduce assertions or supported-platform evidence. |
| Differential database testing | **Not implemented — high-value v13 work.** Run equivalent generated ORM operations against the supported relational backends and compare normalized results, errors and transaction behavior; keep provider-specific semantics explicit instead of forcing false equivalence. |
| Cross-browser and accessibility testing | **Not implemented — high-value v13 work.** Exercise generated applications with Playwright across Chromium, Firefox and WebKit, add keyboard and automated accessibility checks, and retain traces/screenshots for failures. This would complement, not replace, ZAP and server-level integration tests. |
| Mobile physical-device farms | **Not implemented — requires external infrastructure.** Add Android and iOS device-farm execution, lifecycle/network interruption scenarios and signed-package evidence when accounts and secrets are governed. Simulator and compile checks must not be presented as physical-device or store-acceptance proof. |
| Desktop GUI, installer and signing matrix | **Not implemented — requires platform signing operations.** Exercise real WebView sessions and packaged installers on supported Linux, Windows and macOS versions, then validate signatures/notarization without exposing signing credentials to untrusted pull requests. |
| Live provider sandbox conformance | **Not implemented — requires governed provider accounts.** Add opt-in acceptance suites for payment, mail, OAuth, AI, search and storage providers, recording provider/version/region while preserving deterministic offline fixtures as the contribution baseline. |
| Independent security review and penetration test | **External evidence — strongly recommended before broad production assurances.** Define scope, remediation ownership and retest criteria; repository automation cannot award this evidence to itself. |
| Sigstore Cosign signing | **Not implemented.** Consider it for separately distributed binaries/containers; current `.crate` provenance and checksums should remain the immediate priority. |
| Absolute “100% pure Rustls” mandate | **Not established and not recommended as a marketing absolute.** Enforce an audited TLS dependency policy based on supported platforms and threat model instead. |
| Complete upstream OSS-Fuzz integration | **Partial draft — worth finishing.** Validate every intended target with `helper.py build_fuzzers` and `check_build`, then submit upstream; do not imply acceptance before merge. |

### Cross-platform CI acceleration acceptance plan

The September 6 v12 candidate measurements record the pre-optimization baseline:
the all-feature workspace and follow-up contracts took approximately 70 minutes
on Linux, 59 minutes on macOS, and 107 minutes on Windows. Most time was spent
in the clean `cargo test --workspace --all-features` build. At that checkpoint,
only Cargo registry data was cached. Workspace `target/` caching was disabled
after CLI integration tests reused and cleaned nested target paths while the post-job
cache collector traversed them, producing false missing-file annotations and
multi-gigabyte uploads.

This is historical context, not the current configuration or expected duration.
The stable v12 baseline now uses the sharded `cargo test` system documented
above; pinned nextest currently schedules the coverage pass only. New
experiments must measure that inherited configuration, not claim a
speedup against obsolete commands or omit checks that moved into other jobs.

An acceleration change is acceptable only when all of these conditions hold:

1. Give repository compilation and every generated-project family distinct,
   explicit target roots. A generated fixture may clean only its own disposable
   root and must never mutate a cached workspace target.
2. Record cold and warm wall time, cache size/hit data, discovered test counts,
   failures and doctest results on Linux, macOS and Windows. Compare equivalent
   commit content; queue time is reported separately from execution time.
3. Preserve the all-feature workspace test inventory, portable transactional
   outbox contract and Linux live-provider matrices until an alternative proves
   identical coverage. Package sharding must not weaken Cargo feature unification.
4. Evaluate a pinned compiled-artifact cache and `sccache` independently before
   composing them. Keys must include OS, compiler, lockfile and relevant profile
   inputs; caches are performance hints, never release artifacts or evidence that
   tests ran. Bound storage and prevent untrusted pull requests from replacing a
   protected default-branch cache.
5. Evaluate nextest for ordinary CI (or change its coverage configuration) only if tests,
   ignored-test policy, retries, process cleanup and failure reporting remain
   equivalent. Run Cargo doctests
   separately because nextest does not replace them. Keep plain `cargo test` as
   a documented recovery path.
6. Promote the experiment to blocking CI only after repeated green cold and warm
   runs on every supported OS show a material wall-time reduction without new
   flakes, lost tests, hidden failures or multi-gigabyte cache churn. Retain the
   previous workflow as a quick rollback during the observation window.

For development-only presentation skips, comparing with the immediately
preceding commit is insufficient: that commit might have an unfinished or
failed runtime run which a new push cancels. Admission must bind a completed
successful runtime inventory to its exact baseline commit, compare that baseline
with the candidate, and reject incomplete, stale, diagnostic or presentation-only
receipts. Manual/release runs remain full. Do not infer receipt validity from a
green workflow badge, file extensions alone or the observation report.

This work optimizes feedback latency, not the evidence boundary. Any v12
maintenance change must preserve the immutable v12.0.0 artifacts and carry its
own applicable verification. Further scheduling/cache changes need comparable
cross-platform A/B receipts before their claimed benefit is accepted.

The goal of this roadmap is stronger, reproducible evidence—not a larger number
of badges or absolute claims that no finite test suite can establish.
