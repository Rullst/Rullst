Info for developers

- Working locally on the framework:
cargo install --path cargo-rullst #from the repo root


- Testing as an end user:
cargo install cargo-rullst --force


Command to check for updates in the project:
cargo outdated --root-deps-only


Windows environments managed by Application Control / WDAC:

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\rullst-target"
$env:TEMP = "$env:CARGO_TARGET_DIR\tmp"
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
cargo test --workspace --all-features
```

This keeps Cargo build scripts and test binaries outside protected workspace
folders. Some enterprise policies can still reject rustdoc's unsigned temporary
`rust_out.exe` with `os error 4551`; do not disable or weaken that policy for a
test run. Keep the compiled-test result and use the Linux release workflow as
the authoritative doctest execution until an administrator provides an approved
developer-code policy.



Linux Bash:

cargo clean && \
cargo test --workspace --all-features && \
cargo clippy --workspace --all-features --fix --allow-staged && \
cargo fmt --all && \
cargo publish -p rullst-macros --dry-run --allow-dirty && \
cargo publish -p rullst-orm-macros --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-orm --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-core --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-connect --dry-run --allow-dirty && \
cargo publish -p rullst-iot --dry-run --allow-dirty && \
cargo publish -p rullst-security --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-ai --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-capital --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-mail --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-auth --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-nexus --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst-studio --dry-run --no-verify --allow-dirty && \
cargo publish -p rullst --dry-run --no-verify --allow-dirty && \
cargo publish -p cargo-rullst --dry-run --allow-dirty

PowerShell 5.1:

cargo clean; `
cargo test --workspace --all-features; `
cargo clippy --workspace --all-features --fix --allow-dirty --allow-staged; `
cargo fmt --all; `
cargo publish -p rullst-macros --dry-run --allow-dirty; `
cargo publish -p rullst-orm-macros --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-orm --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-core --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-connect --dry-run --allow-dirty; `
cargo publish -p rullst-iot --dry-run --allow-dirty; `
cargo publish -p rullst-security --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-ai --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-capital --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-mail --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-auth --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-nexus --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst-studio --dry-run --no-verify --allow-dirty; `
cargo publish -p rullst --dry-run --no-verify --allow-dirty; `
cargo publish -p cargo-rullst --dry-run --allow-dirty



Powershell 7:

cargo clean &&
cargo test --workspace --all-features &&
cargo clippy --workspace --all-features --fix --allow-dirty --allow-staged &&
cargo fmt --all &&
cargo publish -p rullst-macros --dry-run --allow-dirty &&
cargo publish -p rullst-orm-macros --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-orm --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-core --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-connect --dry-run --allow-dirty &&
cargo publish -p rullst-iot --dry-run --allow-dirty &&
cargo publish -p rullst-security --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-ai --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-capital --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-mail --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-auth --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-nexus --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst-studio --dry-run --no-verify --allow-dirty &&
cargo publish -p rullst --dry-run --no-verify --allow-dirty &&
cargo publish -p cargo-rullst --dry-run --allow-dirty





Releasing a new version:

git tag -a v12.0.0 -m "Release v12.0.0"

git push origin v12.0.0
