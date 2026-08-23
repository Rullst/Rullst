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
- Run `cargo fmt` and `cargo clippy --workspace --all-features -- -D warnings` before committing.
- Fill in the required Pull Request template.
- End files with a newline.

## Development Setup

1. Fork the repo and create your branch from `dev`.
2. Configure git hooks: `git config core.hooksPath .githooks`.
3. Run `cargo build` to build the framework.
4. Run `cargo test --workspace` to ensure all tests pass.
5. Run `cargo fmt --all` to format your code.
6. If you've added code that should be tested, add tests.
7. If you've changed APIs, update the documentation.
8. Ensure the full test suite passes.

## Branching Model
- `main`: Contains the stable, production-ready code.
- `dev`: Active development branch. All PRs must target this branch!

Thank you for your interest in making Rullst better!
