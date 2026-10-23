# 🚀 Rullst — Release & Development Workflow Guide

> This document explains the official process for developing new features, testing them safely, and releasing stable versions of Rullst to crates.io without breaking things for users.

---

## 🧠 The Core Concept

**The golden rule: release majors have explicit protected source lines:
v12 uses `v12`, and v13 uses `main`.** Normal work is reviewed through short-lived branches targeting
the appropriate release line. A branch name is not a publication or a security certification.
Official release artifacts remain crates.io packages and their matching
immutable tags.

The permanent branches are `main`, `v12` and `gh-pages`. Patch releases such as
12.1.1 use tags, not additional permanent branches. Remove short-lived branches
after protected integration. Assess every stable fix for a tested forward-port
to `main`; see the [cross-line maintenance policy](docs/src/compatibility-policy.md#carrying-fixes-between-maintained-lines).

The repository separates maintained releases, next-major work, historical
source and generated site data:

| Reference | What it is | Published to crates.io? |
|--------|------------|------------------------|
| `main` | Protected v13 development and release source | Only after an admitted release tag |
| `v12` | Frozen 12.x source; the line reached end of life on 16 October 2026 | No further release |
| `gh-pages` | Generated website/benchmark history used by Pages | No |
| `feat/*`, `fix/*`, etc. | Short-lived reviewed work | Never directly |
| `vX.Y.Z[-pre]` | Immutable source snapshot approved for release | Triggers the release workflow |
| crates.io `X.Y.Z[-pre]` | Official distributed artifact | Yes |

---

## 📋 The Full Release Cycle (Step by Step)

### Phase 1 — Select the maintenance or development line

Compatible v12 work starts from the latest green `v12` and returns through a
pull request targeting `v12`. Product features and breaking work start from
`main` and target `main`. Keep branches short-lived and do not stack work on a
broken required gate. The example below is for v12 maintenance:

```powershell
# Synchronize the maintenance branch before starting a v12 fix
git switch v12
git pull --ff-only origin v12
git switch -c fix/<short-topic>
```

Make your changes, bug fixes, new features, etc.

```powershell
# Commit your work as usual
git add .
git commit -m "fix(scope): describe the correction"
git push -u origin fix/<short-topic>
```

Every push to `main` or `v12` and every pull request targeting either triggers the relevant
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
  - `coverage.yml`: exact-SHA whole-repository and framework-library line
    coverage with both configured 90% floors and a blocking Codecov upload.
    Patch reports remain visible; they do not replace the governed floors.
  - `proptest.yml`: release-mode invariant suites with 10,000 configured cases.
  - `kani.yml`: Model checking for the explicit harnesses and configured bounds;
    this is not a proof of every path in the workspace.
  - `sanitizers.yml`: ThreadSanitizer (`TSan`) and AddressSanitizer (`ASan`) for
    the targets declared by the workflow.
  - `miri.yml`: Undefined-behavior checks with randomized layouts for its
    declared pure-Rust scopes.
  - `fuzzing.yml`: Bounded libFuzzer runs over the versioned inventory
    (42 targets for v13; 40 for v12). The
    separate `oss-fuzz/projects/rullst` directory remains an unsubmitted local
    integration draft and is not release evidence.
  - `dast-zap.yml`: Blocking ZAP baselines for freshly generated REST API and
    LMS applications plus the explicitly informational blog showcase.
  - `e2e-smoke.yml`: Live SSR, security-header, CSRF and SQLite-persistence
    checks against the release-built Blog application.
- [ ] You have manually verified the mandatory local trifecta:
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-features -- -D warnings`, and
  `cargo fmt --all`.
- [ ] `CHANGELOG.md` has a detailed release section describing all additions and fixes.
- [ ] The [compatibility and MSRV policy](docs/src/compatibility-policy.md) still
  matches the manifests, supported-version table, and intended release changes.
- [ ] The [Cargo feature matrix](docs/src/feature-matrix.md) still matches every
  publishable manifest and the feature-boundary CI matrix.
- [ ] The [v13 migration guide](docs/src/migration-v13.md) (whose rows the
  `rullst-upgrade-rules-v4` catalog references), the
  [v12 migration guides](docs/src/migration-v12.md) and
  [AI capability matrix](docs/src/ai-provider-capabilities.md) match the APIs,
  CLI behavior, and known release-history boundaries.
- [ ] Every current security statement matches the code, test, and limit in the
  [v12 security claims ledger](docs/src/v12-security-claims.md); do not promote
  an unlisted or unevidenced statement into release notes.
- [ ] The packaged
  [security-event v1 JSON Schema](rullst-security/schema/security-event-v1.schema.json)
  matches `LiveSecurityEvent`, and any incompatible event change uses a new
  schema version instead of silently changing v1.
- [ ] Every package in [the release order](.github/release-order.json) has
  synchronized manifest versions and exact internal requirements (`=X.Y.Z`)
  at the selected release version; `.github/release-preflight.jq` refuses a
  caret requirement. The current v13 candidate inventory has 20 packages, including
  `rullst-privacy`, `rullst-supervision`, `rullst-media` and `rullst-labs`.
  Labs ships its contracts only; the Labs runner candidate was removed from
  13.0 and is not packaged. A successful archive rehearsal does not by itself
  authorize publication.
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
   reviewed pull request into the selected release branch (`v12` for v12,
   `main` for v13).
2. On the release-branch head, run `python3 .github/release-preflight.py vX.Y.Z`
   before dispatching the manual admission workflows. It refuses, in seconds,
   problems that `release.yml` would otherwise find hours later:
   - packages not at the tagged version, or a missing changelog section;
   - a stale head or an existing tag;
   - an unregistered or foreign-owned crate name;
   - a version already on crates.io.

   For a change that can behave differently in release builds
   (`cfg(debug_assertions)`, release profile), dispatch `proptest.yml` on its
   branch before merging. It is the admission workflow that runs the workspace
   tests in release mode.
3. Run the full local and CI release gates on the resulting release-branch SHA.
4. Record and review the package/evidence artifacts for that exact SHA.
5. Create a new version tag only on the approved SHA, then push that tag to
   trigger the release workflow. Published tags, including `v12.0.0` and `v12.1.0`, are immutable;
   never recreate or move them. For a reviewed v13 candidate:

```powershell
git switch main
git pull --ff-only origin main
git tag v13.X.Y
git push origin v13.X.Y
```

An RC or stable version is a real public crates.io release. It can be yanked but
never overwritten; inspect and test every `.crate` before pushing the tag.
Prereleases require explicit opt-in with a requirement such as `13.0.0-rc.1`.

GitHub Actions will automatically execute the topological crate publish pipeline:
1. ✅ `rullst-macros` & `rullst-orm-macros`
2. 📦 Foundations: `rullst-orm`, `rullst-core`, `rullst-messaging`
3. 📦 Domain crates: `rullst-connect`, `rullst-iot`, `rullst-security`, `rullst-ai`, `rullst-capital`, `rullst-mail`, `rullst-auth`, then the v13 candidates `rullst-privacy`, `rullst-supervision` and `rullst-media`
4. 📦 Dashboards: `rullst-nexus`, `rullst-studio`
5. 📦 Main bundle & CLI: `rullst`, `cargo-rullst`

Before publication, the workflow:
- requires every admission workflow at the tagged SHA, including the full
  `ci.yml` release matrix;
- extracts the exact version section from `CHANGELOG.md`;
- packages and reproduces every archive;
- writes checksums and the tag-bound evidence bundle;
- creates a SHA-pinned GitHub build-provenance attestation.

The tag run does not repeat the tests that the admission already ran at that
SHA. `.github/release-verification-coverage.json` records which `ci.yml` job
covers each check, and a test fails if one stops covering it. The release also
requires its archive checksums to equal the archives that the admitted
packaged-distribution job tested. The GitHub release uses those extracted notes and is marked as a
prerelease automatically when the semantic version contains a prerelease
suffix. This evidence does not establish a project-wide SLSA level or an
independent certification.

---

### Phase 4 — Continue v13 while maintaining v12

The v12 line reached end of life on 16 October 2026 (12.3.0 was its last
release) and the `v12` branch is frozen, so the v12 maintenance steps of this
phase are historical. The `main` branch develops v13; the former `v13` branch
is retired. Start
next-major work from `main` after
reviewing its roadmap and differences from the published v12 source:

```powershell
git switch main
git pull --ff-only origin main
git switch -c feat/<short-topic>
```

Carry applicable v12 fixes forward through reviewed changes. Dependabot
proposes SemVer-major dependency updates only for `main`; evaluate them there
first. A dependency's major version does not by itself prove that Rullst's
public API must break, so a compatible major may still be backported to `12.x`
deliberately (for example, to resolve an advisory). Changes that break Rullst's
compatibility contract belong to v13.
Keep the v12 release gates active while v13's own CI policy evolves.

Version 12.3.0, the last 12.x release, is the latest published stable release.
Carry the compatible 12.1 [update experience](ROADMAP.md#safe-update-experience)
into v13.
Synchronize package versions only when the candidate is accepted for release.
A major upgrade needs its own tested migration rules, not just an updated
installer. Follow the [v13 delivery plan](docs/src/v13-delivery-plan.md) for
current priorities. This plan does not itself authorize publication.

---

## 🔄 Visual Summary

```
short-lived branches ── reviewed pull requests ──▶ v12 (12.x) / main (v13)
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

All twenty release crates are registered on crates.io and publish through
Trusted Publishing with GitHub OIDC. The protected `crates-io` environment must
require review and be configured for `release.yml`.

The four v13 names were first published in 13.0.0-alpha.1 with a one-time
token, described under
[v13 package registration](docs/src/release-recovery.md#v13-package-registration).
That fallback was removed again, and the bootstrap allowlist in
`.github/crates-ownership-policy.json` is empty. `release.yml` refuses
publication while any release package is unregistered.

Do not maintain a permanent repository-wide registry token.

---

## 📌 Quick Reference Commands

```powershell
# Start a compatible v12 maintenance change (use main for v13 work)
git switch v12
git pull --ff-only origin v12
git switch -c fix/<short-topic>

# Check status before releasing
git status

# After the candidate commit is approved on main
git switch main
git pull --ff-only origin main
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
| Current `main` line | `13.0.0-alpha.3` prerelease source; only a reviewed tag publishes it, a branch merge does not |
| Former maintenance line | Frozen `v12`; end of life since 16 October 2026 |
| Legacy source | Immutable `v5.0.0` tag and published archive |
| Published v13 prereleases | `13.0.0-alpha.1` / `v13.0.0-alpha.1` (October 11, 2026); check [crates.io](https://crates.io/crates/rullst/versions) for later ones |
| Latest published stable | Check [crates.io](https://crates.io/crates/rullst) and the [v12 record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md) |
