# Compatibility, MSRV, deprecation, and support policy

This policy applies to the 16 publishable Rullst packages. They are released as
one synchronized release train even when a user depends on only one crate. The
current supported-version table in [`SECURITY.md`](../../SECURITY.md) remains
authoritative for releases that are actually available; a version in the
workspace is not a supported release merely because its manifest exists.

## Source branches

The permanent branches are `main` (unpublished v13 development), `v12`
(supported 12.x maintenance) and `gh-pages` (generated site/benchmark data).
Use short-lived feature, fix and Dependabot branches with a pull request to the
appropriate protected base. Remove those temporary branches after their changes
are integrated; a superseded PR must identify the replacement preserving its
changes. The former `v13` integration branch is retired. Historical releases
remain available through immutable tags and crates.io archives.

Select a published version for application dependencies; a branch name is not
a release. Stable and development branches have independent commit histories:
ahead/behind counts are not a measure of security, freshness or missing fixes.
Do not reset or merge whole release trains merely to make those counters zero.

### Carrying fixes between maintained lines

Every stable maintenance fix must be assessed for `main`. Before closing its
maintenance item, record the linked forward-port PR, the commit where the fix
already exists, or a specific reason it does not apply. Carry the regression
and relevant documentation with the fix, adapt it to the destination API and
run that branch's required checks before integration. Conversely, evaluate
confirmed v13 fixes for the existing v12 surface without importing new v13
features or breaking the stable API/MSRV.

Versions, release records, feature inventories and branch-specific documentation
stay appropriate to each release train. Dependency updates require the same
assessment, but resolve and validate each branch's own graph; do not copy a
whole lockfile across diverged workspaces. Prefer a linked, focused port over a
blanket merge. A briefly pending port must stay explicitly tracked rather than
being hidden by commit counts. This policy does not establish an indefinite LTS
commitment or alter the support window below.

## Semantic Versioning contract

Rullst follows Cargo Semantic Versioning for stable releases:

- **Patch (`x.y.Z`)** releases contain compatible bug, security, documentation,
  and dependency fixes. They do not intentionally remove public APIs, public
  Cargo features, CLI commands, or accepted configuration fields.
- **Minor (`x.Y.z`)** releases may add compatible APIs, opt-in features,
  diagnostics, and deprecations. Changes to defaults or generated projects must
  include migration notes and compatibility tests.
- **Major (`X.y.z`)** releases may contain breaking changes. Each known break
  must be listed in the changelog and migration guide.
- **Prereleases** such as `12.0.0-rc.1` are public evaluation artifacts. Their
  APIs may still change in a later RC, and users must opt in explicitly.

The compatibility surface includes documented public Rust APIs, public Cargo
feature names, CLI command and flag identifiers, supported configuration keys,
serialized public data contracts, and generator output relied on by the
packaged-distribution tests. Undocumented internals, exact dashboard HTML/CSS,
test fixtures, deterministic mocks, and third-party provider behavior are not
stable interfaces.

## Minimum Supported Rust Version

The v12 release line declares **Rust 1.96.0** in every publishable manifest.

- Patch releases do not raise the MSRV.
- A minor release may raise it only with an explicit changelog entry, updated
  manifests and documentation, and a green MSRV CI job for the release commit.
- A major release may select a new baseline, which must be announced in its
  migration guide.

The MSRV promise covers the supported feature boundaries exercised by CI. A
nightly-only analysis tool or an experimental target does not change the crate
MSRV and must be labelled separately.

## Deprecation and removal

A stable public API scheduled for removal is marked with `#[deprecated]` and
kept for at least one released minor version before removal in the next major
release. Documentation must name the replacement when one exists.

An API that is unsound, enables a security bypass, or cannot be made safe may be
disabled or removed sooner. Such an exception requires a security advisory or
changelog entry describing impact, affected versions, and the supported
migration path. Compatibility never overrides safety or fail-closed behavior.

Removing a public Cargo feature is a major change. Adding an opt-in feature is
normally minor-compatible. Changing default features is treated as an
operationally significant minor change and requires package and generated-app
regression tests.

## Supported release window

Rullst does not currently promise an LTS or multi-minor backport program.
Routine fixes target the latest patch of the latest supported stable minor. The
exact versions receiving security triage are listed in `SECURITY.md`; older
versions are end-of-life unless that table explicitly says otherwise.

Release candidates receive fixes through a subsequent RC number rather than by
overwriting a published artifact. Security reports follow the coordinated
disclosure and response targets in `SECURITY.md` and the
[advisory-exception policy](security-advisory-exceptions.md).

## Release evidence

Before publishing a stable or prerelease version, the release commit must prove
the applicable compatibility contract with:

1. synchronized package versions and internal requirements;
2. the MSRV and multi-OS CI jobs;
3. full tests, strict Clippy, formatting, docs, and feature-boundary checks;
4. packaged crates and generated applications tested without monorepo paths;
5. a changelog and migration notes for every intentional behavior change.

Passing these gates is evidence for the exact commit and declared matrix, not a
guarantee about untested platforms, external providers, or downstream
application code.
