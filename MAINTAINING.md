# Maintaining Rullst

This guide is for the maintainer and for any AI assistant that maintains the
framework. It records how the work is actually done: the operating rules of a
small workstation, how changes are packaged and merged, what each CI gate
expects and how to satisfy it, and how a release is admitted. It does not
replace the documents it links to:

| Topic | Source of truth |
| :--- | :--- |
| Framework behaviour and APIs | [`docs/src/spec.md`](docs/src/spec.md) |
| Coding rules for agents | [`AGENTS.md`](AGENTS.md) |
| Release phases and tags | [`RELEASE_GUIDE.md`](RELEASE_GUIDE.md) |
| Workflow inventory, evidence boundaries, accepted mutants | [`WORKFLOWS.md`](WORKFLOWS.md) |
| Plans and release lines | [`ROADMAP.md`](ROADMAP.md) |
| User-facing changes | [`CHANGELOG.md`](CHANGELOG.md), [`docs/src/migration-v13.md`](docs/src/migration-v13.md) |
| Governance and succession | [`GOVERNANCE.md`](GOVERNANCE.md) |
| Vulnerability handling | [`SECURITY.md`](SECURITY.md) |

When this guide and one of those documents disagree, the document in the table
wins; fix this guide.

## 1. Lines and versions

- `main` is the v13 line. Prereleases are published from reviewed tags
  (`v13.0.0-alpha.N`); a merge never publishes anything.
- `v12` reached end of life on 16 October 2026 with 12.3.0. The branch is
  frozen: do not push to it, port fixes to it or keep Dependabot entries for it.
- Every package in [`.github/release-order.json`](.github/release-order.json)
  shares one version. Prerelease internal requirements are exact
  (`=13.0.0-alpha.N`): `html!` in `rullst-macros` emits code that only the
  matching `rullst-core` understands, and a caret requirement would let Cargo
  pair different alphas. The CLI writes the same exact pin into generated
  projects.

## 2. Workstation operating rules

The maintainer's workstation has about 3.4 GiB of RAM and limited disk. These
rules come from real failures; follow them on any similar machine.

- **One Cargo command at a time.** Before starting one, wait until no `cargo`
  or `rustc` process runs (`while pgrep -x cargo || pgrep -x rustc; do sleep 20; done`).
  Use `CARGO_BUILD_JOBS=2` and `-- --test-threads=2`.
- **Shared target directory.** All worktrees use
  `CARGO_TARGET_DIR=<repository>/target`, so artifacts are reused. Cargo can
  occasionally run a test binary built from another worktree's copy of a crate
  (most visible with proc macros such as `rullst-orm-macros`). If a test fails
  in a way the source cannot explain, `touch <crate>/src/lib.rs` and rerun.
- **Temporary files.** `/tmp` is a small tmpfs: set `TMPDIR` to a directory on
  the main disk, and never build there.
- **Disk.** Follow AGENTS.md section 3.8: intervene at 15 GiB available, stop
  at 12 GiB. The usual reclaimable space is `target/debug/incremental` (safe to
  delete when no Cargo process runs), old `target/debug/deps` artifacts and
  worktrees whose branches are merged (`git worktree remove`).
- **Prefer hosted CI** for cold all-features builds, the generated-project
  suites, Windows and macOS, sanitizers and mutation runs. Locally, run the
  crate you changed with `-p` and a test filter.
- **Docker.** Tests that need PostgreSQL, MySQL or Redis are ignored or gated
  by `RULLST_*_URL` variables, and wrapper scripts in `.github/` start disposable
  containers in CI (for example `check-passkey-postgres.py`,
  `check-connect-sql.py`). Run them locally only when needed, remove the
  containers afterwards, and set `DOCKER_HOST=unix:///nonexistent` for ordinary
  test runs so nothing reaches a daemon by accident.
- **Long sessions.** Background shells can be killed under memory pressure.
  Run builds in the foreground, keep a running log of what was done, and push
  small commits so interrupted work can resume.

## 3. Packaging work

- **One package, one worktree, one branch.** Create a worktree per package
  (`git worktree add -b <type>/<topic> <dir> origin/main`) so parallel work
  never shares a checkout. Branch names follow the conventional-commit type:
  `feat/`, `fix/`, `test/`, `docs/`, `chore/`, `ci/`.
- **Commits** follow AGENTS.md section 3.10. Commit and push after each
  self-contained item.
- **Documentation travels with the change.** Update the spec section, the
  crate README or guide, and `CHANGELOG.md` under `## [Unreleased]`. A breaking
  change also gets a row in `docs/src/migration-v13.md` and, when the old usage
  can be detected in an application, a `cargo rullst upgrade` rule
  (`cargo-rullst/src/generators/build/upgrade/rules/`). The classification
  table and counts in
  [tutorial 36](docs/src/tutorials/36-assisted-framework-upgrades.md) must
  match the migration rows and the rule catalog.
- **CHANGELOG conflicts.** Parallel branches append to the same
  `[Unreleased]` subsections. When merging `main` into a branch, keep both
  sides' bullets and keep the subsection order already on `main`.
- **Merging.** Merge a pull request only when every check, including the
  mutation gate, is green on its latest commit. Branch protection does not
  require an up-to-date branch, so after merging one pull request, merge
  `main` into the others that touch the same files and let CI run again
  rather than merging a combination nobody tested.
- **Dependabot** groups compatible updates for the root and every fuzz
  lockfile (`.github/dependabot.yml`; `validate-fuzz-targets.py` checks the
  directory list). If Dependabot cannot rebase or recreate a pull request,
  apply the same `cargo update -p <crate>@<version>` in the root and the
  affected `*/fuzz` directories on a fresh branch, run
  `.github/check-fuzz-locks.sh`, and close the old pull request with a link.
- **Dogfooding reports.** Applications built on a prerelease are read-only
  inputs: read their reports, reproduce in this repository and add the items
  to the next prerelease plan in `ROADMAP.md`; never edit those applications.

## 4. CI gates and how to satisfy them

### Mutation gate on pull requests

`mutants-pr.yml` runs [`.github/mutants-pr.sh`](.github/mutants-pr.sh), which
mutates the production lines a pull request changes, with `--all-features`,
and fails on any surviving mutant.

- Kill a survivor with a test that fails under the mutation. Assert exact
  values with literals, never with the constant or function being mutated.
- Code compiled only without some feature is invisible to the gate; code in
  `*_tests.rs`, `tests.rs`, `tests/` and the paths in
  [`.github/mutants-pr-exclude.txt`](.github/mutants-pr-exclude.txt) is not
  mutated. Of `cargo-rullst`, only the files in
  [`.github/mutants-pr-cli.txt`](.github/mutants-pr-cli.txt) are mutated, and
  their mutants run against the CLI's library unit tests only, so a CLI test
  that kills one must live in the library (`src/`), not in `tests/`.
- A redundant condition produces an equivalent mutant (for example
  `a == X || b == X || a != b`, where the middle term never changes the
  result). Simplify the code instead of documenting the mutant.
- A proc macro's generated code is only exercised when another crate compiles
  it. Add a unit test in the macro crate that asserts on the generated tokens
  (`generate_…(&parsed).to_string()`), next to the existing ones.
- Tests that change process-wide state (environment variables, the current
  directory, global registries) go in their own integration-test binary.
- Every wait in a test needs a deadline, or a mutant that freezes a clock
  becomes a timeout.
- A truly equivalent mutant needs a row in WORKFLOWS.md "Accepted equivalent
  mutants" **and** its exact cargo-mutants name in
  [`.github/mutants-pr-equivalents.txt`](.github/mutants-pr-equivalents.txt).

### Other gates that catch maintainers out

- **IDOR audit** (`cargo rullst audit --idor`, run by `pr-security-evidence.yml` and
  the `scaffold_contracts` test): every route with a path parameter, test
  fixtures included, needs a `// rullst-access: public|owner|role|admin — reason`
  comment on the line above. `public` is accepted only when `get(` appears on
  the same line as the route path, so keep `.route("/x/{id}", get(handler))`
  on one line (use a named handler if rustfmt would split a closure).
- **Upgrade-rule fixtures.** New rules can add findings to unrelated test
  fixtures. Give fixtures exactly what their test is about (for example
  `default-features = false` on a `rullst` dependency that is not testing the
  default-features rule) instead of loosening assertions.
- **Site validation** (`.github/validate-site.py`): the README workflow counts
  and the WORKFLOWS.md inventory must match the files in `.github/workflows/`,
  and every badge or link to a workflow in the READMEs, WORKFLOWS.md,
  MAINTAINING.md and the book must name an existing file. Renaming, merging or
  removing a workflow means updating all of them.
- **Feature matrix** (`.github/check-feature-matrix.py`) and feature
  boundaries (`.github/check-feature-boundaries.sh`) must list every facade
  feature.
- **Advisories.** Every exception must appear in `deny.toml`, the ledger
  [`docs/src/security-advisory-exceptions.md`](docs/src/security-advisory-exceptions.md)
  and the `--ignore`/`--audit-ignore` flags of every workflow that scans
  advisories (`supply-chain.yml`, `crypto-containment.yml`, `release.yml` and
  `pr-security-evidence.yml`), with an expiry; the governance job in
  `supply-chain.yml` compares the three. Prefer an upgrade; an exception is
  temporary and time-bounded.
- **CodeQL.** Rust results go through
  `.github/filter-codeql-test-fixtures.py` before upload, which drops the
  listed rules in test code only. Real findings in production code are fixed,
  not filtered.
- **Windows and macOS** run nightly on `main`, on manual dispatch and release
  admission, and on a pull request labeled `full-matrix` (add the label when a
  change touches paths, processes or platform-specific code, so the matrix
  runs before merge). Avoid path-separator assumptions and POSIX-only file
  behaviour in tests; never `Box::leak` in tests (the AddressSanitizer job
  reports it).

## 5. Fuzzing evidence

Release mode of `fuzzing.yml` covers all 42 targets
(`python3 .github/validate-fuzz-targets.py`) in three waves of about five and
a half hours each, about 17 hours in total. Evidence from an earlier run is
reused only for targets whose inputs did not change. A fuzz package's inputs
are its build graph ([`docs/src/fuzz-evidence.md`](docs/src/fuzz-evidence.md)):
every crate it reaches through path dependencies, files those crates
`include_str!`/`#[path]` from elsewhere, and a few global files (root
`Cargo.toml`, toolchain, `.cargo/**`, every `.gitattributes`, inventory and
fuzz scripts). Docs, changelog, READMEs outside reached crates, `cargo-rullst`,
examples and other CI files no longer rerun anything; a change to a shared
crate reruns only the packages that reach it. A global or toolchain change
always reruns everything.

The rule guards against accidental drift (refactors, new dependencies, new
includes), not deliberately obfuscated file access: a maintainer who wanted to
skip fuzzing could simply not run it. It fails closed (that package's input
becomes the whole repository, the old behaviour) on patterns it does not
model: a build script or build-dependency, proc-macro I/O or a proc macro with
a path dependency or a registry dependency outside `syn`, `quote`,
`proc-macro2` and `toml`, known file-reading macros (`migrate!`,
`include_dir!`, `#[folder]`, …), a `..`, absolute or `$VAR` path passed to a
macro whose code is not scanned, and includes it cannot resolve. The receipt's
`whole_repository_inputs` says which package fell back and why. The fuzz
workflow also runs `.github/check-fuzz-build-graph.py`, which fails when
`cargo metadata --locked` resolves a path package, build script or proc macro
the scope misses.

Plan releases around this: freeze the code first, then start the release-mode
run on that exact commit. Adding a build script or build-dependency to a
fuzzed crate makes every package that reaches it rerun on any change until
the policy models it. If a reviewed proc-macro reader in `REVIEWED_COMPILE_IO`
(`.github/fuzz_build_graph.py`) changes, review the new blob and add it with a
test, or the packages that reach it keep falling back; do the same before
adding a registry macro to `REVIEWED_PLAIN_MACROS` or a crate to
`REVIEWED_PROC_MACRO_DEPENDENCIES`.

## 6. Mutation campaigns

Full measurements run on hosted runners with `mutants.yml`
(`gh workflow run mutants.yml --ref <branch> -f mode=targeted -f file=<path>
-f test_scope=package [-f shard=k/n]`). A dispatch from a branch runs that
branch's version of the workflow. Download each run's `outcomes.json`, list
the `MissedMutant` and `Timeout` entries, and kill them with tests as in
section 4. Measure `cargo-rullst` files with `-f test_scope=lib`, the scope the
pull-request gate uses for them; a file reaches the gate's
`.github/mutants-pr-cli.txt` once that run leaves only accepted equivalents. Record campaign status (what is measured, what remains) in
`ROADMAP.md`; mutants that only a live provider response reaches stay with
provider sandbox testing.

## 7. Releasing a prerelease

[`RELEASE_GUIDE.md`](RELEASE_GUIDE.md) is the procedure. In practice:

1. Prepare the release commit: version of every package in
   `release-order.json`, exact internal pins, the CLI's generated pins,
   `Cargo.lock` and every fuzz lockfile; fold `[Unreleased]` into the version
   section with a short summary and a `### Known limitations` subsection.
2. Show the maintainer the known limitations and get an explicit approval for
   this publication. An approval for an earlier release does not carry over.
3. On the exact candidate commit on `main`, every workflow in
   [`.github/release-required-workflows.json`](.github/release-required-workflows.json)
   must succeed: the push-triggered ones run on merge; dispatch the manual
   ones (`ci.yml` with all platforms and shards, `fuzzing.yml` in release
   mode, `dast-zap.yml`, `kani.yml`, `miri.yml`, `proptest.yml`,
   `sanitizers.yml`). `python3 .github/release-preflight.py v<version>` and
   `.github/check-release-admission.py` report what is missing.
4. Tag only when admission is complete. The release workflow packages,
   verifies and publishes in topological order; the `crates-io` environment
   waits for a reviewer's approval.
5. Never bypass a failing gate to publish. Fix the cause and admit the new
   commit. A tag whose release published nothing may be deleted and recreated
   only with the maintainer's approval; a published version is never
   reused — release the next version instead.
6. Record the publication in `ROADMAP.md` (release lines) and
   `docs/src/v13-delivery-plan.md`.

Upstream surprises happen: crates.io yanks, new RustSec advisories and new
toolchains have each blocked an admission. Check `cargo audit` and fresh
dependency resolution early in a release week.

## 8. What needs the maintainer

Assistants may commit, push work branches, open and merge pull requests whose
checks are green, and dispatch workflows. These need the maintainer's explicit
approval each time:

- publishing to crates.io or creating a release tag;
- deleting remote branches or tags, and force-pushing a shared branch;
- adding or changing an advisory exception;
- anything involving secrets, signing keys or repository settings.

Never put a secret in the repository, an issue or a pull request, and never
ask the maintainer to paste one into a chat.

## 9. Working with AI assistants

- Give each assistant one package with a written brief: worktree and branch,
  the items with file references and the evidence behind them, what other
  packages are changing at the same time (files to avoid), the verification
  commands, and the expected final report.
- Ask for a running log in a scratch file and small pushed commits, so a
  usage limit or a crash never loses work.
- Review every report before merging: the decisions an assistant made, what it
  could not verify locally, and the CI result of the merged combination.
- Usage limits are part of planning: estimate packages before starting them,
  and finish and merge work in progress before starting new work when a limit
  is close.
