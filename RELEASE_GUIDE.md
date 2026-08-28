# 🚀 Rullst — Release & Development Workflow Guide

> This document explains the official process for developing new features, testing them safely, and releasing stable versions of Rullst to crates.io without breaking things for users.

---

## 🧠 The Core Concept

**The golden rule: `main` is the promoted release/maintenance line; active work
belongs in `dev`.** A branch name is not a publication or a security
certification. Official release artifacts remain crates.io packages and their
matching immutable tags.

We use **two permanent branches**:

| Reference | What it is | Published to crates.io? |
|--------|------------|------------------------|
| `main` | Promoted release or maintenance source line | Only after an approved release tag |
| `dev` | Active integration and unreleased development | Never directly |
| `vX.Y.Z[-pre]` | Immutable source snapshot approved for release | Triggers the release workflow |
| crates.io `X.Y.Z[-pre]` | Official distributed artifact | Yes |

---

## 📋 The Full Release Cycle (Step by Step)

### Phase 1 — Develop on `dev`

All new work starts from `dev` and returns through a pull request targeting
`dev`. Direct pushes to `main` are not part of the normal workflow; promotion
uses a reviewed `dev` → `main` pull request.

```powershell
# Switch to dev before starting any new work
git switch dev
git pull --ff-only origin dev
git switch -c feat/<short-topic>
```

Make your changes, bug fixes, new features, etc.

```powershell
# Commit your work as usual
git add .
git commit -m "feat(scope): add concise capability"
git push -u origin feat/<short-topic>
```

Every push and pull request to `dev` triggers the relevant CI. Checks are
classified so unfinished roadmap work does not make every development signal
meaningless:

- **Development baseline:** formatting, compilation, strict Clippy, tests for
  implemented behavior, and current panic/unsafe/security invariants should
  remain green. A broken current contract is fixed before more work is stacked.
- **Readiness evidence:** coverage, benchmarks, broader platform matrices and
  unfinished v12 acceptance scenarios may be observational while their named
  checklist item is open. Failures stay visible and tracked; they are not
  silently presented as passing.
- **Release gates:** before an RC or stable tag, every required check in
  `docs/src/v12.md` must pass on the exact candidate commit.

A test for a capability that is deliberately not implemented yet must be tied
to a v12 checklist item and explicitly quarantined with a reason. It must not
remain an unexplained ordinary test failure until the end of the release cycle.

---

### Phase 2 — Verify Stability

Before releasing, make sure:

- [ ] All required release checks have been rerun and are ✅ **green** on the
  exact candidate commit:
  - `ci.yml`: Multi-OS test matrix (Ubuntu, macOS ARM64, Windows MSVC).
  - `kani.yml`: Model checking for the explicit harnesses and configured bounds;
    this is not a proof of every path in the workspace.
  - `sanitizers.yml`: ThreadSanitizer (`TSan`) and AddressSanitizer (`ASan`) for
    the targets declared by the workflow.
  - `miri.yml`: Undefined Behavior and strict-provenance checks for its declared
    package matrix.
  - `fuzzing.yml`: Bounded libFuzzer runs and OSS-Fuzz packaging/readiness.
  - `e2e-smoke.yml`: Live SSR HTML status 200 checks, CSRF, and SQLite/Postgres persistence.
- [ ] You have manually verified the mandatory local trifecta:
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace --all-features`.
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
- [ ] All 15 publishable crate `Cargo.toml` versions and internal requirements
  are synchronized (`12.0.0-rc.1` for the RC or `12.0.0` for stable):
  - `rullst-macros`, `rullst-orm-macros`
  - `rullst-core`, `rullst-orm`, `rullst-auth`, `rullst-security`
  - `rullst-ai`, `rullst-capital`, `rullst-connect`, `rullst-iot`, `rullst-mail`
  - `rullst-studio`, `rullst-nexus`
  - `cargo-rullst`, `rullst`

---

### Phase 3 — Release (Promote to `main` + Create a Tag)

Once everything is stable and verified:

1. Freeze feature work and prepare the synchronized version change in `dev`.
2. Run the full local and CI release gates.
3. Open a promotion PR from `dev` to `main` and require the configured checks.
4. If merging produces a different SHA, rerun the release gates on that final
   `main` SHA.
5. Create `v12.0.0-rc.1` or `v12.0.0` only on the approved SHA, then push that
   tag to trigger the release workflow:

```powershell
git switch main
git pull --ff-only origin main
git tag v12.0.0-rc.1
git push origin v12.0.0-rc.1
```

The RC is a real public crates.io release. It can be yanked but never
overwritten; inspect and test every `.crate` before pushing the tag. Users must
opt in to it explicitly with a requirement such as `12.0.0-rc.1`.

GitHub Actions will automatically execute the topological crate publish pipeline:
1. ✅ `rullst-macros` & `rullst-orm-macros`
2. 📦 Foundations: `rullst-orm`, `rullst-core`
3. 📦 Domain crates: `rullst-connect`, `rullst-iot`, `rullst-security`, `rullst-ai`, `rullst-capital`, `rullst-mail`, `rullst-auth`
4. 📦 Dashboards: `rullst-nexus`, `rullst-studio`
5. 📦 Main bundle & CLI: `rullst`, `cargo-rullst`

---

### Phase 4 — Start the Next Version on `dev`

After the release, immediately start the next development cycle on `dev`:

```powershell
# Switch back to dev
git switch dev

# Update versions to next iteration (e.g., 12.1.0-dev)
# Add new [Unreleased] section to CHANGELOG.md

git add .
git commit -m "chore(release): start 12.1 development"
git push
```

---

## 🔄 Visual Summary

```
                        YOU WORK HERE
                              │
                              ▼
dev ──────────────────────────────────────────────────▶
     commit commit commit    │ cargo sync, version bump
                             │ reviewed promotion PR
                             ▼
main ────────────────────────●────────────────────────▶
                             │ git tag vX.Y.Z
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
> **Never** use `main` as a work branch. Start from `dev` and promote through the
> reviewed process above.

> [!IMPORTANT]
> The automatic publishing only triggers when you push a **version tag** (e.g., `v1.0.5`). A regular `git push` to `main` does **NOT** publish to crates.io.

---

## 🔑 One-time GitHub Setup Required

Registered crates use crates.io Trusted Publishing through GitHub OIDC. The
protected `crates-io` environment must require review and be configured for
`release.yml`. Names that have never been registered require the narrowly
scoped, short-lived `CRATES_IO_BOOTSTRAP_TOKEN` described in
[`docs/src/release-recovery.md`](docs/src/release-recovery.md). Revoke and remove
that bootstrap credential immediately after first publication; do not maintain
a permanent repository-wide registry token.

---

## 📌 Quick Reference Commands

```powershell
# Start new work
git switch dev
git pull --ff-only origin dev

# Sync README badges after bumping version
cargo sync

# Check status before releasing
git status

# After an approved dev -> main promotion PR
git switch main
git pull --ff-only origin main
git tag vX.Y.Z
git push origin vX.Y.Z
```

---

## 🗺️ Current State

| Item | Version |
|------|---------|
| `rullst` | Check `rullst/Cargo.toml` |
| `rullst-macros` | Check `rullst-macros/Cargo.toml` |
| `cargo-rullst` | Check `cargo-rullst/Cargo.toml` |
| Current `main` line | Legacy v5 maintenance baseline during v12 development |
| Active dev branch | `dev` |
| v12 stable decision | `NO-GO` until the documented gates pass |
