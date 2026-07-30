Info for developers

- Working locally on the framework:
cargo install --path cargo-rullst (from the repo root)


- Testing as an end user:
cargo install cargo-rullst --force


Command to check for updates in the project:
cargo outdated --root-deps-only


cargo clean ; cargo test --all-features ; cargo clippy --all-features --fix

cargo fmt ; cargo publish --dry-run



Releasing a new version:

git tag -a v12.0.0 -m "Release v12.0.0"

git push origin v12.0.0
