# Contributing to rullst-connect

First off, thank you for considering contributing to `rullst-connect`! It's people like you that make this tool better for everyone.

## Getting Started

1. Fork the repository on GitHub.
2. Clone your fork locally.
3. Make sure you have Rust and Cargo installed.
4. Create a new branch for your feature or bugfix.

## Development Workflow

- Run `cargo build` to compile the project.
- Run `cargo test` to execute the test suite. Ensure all tests pass before submitting a Pull Request.
- Run `cargo clippy --all-targets --all-features` to catch common mistakes and improve your Rust code.
- Format your code with `cargo fmt`.

## Pull Request Process

1. Ensure your changes are covered by tests.
2. Update the `CHANGELOG.md` or documentation if your change introduces new features or alters existing behavior.
3. Submit a Pull Request targeting the `main` branch.
4. Wait for a maintainer to review your PR. We might suggest some changes or improvements.

## Releasing

The release checklist lives in [RELEASING.md](RELEASING.md). It covers `cargo-release`, tag-based publishing, and the manual GitHub Actions fallback.

## Code of Conduct

By participating in this project, you are expected to uphold our [Code of Conduct](CODE_OF_CONDUCT.md).
