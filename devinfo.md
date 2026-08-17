Info for developers

- Working locally on the framework:
cargo install --path cargo-rullst #from the repo root


- Testing as an end user:
cargo install cargo-rullst --force


Command to check for updates in the project:
cargo outdated --root-deps-only


cargo clean && \
cargo test --workspace --all-features && \
cargo clippy --workspace --all-features --fix --allow-staged && \
cargo fmt --all && \
cargo publish -p rullst-macros --dry-run && \
cargo publish -p rullst-orm-macros --dry-run --no-verify && \
cargo publish -p rullst-core --dry-run --no-verify && \
cargo publish -p rullst-orm --dry-run --no-verify && \
cargo publish -p rullst-iot --dry-run && \
cargo publish -p rullst-mail --dry-run --no-verify && \
cargo publish -p rullst-ai --dry-run --no-verify && \
cargo publish -p rullst-connect --dry-run && \
cargo publish -p rullst-security --dry-run --no-verify && \
cargo publish -p rullst-auth --dry-run --no-verify && \
cargo publish -p rullst-capital --dry-run --no-verify && \
cargo publish -p rullst-studio --dry-run --no-verify && \
cargo publish -p rullst-nexus --dry-run --no-verify && \
cargo publish -p rullst --dry-run --no-verify && \
cargo publish -p cargo-rullst --dry-run


Powershell:

cargo clean &&
cargo test --workspace --all-features &&
cargo clippy --workspace --all-features --fix --allow-staged &&
cargo fmt --all &&
cargo publish -p rullst-macros --dry-run &&
cargo publish -p rullst-orm-macros --dry-run --no-verify &&
cargo publish -p rullst-core --dry-run --no-verify &&
cargo publish -p rullst-orm --dry-run --no-verify &&
cargo publish -p rullst-iot --dry-run &&
cargo publish -p rullst-mail --dry-run --no-verify &&
cargo publish -p rullst-ai --dry-run --no-verify &&
cargo publish -p rullst-connect --dry-run &&
cargo publish -p rullst-security --dry-run --no-verify &&
cargo publish -p rullst-auth --dry-run --no-verify &&
cargo publish -p rullst-capital --dry-run --no-verify &&
cargo publish -p rullst-studio --dry-run --no-verify &&
cargo publish -p rullst-nexus --dry-run --no-verify &&
cargo publish -p rullst --dry-run --no-verify &&
cargo publish -p cargo-rullst --dry-run





Releasing a new version:

git tag -a v12.0.0 -m "Release v12.0.0"

git push origin v12.0.0
