# Contributing to rullst-orm

First off, thanks for taking the time to contribute! :tada: :+1:

The following is a set of guidelines for contributing to `rullst-orm`.

## Branching Strategy

- **`main`**: The stable branch. Do not submit pull requests directly to `main`.
- **`dev`**: The active development branch. **All Pull Requests must target the `dev` branch.**

## Local Development

1. Fork the repository and clone it locally.
2. Create a new branch off `dev`: `git checkout -b feature/my-feature`
3. Make your changes.
4. Make sure tests pass: `cargo test`
5. Ensure your code is formatted properly: `cargo fmt`
6. Check for linter warnings: `cargo clippy`

## Submitting a Pull Request

- Target the `dev` branch.
- Provide a clear and descriptive title.
- Explain the changes you've made in the description.
- Link any relevant issues.
