# Fuzz evidence reuse

The v13 release requires all 42 declared targets across eleven fuzz workspaces,
including challenge-token and signed-attestation checks in `rullst-privacy`.
The immutable v12 release line retains its 40-target policy. A successful
`fuzzing.yml` run on the final release-branch commit remains mandatory (`v12`
for v12, `main` for v13). In release mode,
the planner can credit an original successful campaign when its reviewed inputs
match the candidate; only targets without eligible evidence execute again.
`force_full: true` requests every declared execution. Diagnostic mode remains a
five-minute, single-target check and cannot replace release evidence.

## What is compared

**Threat model.** The input rule guards against *accidental* drift: ordinary
refactors, new dependencies, new includes and new build-time code that a
fuzzed build would read without anyone noticing. It does not defend against
deliberately obfuscated file access (a path assembled from characters, a proc
macro computing an absolute path for the reviewed reader, …); a maintainer who
wanted to skip fuzzing could simply not run it. Within that model, unknown
patterns fail closed rather than being modelled.

`.github/fuzz_evidence_inputs.py` reads committed Git objects, not the dirty
working tree. A SHA-256 identity includes tracked paths, modes and blob IDs,
the pinned execution environment, preflight and campaign commands, runner
labels, tool/action versions, flags and each fuzz package's complete contents.
Each of the eleven separate fuzz workspaces retains its own dependency lockfile.

A fuzz workspace builds only its own crate and the workspace packages it
reaches through path dependencies, so its inputs follow that build graph
(`.github/fuzz_build_graph.py`). The rule prefers failing closed on the mere
presence of a pattern over modelling what it does:

- **The fuzz package and every reached package, whole.** Starting from the
  fuzz manifest, normal, dev, target-specific and `[patch]`/`[replace]`
  entries with a `path`, or `workspace = true` resolving to a path in the
  root `[workspace.dependencies]`, are followed transitively. Every tracked
  file below each reached directory is an input, without exceptions: sources,
  tests, manifest, assets and any fuzz directory nested in it. A fuzz
  package's lockfile, harnesses and corpus therefore also invalidate the fuzz
  packages that reach its parent crate.
- **Files a reached package includes from outside its directory.** Every Rust
  file of a reached package is scanned for `include_str!`, `include_bytes!`,
  `include!` (including `concat!(env!("CARGO_MANIFEST_DIR"), "/…")`) and
  `#[path]` (also inside `cfg_attr`; `r#` forms included), with literal paths
  forwarded through a `macro_rules!` resolved at each call site. Those targets
  become inputs of the fuzz packages that reach the referring package, and
  included code is scanned again. Today this adds the 93 book pages of
  `rullst/src/book_doctests.rs` and six `.github/fixtures` sources of the
  facade's tests to `rullst/fuzz`.
- **Global inputs of every package.** The root `Cargo.toml`,
  `rust-toolchain(.toml)`, `.cargo/**`, every `.gitattributes`, the target
  inventory, release-line policy, `.github/validate-fuzz-targets.py`, every
  other `.github` path the fuzz workflow names (and sibling modules it
  imports), and the workflow's execution contract.

Anything outside those sets is not an input: `docs/**`, `CHANGELOG.md`,
`README.md` and other Markdown outside reached packages, `cargo-rullst/**`,
`examples/**`, the remaining crates, the root lockfile (the eleven fuzz
workspaces keep their own locks) and other `.github/**` files, unless a
reached package includes them. The reviewed non-input list only matters for
the global set and for a scope that fails closed.

All 42 targets still run after a global-input or execution-contract change,
with `force_full`, or when no eligible earlier campaign exists. A change in a
shared crate reruns the packages that reach it: `rullst-core` or
`rullst-macros` 7 packages (29 targets), `rullst-orm` or `rullst-orm-macros`
8 packages (34 targets). A doctest-included book page reruns `rullst/fuzz`
(12 targets).

### When the scope fails closed

The whole repository (minus the reviewed non-inputs, the previous rule)
becomes a fuzz package's input when any of these is present in its graph:

- **Build-time code:** a build script (`build.rs` or `package.build`) or any
  build-dependency in a reached package. None exists today.
- **Proc-macro I/O:** a reached proc-macro crate (`proc-macro = true` or
  `crate-type = ["proc-macro"]`) whose compiled files, including `#[path]`
  and `include!` targets outside it, use filesystem, process or environment
  APIs (`fs`, `File`, `Command`, `read_dir`, `exists`, `is_symlink`,
  `metadata`, …) or emit computed includes, unless the exact blob is reviewed
  in `REVIEWED_COMPILE_IO`; that holds a string with a `..` segment that is
  not an include path or a `==`/`!=` operand; that has any path dependency
  (a helper crate would hide its I/O from review); or that links a registry
  crate other than `syn`, `quote`, `proc-macro2` and `toml`
  (`REVIEWED_PROC_MACRO_DEPENDENCIES`, what `rullst-macros` and
  `rullst-orm-macros` use; dev-dependencies are not linked and not checked).
  Only the `rullst-macros` i18n `catalog!` reader is reviewed: it reads
  `<invoking package>/<dir>/*.toml`.
- **Known file-reading macros and attributes:** `migrate!`, `query*!`,
  `include_dir!`, `embed_migrations!`, `include_proto!`, `dotenv!`, `#[folder]`
  (rust-embed), `#[grammar]` (pest), `#[template]` and similar, also as an item
  of `cfg_attr(…)` or when renamed with `use … as …`.
- **Path-like macro arguments:** a string with a `..` segment, a leading `/`
  or a leading `$VAR`/`${VAR}`/`%VAR%` placeholder, when any macro call or
  attribute enclosing it is not trusted. Each `cfg_attr(…)` item counts as an
  attribute of its own, and names resolve through the file's `use` imports.
  Trusted are: built-ins (formatting, assertions, includes, `quote!`) only when
  unqualified or written as `std::`/`core::`/`alloc::` (`quote::`/`syn::` for
  quote's macros), so `#[sqlx::test(migrations = …)]` is not `#[test]`; a
  `macro_rules!` defined in the scanned sources, called unqualified or through
  a workspace crate path, that passes its arguments only to built-ins; and the
  reviewed registry macros `serde_json::json!` and `libfuzzer_sys::fuzz_target!`
  by full path. Routes and placeholders (but never `..`, e.g.
  `catalog!(dir = "../x")`) are also allowed in the workspace's own proc macros
  and attributes, whose I/O is checked above. An untrusted macro call that
  contains a call to a local `macro_rules!` holding a path-like literal fails
  closed too. Plain string literals in code, such as WAF traversal patterns,
  are not affected.
- **Includes that cannot be resolved:** computed, absolute, with a backslash,
  leaving the repository, or with a `..` after a placeholder component
  (`{dir}/..`, `$VAR/..`, `%X%/..`); `CARGO_MANIFEST_DIR` paths inside a
  macro or in code included from outside every package; `OUT_DIR` paths that
  climb; generated includes that climb; a renamed built-in include
  (`use std::include_str as …`); a path inside a `macro_rules!` that is
  renamed with `as`, called from `quote!`, forwarded from another macro, or
  given an argument containing a macro call; a macro that defines macros
  (`macro_rules! $name`); a module tree at the repository root; more than
  eight levels of nested inclusion; source that cannot be lexed.
- **Manifests and configuration:** a path dependency without a package
  manifest or outside the repository; a target outside its package; workspace
  inheritance without a workspace root or from a workspace that excludes the
  package; malformed dependency or target tables; Cargo configuration other
  than aliases, build jobs/flags, profiles, terminal or network settings.
- **Run-time access in a fuzz harness** to files, processes or the
  environment.

The planner's receipt lists every package that fell back and why. Every
other manifest field, including target-specific and normal dependencies,
remains an input; only the top-level `dev-dependencies` table of
`rullst-mail/Cargo.toml` is normalized out, as before.

### What is trusted

Third-party crates are pinned by each fuzz workspace's lockfile. Their build
scripts and proc macros are trusted to read only their own package; the scan
covers workspace sources, not registry code, and the known-macro list above
is not exhaustive. Package-local run-time reads resolve from the fuzz
directory; library code reading repository files through other paths at run
time is not modelled. `.github/check-fuzz-build-graph.py` runs in the fuzz
workflow's inventory job and fails if `cargo metadata --locked` resolves a
path package the scope does not reach, a build script the scope did not fail
closed on, a proc macro it does not check, or a proc macro linking a path
package or an unreviewed registry crate.

Accepted and out of scope under the threat model above: unreviewed proc-macro
code computing an absolute path and handing it to the reviewed reader
(`expand_in` stays crate-visible because the production `catalog!` entry calls
it); a literal laundered through a local macro so that it sits next to
`==`/`!=`; an attribute macro reading a literal from the item it annotates;
`#[link]` library paths; and `..` assembled from characters. The regular locked-resolution,
all-feature CI and CodeQL checks still run on the changed candidate.
Unsupported Git layouts, missing history or malformed evidence stop
verification.

### Reviewed v12.1 publication documentation

The two documentation reviews below predate the build-graph rule. They still
normalize their exact blobs where those files remain inputs (crate READMEs
inside reached packages and book pages included by the facade's doctests).

The publication review in commit `c3417135` changes 48 README, book and public
site files relative to `44902312`, without changing runtime source, build
scripts, dependency manifests, fuzz harnesses, locks or execution commands.
`.github/fuzz-reviewed-publication-docs.json` records the exact old/new Git blob
IDs by path. Only those two contents, with their original regular-file mode,
share an input identity; a third blob, removal, rename, mode change or new
document remains an input change. This is not a documentation-directory
exclusion. The review table itself is trusted policy included in the receipt's
policy digest.

The changed README files are Cargo package metadata, not runtime inclusions.
The book's Rust snippets are included only by `#[cfg(doctest)]`; fuzz binaries
do not compile or execute these doctests. The site template, release banner
and two site validators run in the separate documentation/browser checks.
The Core error-console test mentions `README.md` to reject an unsupported
source-file extension; it does not feed README contents to a fuzz harness.
The exact changed contents were reviewed against these consumers. A new
production consumer changes shared source and invalidates all targets.
Package/readme audits, browser checks and doctests still validate the changed
candidate; this exception only concerns the bounded fuzz campaign.

## Which results qualify

The planner examines at most 30 recent runs and credits only original jobs:

- The same repository, candidate release branch, `workflow_dispatch`, and
  `fuzzing.yml` identity. The source's committed release policy must identify
  that same branch; results do not cross the v12/v13 release boundary.
- A source commit that is the candidate or a Git ancestor of it.
- A run created within the preceding seven days; reusing it does not renew age.
- A completed successful workflow and evidence boundary, a successful package
  compile preflight, and the unique successful target job from one exact attempt.
- A successful `Run bounded target campaign` step lasting at least 19,800 seconds.
- Equivalent package input and execution identities, independently recomputed.

A newer matching failed, timed-out, cancelled or running target prevents falling
back to an older success. Short diagnostics, absent/skipped targets, duplicate
job names, incomplete API pagination, foreign repositories/branches, and
unrelated commits receive no credit. Missing original evidence requires a new
execution; a reuse receipt is never a substitute for the original job.

## Publication and receipts

The current campaign emits a receipt with the candidate, policy/input digests,
newly selected targets and each reused target's original run, attempt, job,
source commit and completion time. The GitHub summary links to those runs.

The tag-only release admission check first requires a successful current-commit
fuzz workflow. If some target jobs were reused, it independently reads Git and
GitHub again and requires complete coverage of the release line's reviewed inventory; it does not trust the
uploaded receipt. It retains `release-fuzz-evidence.json` for that decision.
Expired or newly invalidated evidence blocks publication and requires another
campaign. The other 27 required workflows retain their existing exact-SHA rules.

This is reuse of bounded historical testing, not proof that every input is safe
or that another randomized campaign would find nothing new. It compares tracked
build/run inputs and pinned tool versions, not bit-identical hosted runner
images or external state. Corpora can evolve and hosted runner images change.
The seven-day limit bounds age; maintainers can always request a fresh complete
campaign. The reviewed policy code remains part of the release trust boundary.

### Complete-context publication documentation review

The development transition from `a8a78aa2` to `181dd8a2` changes publication
provenance, spelling configuration and nine exact documentation paths. It
changes no runtime source, manifest, compiler configuration, fuzz harness,
dependency lock or campaign execution command.

`.github/fuzz-reviewed-context.json` binds these exact old/new regular-file
blobs to a SHA-256 of all other tracked paths, modes and blobs, the `main`
release branch and the fuzz execution contract. Its review is independent of
the stable `v12` table. A small explicit control-plane list excludes only the
review machinery, this document, workflow lint and release reporting. The fuzz
workflow retains its execution contract while its scheduler and verifier remain
reviewed policy. The root lockfile and existing admission policy are also pinned.

Only the complete before or after documentation state receives normalization.
Mixed states, later edits, deleted files, changed modes, another branch, unknown
files or changed consumers receive no exception. New governance and Scorecard
documents map to their former absence only with their exact reviewed contents.
The original committed files remain available for source/dependency discovery.
The reviewed root README and book material is metadata or doctest-only content;
none of these changed documents is consumed by the current fuzzed APIs. A new
consumer invalidates this profile even after passing its own campaign.

The review engine, table and tests are included in the receipt's policy digest.
Real Git fixture tests cover altered source, manifests, locks, macros, build
scripts, configuration, document states, execution contracts and malformed
reviews. Provenance, original seven-day expiry, the full 19,800-second duration,
newer-failure protection and a successful final-source evidence workflow remain
mandatory. All 42 development targets require their own eligible evidence;
v12 campaigns cannot certify v13. This review alone grants no release admission.
