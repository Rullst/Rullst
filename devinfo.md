# Local developer notes

The framework specification is [`docs/src/spec.md`](docs/src/spec.md). The
official branch, verification, packaging, and tag process is
[`RELEASE_GUIDE.md`](RELEASE_GUIDE.md); do not maintain a second release recipe
in this file.

## Work on the unreleased v12 source

From the repository root, install the CLI from the exact checkout:

```bash
cargo install --locked --path cargo-rullst
cargo rullst --help
```

Run `cargo rullst new` from this root during the source-only phase so the
generator can select the sibling framework crates as path dependencies. A plain
`cargo install cargo-rullst` installs the latest published release, which does
not expose unreleased v12 APIs. After an immutable RC exists, install its exact
version with `--version 12.0.0-rc.1 --locked`.

## Required local verification

Do not run `cargo clean` as a routine pre-flight; it destroys reusable build
artifacts and makes verification slower. The required commands are:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

Run feature, database, package, platform, and research workflows according to
[`docs/src/workflows.md`](docs/src/workflows.md). Do not run `clippy --fix` as a
release check because it mutates source instead of proving the checked tree is
clean.

## Windows Application Control / WDAC

On a Windows workstation whose policy permits developer binaries only in an
approved Cargo target directory:

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\rullst-target"
$env:TEMP = "$env:CARGO_TARGET_DIR\tmp"
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
cargo test --workspace --all-features
```

Some enterprise policies can still reject rustdoc's unsigned temporary
`rust_out.exe` with `os error 4551`. Do not disable or weaken the policy for a
test run. Retain the compiled-test result and use an approved runner for the
doctest gate.

## Releases

Never run `cargo publish`, `cargo release`, or create a version tag from this
informal note. Releases use one synchronized version, the machine-readable
`.github/release-order.json`, inspected package artifacts, and the protected
tag-only workflow described in the release guide. A normal push to `main` does
not publish a crate.
