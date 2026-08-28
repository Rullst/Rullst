# Rullst Release and Branch Workflow

This document defines how development, maintenance, and releases move through
the repository. A branch name is not a publication or a security
certification. Official distributed artifacts are versioned crates.io packages
and their matching immutable tags.

## Permanent references

| Reference | Purpose | Published directly? |
| --- | --- | --- |
| `main` | Promoted release or maintenance source line; currently the legacy v5 baseline | No |
| `dev` | Active integration branch for unreleased v12 work | Never |
| `vX.Y.Z[-pre]` | Immutable source snapshot approved for release | Triggers the release workflow |
| crates.io `X.Y.Z[-pre]` | Official distributed artifact | Yes |

The current `main` source is based on the `v5.0.0` tag. Documentation or
repository-workflow maintenance on top of that source does **not** create
version `5.0.1`. A new crate version exists only after the manifests and
changelog are deliberately updated, the release gates pass, and the matching
tag is published.

Active v12 status and gates are maintained on
[`dev`](https://github.com/Rullst/Rullst/blob/dev/docs/src/v12.md). Until those
gates pass, v12 is unreleased and **NO-GO**.

## Normal development

New v12 work starts from `dev` and returns through a pull request targeting
`dev`:

```bash
git switch dev
git pull --ff-only origin dev
git switch -c feat/<short-topic>
```

Use Conventional Commits with a scope and a concise technical summary:

```bash
git add <paths>
git commit -m "feat(core): add typed request guard"
git push -u origin feat/<short-topic>
```

Pushes and pull requests to `dev` run the relevant day-to-day GitHub Actions,
including the core build, lint, test, panic, unsafe, spelling, platform, and
smoke-test checks. Workflows that are release-only, tag-only, scheduled,
manual, or tied to the default branch remain separate by design.

An explicitly approved v5 maintenance fix should branch from `main`, remain
small, and return through a pull request to `main`. Do not merge unreleased v12
work into a v5 maintenance change.

## Required local pre-flight

Before a change is declared complete, run:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all
```

Before a release candidate, also run the stricter all-target lint and every
release gate documented for that version:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Unimplemented roadmap behavior must remain explicitly identified and tracked.
It must not appear as an unexplained ordinary test failure or be described as a
completed capability.

## Promotion and release

1. Freeze feature work and synchronize crate versions and the changelog in
   `dev`.
2. Run all required local and GitHub release gates on the exact candidate SHA.
3. Open a reviewed promotion pull request from `dev` to `main`.
4. If the merge produces a different SHA, rerun the required gates on the final
   `main` SHA.
5. Create the release or prerelease tag only on that approved SHA:

```bash
git switch main
git pull --ff-only origin main
git tag vX.Y.Z
git push origin vX.Y.Z
```

A regular push to `main` or `dev` does not publish crates. Do not run a manual
multi-crate `cargo publish` sequence when the release workflow is available.
The tag workflow uses crates.io Trusted Publishing through short-lived OIDC
credentials; each published crate must have the repository workflow registered
as a trusted publisher. No permanent `CARGO_REGISTRY_TOKEN` is required for
that configured path.

## Current repository state

| Reference | State |
| --- | --- |
| `main` | Legacy v5 maintenance baseline; manifests remain `5.0.0` |
| `dev` | Active, unreleased v12 development; **NO-GO** |
| `v5.0.0` | Existing immutable historical tag; unchanged |

For production dependencies, prefer an exact crates.io version. If a Git
dependency is unavoidable, pin an immutable tag or exact commit rather than a
moving branch.
