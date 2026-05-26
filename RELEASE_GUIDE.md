# 🚀 Rullst — Release & Development Workflow Guide

> This document explains the official process for developing new features, testing them safely, and releasing stable versions of Rullst to crates.io without breaking things for users.

---

## 🧠 The Core Concept

**The golden rule: the `main` branch is ALWAYS the stable, production version.**

We use **two permanent branches**:

| Branch | What it is | Published to crates.io? |
|--------|------------|------------------------|
| `main` | ✅ **Stable** — tested and approved | ✅ **YES**, via `git tag` |
| `dev` | 🔧 **Work in progress** — active development | ❌ **NEVER** directly |

---

## 📋 The Full Release Cycle (Step by Step)

### Phase 1 — Develop on `dev`

All new work happens on the `dev` branch. Never commit directly to `main`.

```powershell
# Switch to dev before starting any new work
git checkout dev
git pull origin dev   # Always pull latest before starting
```

Make your changes, bug fixes, new features, etc.

```powershell
# Commit your work as usual
git add .
git commit -m "feat: add awesome new feature"
git push
```

Every push to `dev` automatically triggers the CI (GitHub Actions), which:
- Runs `cargo fmt --check` to validate code formatting
- Runs `cargo clippy` to check for code quality warnings
- Runs `cargo test` to run all unit tests

---

### Phase 2 — Verify Stability

Before releasing, make sure:

- [ ] All CI checks on `dev` are ✅ **green** on GitHub
- [ ] You have manually tested the feature locally
- [ ] The `CHANGELOG.md` has a new section describing what changed
- [ ] All `Cargo.toml` versions have been bumped (e.g., `1.0.4` → `1.0.5`) in:
  - `rullst-macros/Cargo.toml`
  - `rullst/Cargo.toml` (also update `rullst-macros` dependency version)
  - `cargo-rullst/Cargo.toml`
- [ ] README badges are synchronized by running:

```powershell
cargo sync
```

---

### Phase 3 — Release (Merge to `main` + Create a Tag)

Once everything is stable and verified:

```powershell
# 1. Switch to main
git checkout main

# 2. Merge the stable dev branch into main
git merge dev

# 3. Push main
git push origin main

# 4. Create a version tag (replace 1.0.5 with your actual version)
git tag v1.0.5

# 5. Push the tag — THIS triggers the automatic crates.io publish!
git push origin v1.0.5
```

That's it! GitHub Actions will automatically:
1. ✅ Run all tests one final time
2. 📦 Publish `rullst-macros` to crates.io
3. ⏳ Wait 30 seconds for crates.io to index it
4. 📦 Publish `rullst` to crates.io
5. ⏳ Wait 30 seconds
6. 📦 Publish `cargo-rullst` to crates.io

---

### Phase 4 — Start the Next Version on `dev`

After the release, immediately start the next development cycle on `dev`:

```powershell
# Switch back to dev
git checkout dev

# Open rullst-macros/Cargo.toml, rullst/Cargo.toml, cargo-rullst/Cargo.toml
# and bump version to the NEXT version (e.g., 1.0.6)
# Also add a new [Unreleased] section to CHANGELOG.md

git add .
git commit -m "chore: bump version to 1.0.6-dev"
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
                             │ git merge dev
                             ▼
main ────────────────────────●────────────────────────▶
                             │ git tag v1.0.5
                             ▼
                   🤖 GitHub Actions CI
                   runs all tests...
                             │ if ✅ all green
                             ▼
                   📦 cargo publish (automatic)
                       crates.io v1.0.5
```

---

## ⚠️ Important Rules

> [!CAUTION]
> **Never** run `cargo publish` manually from your machine anymore. Let the GitHub Actions automation do it. This ensures tests ALWAYS pass before publishing.

> [!WARNING]
> **Never** commit directly to `main`. Always work on `dev` and merge via the process above.

> [!IMPORTANT]
> The automatic publishing only triggers when you push a **version tag** (e.g., `v1.0.5`). A regular `git push` to `main` does **NOT** publish to crates.io.

---

## 🔑 One-time GitHub Setup Required

For the automatic publishing to work, you need to add your crates.io API token as a GitHub secret:

1. Go to **[crates.io](https://crates.io)** → Account Settings → **API Tokens** → Generate a new token
2. Go to your **GitHub repository** → **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Set:
   - **Name:** `CARGO_REGISTRY_TOKEN`
   - **Value:** *(paste your crates.io token)*
5. Click **Add secret**

---

## 📌 Quick Reference Commands

```powershell
# Start new work
git checkout dev && git pull origin dev

# Sync README badges after bumping version
cargo sync

# Check status before releasing
git status

# Release a new stable version
git checkout main
git merge dev
git push origin main
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
| Active dev branch | `dev` |
