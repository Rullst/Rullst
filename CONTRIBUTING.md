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

### AI-Assisted Contributions

Rullst welcomes responsible use of generative AI, code assistants, and
autonomous agents. AI assistance never transfers responsibility away from the
human contributor. The person opening a pull request must understand the diff,
have the right to contribute it, and be able to explain and maintain it.

The pull request template requires one of two disclosures: no generative AI was
used, or AI was used and its tools/models, affected scope, and human validation
are summarized. Assisted pull requests must also:

- describe the intended behavior, trust boundaries, and meaningful negative
  cases instead of pasting a model-generated narrative;
- include tests that exercise the changed behavior and list the exact local
  commands that were run;
- independently verify every new dependency, API, security claim, benchmark,
  citation, and generated file;
- keep credentials, personal data, private vulnerability details, proprietary
  prompts, and third-party source code out of prompts and transcripts; and
- remain small enough for a reviewer to understand. Maintainers may request
  that broad agent-generated changes be split into independently reviewable
  pull requests.

The exact prompt or transcript is **not required**. A contributor may provide a
redacted prompt or content-addressed transcript as optional review context, but
a screenshot, recording, signature, timestamp, or SHA-256 digest can prove at
most the integrity of the disclosed material. It cannot prove that the record
is complete, that it was the only model interaction, or that the resulting code
was not changed later. Such material does not replace code review, CI, tests, or
artifact provenance and does not increase the trust assigned to a pull request.

Undisclosed or misunderstood AI assistance may cause a pull request to be
closed. Deliberately false disclosure, fabricated test evidence, hidden
behavior, prompt-injection instructions aimed at reviewers or tools, and
dependency confusion/slopsquatting are treated as supply-chain security
concerns. Do not put vulnerability details in a public pull request; follow
[`SECURITY.md`](SECURITY.md) for coordinated private disclosure.

This policy follows the OpenSSF/CNCF guidance that projects should state their
AI contribution rules while recognizing that AI use cannot be detected with an
absolute guarantee. Rullst therefore applies the same fail-closed review and
verification gates to every contribution, regardless of authorship. See
[Securing Open Source in the Age of AI](https://openssf.org/resources/securing-open-source-in-the-age-of-ai-a-practical-guide/).

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
