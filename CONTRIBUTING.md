# Contributing to Rullst

First off, thank you for considering contributing to Rullst! It's people like you that make Rullst such a great tool for the Rust ecosystem.

## How Can I Contribute?

### Reporting Bugs
This section guides you through submitting a bug report for Rullst. Following these guidelines helps maintainers and the community understand your report, reproduce the behavior, and find related reports.
- Use the provided **Bug Report** issue template.
- Explain the problem and include additional details to help maintainers reproduce the problem.

### Suggesting Enhancements
This section guides you through submitting an enhancement suggestion for Rullst, including completely new features and minor improvements to existing functionality.
- Use the provided **Feature Request** issue template.
- Provide a clear and descriptive title for the issue to identify the suggestion.

### Pull Requests & Commit Guidelines
- **Strict Conventional Commits**: All commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:
  - Format: `<type>(<scope>): <short imperative description>`
  - Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
  - Example: `fix(auth): offload password hashing to spawn_blocking`.
- **Zero "Smoke/AI" Commit Fluff**: Never use AI-generated paragraphs, marketing buzzwords, or verbose descriptions in commit titles. Keep messages concise, technical, and accurate to the exact diff.
- Run `cargo fmt --all`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace --all-features` before declaring a change complete.
- Fill in the required Pull Request template.
- End files with a newline.

## Development Setup

1. Fork the repo and create a short-lived branch from the latest green `main`.
2. Configure git hooks: `git config core.hooksPath .githooks`.
3. Run `cargo build --workspace --all-features` to build the framework.
4. Run `cargo test --workspace --all-features` to exercise implemented behavior.
5. Run `cargo fmt --all` to format your code.
6. If you've added code that should be tested, add tests.
7. If you've changed APIs, update the documentation.
8. Ensure the full test suite passes.

## Branching Model
- `main`: Protected active integration and release source line. Normal pull
  requests target this branch; required checks should remain green. It is not
  itself a crates.io publication or security certification.
- `v5`: Frozen historical source for the legacy v5 line. Do not target it with
  routine fixes or dependency updates; released v5 consumers should pin the
  immutable `v5.0.0` tag or crates.io artifact.
- Feature and fix branches are short-lived and branch from `main`.

Thank you for your interest in making Rullst better!
