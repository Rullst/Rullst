# Reproducible Crate Archives

Rullst claims one specific kind of reproducibility: for the same commit and the
toolchain pinned in that commit's `rust-toolchain.toml`, `cargo package`
produces **bit-for-bit identical `.crate` archives**. The checkout path, file
modification times, file ownership and the user's umask do not change the
bytes. This page states the evidence, how to check it yourself and what is not
claimed.

## Why the archives are stable

Cargo writes the tar archive in a deterministic mode. In the archives Rullst
publishes, every entry has the same fixed timestamp (2006-07-23 22:21 UTC),
user and group 0, no user or group names, and normalized permissions: a
checkout made under umask `077` still produces the same bytes. The gzip header
carries no timestamp. `.cargo_vcs_info.json` records the commit
and the crate's path inside the repository, not an absolute path. The
embedded `Cargo.lock` comes from the workspace lockfile through `--locked`.

## Evidence

| Check | When it runs | What it compares |
| --- | --- | --- |
| [`check-package-reproducibility.sh`](https://github.com/Rullst/Rullst/blob/main/.github/check-package-reproducibility.sh) in the `Code Quality & Format` job of [`ci.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) | Every pull request and push that runs Rust CI | `rullst-macros` and `rullst-orm-macros`, packaged once from the CI checkout and once from a fresh `git worktree` of the same commit at another path, with new file times and umask `077`. Each run uses its own empty target directory, and the SHA-256 digests must match. |
| `Reproduce publication inputs without registry identity` in [`release.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml) | Every release tag, before publication | Every crate in [`release-order.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-order.json) is repackaged from the tagged checkout on a separate runner and compared byte-for-byte (`cmp`) with the archives that were verified and attested. Any difference stops the release. For example, [v12.2.0 run 37128793097](https://github.com/Rullst/Rullst/actions/runs/37128793097) passed this job. |
| Independent local rebuild, 9 October 2026 | One-off check | A fresh clone of tag `v12.2.0` (`565e222a3249f788a0eb560b7ecd6a2ce379ad47`), packaged with Rust 1.98.1 on a developer workstation. It reproduced `rullst-macros-12.2.0.crate` with SHA-256 `23f73c494af1e7d37084e7bce8bcef77bf5638f0f822d79c3334a2353e807bf5`, the same digest as the GitHub release asset, its `checksums.txt` and the crates.io checksum. |

The release workflow's verification job packages with full build verification
and the preflight job packages with `--no-verify`. Their archives are compared
byte-for-byte, so build verification does not change the archive.

## Reproduce a published crate yourself

Pick a published version and its tag. The tag's `rust-toolchain.toml` selects
the pinned toolchain automatically through rustup.

```sh
git clone https://github.com/Rullst/Rullst.git
cd Rullst
git checkout v12.2.0
target_dir="$(mktemp -d)"
CARGO_TARGET_DIR="$target_dir" cargo package -p rullst-macros \
  --all-features --locked --no-verify
sha256sum "$target_dir/package/rullst-macros-12.2.0.crate"
curl --silent --user-agent "rullst-reproduction-check" \
  https://crates.io/api/v1/crates/rullst-macros/12.2.0 | jq -r .version.checksum
```

The two digests must be equal. They must also match the line for the same
file in the release's `checksums.txt`. To check the signature on that release
as well, see [Verifying a release](../../SECURITY.md#verifying-a-release).

## Limits

- **Only `.crate` source archives are claimed.** The native `rullst` and
  `cargo-rullst` executables attached to GitHub releases are built on
  GitHub-hosted runners and attested, but they are not claimed to be
  bit-for-bit reproducible. They embed platform, linker and toolchain details,
  and no job rebuilds and compares them.
- **The same toolchain is required.** A different Cargo version may normalize
  manifests or lay out archives differently. Use the toolchain pinned by the
  tag.
- **The same clean commit is required.** Packaging uncommitted changes with
  `--allow-dirty` records a dirty state and different content.
- **The pull-request check is bounded.** It covers two small crates to keep the
  cost low. All admitted crates are compared once per release tag.
- **Release evidence files are not claimed reproducible.** The SBOM, audit
  results, metadata and release context are generated at release time. The
  context records the workflow run, and the audit reflects the advisory
  database of that moment. They are attested, not reproduced.
