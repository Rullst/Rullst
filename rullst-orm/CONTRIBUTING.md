# Contributing to rullst-orm

First off, thanks for taking the time to contribute! :tada: :+1:

The following is a set of guidelines for contributing to `rullst-orm`.

## Branching Strategy

- **`main`**: The protected active integration and release source line. Normal
  pull requests target `main`; a branch name alone is not a stable release.
- **`v5`**: Frozen historical source for the legacy v5 line.

## Local Development

1. Fork the repository and clone it locally.
2. Create a new branch off the latest green `main`: `git checkout -b feature/my-feature`
3. Make your changes.
4. Make sure tests pass: `cargo test`
5. Ensure your code is formatted properly: `cargo fmt`
6. Check for linter warnings: `cargo clippy`

## Submitting a Pull Request

- Target the `main` branch.
- Provide a clear and descriptive title.
- Explain the changes you've made in the description.
- Link any relevant issues.
