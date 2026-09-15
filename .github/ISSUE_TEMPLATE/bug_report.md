---
name: Bug Report
about: Create a report to help us improve
title: "[BUG] "
labels: 'bug'
assignees: ''
---

**Security reports**
Do not disclose suspected vulnerabilities in a public issue. Follow
[SECURITY.md](https://github.com/Rullst/Rullst/blob/main/SECURITY.md) and contact
`officialrullst@gmail.com` privately. Never attach credentials, access tokens,
real user data, or an unredacted production database.

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Initialize a project with the exact `cargo rullst new ...` command or wizard choices.
2. Add a minimal reproducible example, including the relevant Cargo features.
3. Run the exact command that fails.
4. See error

**Expected behavior**
A clear and concise description of what you expected to happen.

**Environment (please complete the following information):**
 - OS and architecture:
 - Rust and Cargo versions (`rustc --version`, `cargo --version`):
 - Affected Rullst crate versions from `Cargo.lock` (e.g. `12.0.0`):
 - CLI version (`cargo rullst --version`), if applicable:
 - Exact commit SHA, if using Git dependencies or unreleased v13 source:
 - Enabled Cargo features, blueprint and database/provider profile:
 - Development restart or release build:

**Evidence**
Include the complete relevant error with secrets and personal data removed.
For CI-only failures, include the run URL, job name and source SHA. Describe
whether the failure is repeatable and the last known working version; a green
unrelated workflow is not a reproduction of this issue.

**Additional context**
Add any other context about the problem here.
