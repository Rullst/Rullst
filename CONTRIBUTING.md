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

### Pull Requests
- Fill in the required Pull Request template.
- Do not include issue numbers in the PR title.
- Include screenshots and animated GIFs in your pull request whenever possible if your change affects UI or logs.
- End files with a newline.
- Run `cargo fmt` and `cargo clippy` before committing.

## Development Setup

1. Fork the repo and create your branch from `dev`.
2. Run `cargo build` to build the framework.
3. Run `cargo test --workspace` to ensure all tests pass.
4. Run `cargo fmt --all` to format your code.
5. If you've added code that should be tested, add tests.
6. If you've changed APIs, update the documentation.
7. Ensure the test suite passes.

## Branching Model
- `main`: Contains the stable, production-ready code.
- `dev`: Active development branch. All PRs must target this branch!

Thank you for your interest in making Rullst better!
