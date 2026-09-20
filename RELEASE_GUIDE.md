# 🚀 Rullst — Release & Development Workflow Guide

> This document explains the official process for developing new features, testing them safely, and releasing stable versions of Rullst to crates.io without breaking things for users.

---

## 🧠 The Core Concept

**The golden rule: release majors have explicit protected source lines:
v12 uses `main`, and v13 uses `v13`.** Normal work is reviewed through short-lived branches targeting
the appropriate release line. A branch name is not a publication or a security certification.
Official release artifacts remain crates.io packages and their matching
immutable tags.

The repository separates maintained releases, next-major work, historical
source and generated site data:

| Reference | What it is | Published to crates.io? |
|--------|------------|------------------------|
| `main` | Protected source for v12 maintenance releases | Only after an approved release tag |
| `v13` | Protected next-major development and v13 release source | Only after its own admitted release tag |
| `v5` | Frozen source snapshot of the legacy v5 line | No; use the existing `v5.0.0` tag/crate |
| `gh-pages` | Generated website/benchmark history used by Pages | No |
| `feat/*`, `fix/*`, etc. | Short-lived reviewed work | Never directly |
| `vX.Y.Z[-pre]` | Immutable source snapshot approved for release | Triggers the release workflow |
| crates.io `X.Y.Z[-pre]` | Official distributed artifact | Yes |

---

## 📋 The Full Release Cycle (Step by Step)

### Phase 1 — Select the maintenance or development line

Compatible v12 work starts from the latest green `main` and returns through a
pull request targeting `main`. Product features and breaking work start from
`v13` and target `v13`. Keep branches short-lived and do not stack work on a
broken required gate. The example below is for v12 maintenance:

```powershell
# Synchronize main before starting any new work
git switch main
git pull --ff-only origin main
git switch -c fix/<short-topic>
```

Make your changes, bug fixes, new features, etc.

```powershell
# Commit your work as usual
git add .
git commit -m "fix(scope): describe the correction"
git push -u origin fix/<short-topic>
```

Every push to `main` or `v13` and every pull request targeting either triggers the relevant
CI. Checks are classified so unfinished roadmap work does not make every
development signal meaningless:

- **Development baseline:** formatting, compilation, strict Clippy, tests for
  implemented behavior, and current panic/unsafe/security invariants should
  remain green. A broken current contract is fixed before more work is stacked.
- **Readiness evidence:** coverage, benchmarks, broader platform matrices and
  exploratory future-version scenarios may be observational while their named
  roadmap item is open. Failures stay visible and tracked; they are not
  silently presented as passing.
- **Release gates:** before an RC or stable tag, every required check in the
  current release policy and `WORKFLOWS.md` must pass on the exact candidate
  commit. The [v12 stable record](docs/src/v12.md) preserves the completed v12
  release identity rather than serving as a future checklist.

A test for a capability that is deliberately not implemented yet must be tied
to a roadmap item and explicitly quarantined with a reason. It must not
remain an unexplained ordinary test failure until the end of the release cycle.

---

### Phase 2 — Verify Stability

Before releasing, make sure:

- [ ] All required release checks have been rerun and are ✅ **green** on the
  exact candidate commit:
  - `ci.yml`: Multi-OS test matrix (Ubuntu, macOS, Windows MSVC), isolated
    feature boundaries, MSRV and live provider matrices on Linux.
  - `coverage.yml`: exact-SHA line and patch coverage with the configured 90%
    repository/framework/component gates and a blocking Codecov upload.
  - `proptest.yml`: release-mode invariant suites with 10,000 configured cases.
  - `kani.yml`: Model checking for the explicit harnesses and configured bounds;
    this is not a proof of every path in the workspace.
  - `sanitizers.yml`: ThreadSanitizer (`TSan`) and AddressSanitizer (`ASan`) for
    the targets declared by the workflow.
  - `miri.yml`: Undefined-behavior checks with randomized layouts for its 15
    declared pure-Rust scopes.
  - `fuzzing.yml`: Bounded libFuzzer runs over the 40 validated targets. The
    separate `oss-fuzz/projects/rullst` directory remains an unsubmitted local
    integration draft and is not release evidence.
  - `dast-zap.yml`: Blocking ZAP baselines for freshly generated REST API and
    LMS applications plus the explicitly informational blog showcase.
  - `e2e-smoke.yml`: Live SSR, security-header, CSRF and SQLite-persistence
    checks against the release-built Blog application.
  - `omni-android.yml`, `omni-desktop.yml`, and `omni-ios.yml`: fresh
    deterministic shell generation and the declared hosted compile matrices.
    These are not physical-device, signing, store-review or GUI acceptance
    evidence.
- [ ] You have manually verified the mandatory local trifecta:
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-features -- -D warnings`, and
  `cargo fmt --all`.
- [ ] `CHANGELOG.md` has a detailed release section describing all additions and fixes.
- [ ] The [compatibility and MSRV policy](docs/src/compatibility-policy.md) still
  matches the manifests, supported-version table, and intended release changes.
- [ ] The [Cargo feature matrix](docs/src/feature-matrix.md) still matches every
  publishable manifest and the feature-boundary CI matrix.
- [ ] The [v12 migration guides](docs/src/migration-v12.md) and
  [AI capability matrix](docs/src/ai-provider-capabilities.md) match the APIs,
  CLI behavior, and known release-history boundaries.
- [ ] Every current security statement matches the code, test, and limit in the
  [v12 security claims ledger](docs/src/v12-security-claims.md); do not promote
  an unlisted or unevidenced statement into release notes.
- [ ] The packaged
  [security-event v1 JSON Schema](rullst-security/schema/security-event-v1.schema.json)
  matches `LiveSecurityEvent`, and any incompatible event change uses a new
  schema version instead of silently changing v1.
- [ ] All 16 publishable crate `Cargo.toml` versions and internal requirements
  are synchronized at the selected new release version:
  - `rullst-macros`, `rullst-orm-macros`
  - `rullst-core`, `rullst-orm`, `rullst-auth`, `rullst-security`
  - `rullst-ai`, `rullst-capital`, `rullst-connect`, `rullst-messaging`, `rullst-iot`, `rullst-mail`
  - `rullst-studio`, `rullst-nexus`
  - `cargo-rullst`, `rullst`
- [ ] Review the README extracted from each `.crate`, installation examples and
  public demo links before creating the tag. The facade and CLI package the root
  README. Run both `rullst --version` and `cargo rullst --version` from the staged
  native artifacts and check a generated project's framework dependency version.
  A crate version already uploaded to crates.io cannot be overwritten to correct
  its packaged documentation.

---

### Phase 3 — Freeze the release branch + Create a Tag

Once everything is stable and verified:

1. Freeze feature work and prepare the synchronized version change through a
   reviewed pull request into the selected release branch (`main` for v12,
   `v13` for v13).
2. Run the full local and CI release gates on the resulting release-branch SHA.
3. Record and review the package/evidence artifacts for that exact SHA.
4. Create a new version tag only on the approved SHA, then push that tag to
   trigger the release workflow. Published tags, including `v12.0.0` and `v12.1.0`, are immutable;
   never recreate or move them. For a reviewed v13 candidate:

```powershell
git switch v13
git pull --ff-only origin v13
git tag v13.X.Y
git push origin v13.X.Y
```

An RC or stable version is a real public crates.io release. It can be yanked but
never overwritten; inspect and test every `.crate` before pushing the tag.
Prereleases require explicit opt-in with a requirement such as `13.0.0-rc.1`.

GitHub Actions will automatically execute the topological crate publish pipeline:
1. ✅ `rullst-macros` & `rullst-orm-macros`
2. 📦 Foundations: `rullst-orm`, `rullst-core`, `rullst-messaging`
3. 📦 Domain crates: `rullst-connect`, `rullst-iot`, `rullst-security`, `rullst-ai`, `rullst-capital`, `rullst-mail`, `rullst-auth`
4. 📦 Dashboards: `rullst-nexus`, `rullst-studio`
5. 📦 Main bundle & CLI: `rullst`, `cargo-rullst`

Before publication, the workflow extracts the exact version section from
`CHANGELOG.md`, packages and reproduces every archive, writes checksums and the
tag-bound evidence bundle, and creates a SHA-pinned GitHub build-provenance
attestation. The GitHub release uses those extracted notes and is marked as a
prerelease automatically when the semantic version contains a prerelease
suffix. This evidence does not establish a project-wide SLSA level or an
independent certification.

---

### Phase 4 — Continue v13 while maintaining v12

The `v13` branch already exists. Start next-major work from that branch after
reviewing its roadmap and differences from the published v12 source:

```powershell
git switch v13
git pull --ff-only origin v13
git switch -c feat/<short-topic>
```

Carry applicable v12 fixes forward through reviewed changes. Evaluate
Dependabot updates individually: a dependency's major version does not by
itself prove that Rullst's public API must break. Compatible fixes may ship in
`12.x`; changes that break Rullst's compatibility contract belong to v13.
Keep the v12 release gates active while v13's own CI policy evolves.

Version 12.1.0 is published. Carry its compatible
[update experience](ROADMAP.md#safe-update-experience) into v13 while keeping
maintenance on `main`; do not merge the entire v13 branch into `main`.
Synchronize package versions only when the candidate is accepted for release.
A major upgrade needs its own tested migration rules, not just an updated
installer. Follow the [v13 delivery plan](docs/src/v13-delivery-plan.md) for
current priorities. This plan does not itself authorize publication or end
v12 maintenance.

---

## 🔄 Visual Summary

```
short-lived branches ── reviewed pull requests ──▶ main (v12) / v13 (v13)
                                                   │
                                                   │ exact approved SHA
                                                   ▼
                                            git tag vX.Y.Z
                                                   │
                                                   ▼
                   🤖 GitHub Actions CI
                   runs all tests...
                             │ if ✅ all green
                             ▼
                   📦 verified publish workflow
                       crates.io X.Y.Z
```

---

## ⚠️ Important Rules

> [!CAUTION]
> **Never** run `cargo publish` manually from a workstation. The protected tag
> workflow verifies and publishes the inspected package artifacts in dependency
> order. A workflow name alone is not evidence; the exact release run must pass.

> [!WARNING]
> Keep both release branches green and protected. Normal changes arrive through reviewed,
> short-lived branches; emergency direct pushes require the same evidence and
> must not bypass repository rulesets.

> [!IMPORTANT]
> The automatic publishing only triggers when you push a **version tag** (e.g., `v1.0.5`). A regular branch push does **NOT** publish to crates.io.

---

## 🔑 One-time GitHub Setup Required

All sixteen registered v12 crates use crates.io Trusted Publishing through
GitHub OIDC. The protected `crates-io` environment must require review and be
configured for `release.yml`. The first-publication bootstrap token has been
revoked and its GitHub secret removed; the bootstrap allowlist is empty. A
future new package name requires the narrowly scoped, short-lived procedure in
[`docs/src/release-recovery.md`](docs/src/release-recovery.md). Do not maintain a
permanent repository-wide registry token.

---

## 📌 Quick Reference Commands

```powershell
# Start a compatible v12 maintenance change (use v13 for next-major work)
git switch main
git pull --ff-only origin main
git switch -c fix/<short-topic>

# Check status before releasing
git status

# After the candidate commit is approved on main
git switch v13
git pull --ff-only origin v13
git tag v13.X.Y
git push origin v13.X.Y
```

---

## 🗺️ Current State

| Item | Version |
|------|---------|
| `rullst` | Check `rullst/Cargo.toml` |
| `rullst-macros` | Check `rullst-macros/Cargo.toml` |
| `cargo-rullst` | Check `cargo-rullst/Cargo.toml` |
| Current `main` line | v12 stable maintenance after the approved tag; new feature work belongs on the v13 line |
| Legacy source | Frozen `v5` branch and immutable `v5.0.0` tag |
| Published prerelease | `12.0.0-rc.1` / `v12.0.0-rc.1` |
| Published stable | `12.0.0` / `v12.0.0` at `eb11f892ae28f076e7a83c38a635316c6ed89028` |
