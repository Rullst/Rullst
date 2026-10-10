# OpenSSF Best Practices: Silver Evidence

Rullst holds the **passing** [OpenSSF Best Practices badge](https://www.bestpractices.dev/projects/13321)
(project 13321). This page is the evidence guide for the **Silver** form: one
row per Silver criterion, with the answer to select, a justification ready to
paste and the evidence link. Only a project owner can edit the form, and a
Silver badge is not claimed until bestpractices.dev grants it.

The criteria list was taken from the
[official Silver criteria](https://www.bestpractices.dev/en/criteria/1) on
9 October 2026: 55 criteria (44 MUST, 10 SHOULD, 1 SUGGESTED). Several also
appear in the passing form and are already answered there.

**How to use it.** Open the project's Silver form, select the answer in the
second column and paste the justification. Where the form requires a URL
(marked "URL required"), paste the evidence link after the justification.
Pages added together with this guide (the assurance case, release
verification and reproducible archives) resolve only after the change is
merged to `main` and the book is published.

## Blocking

These MUST criteria are not met yet:

| Criterion | What is missing |
| --- | --- |
| `documentation_achievements` | The README does not show the Best Practices badge yet. [PR #447](https://github.com/Rullst/Rullst/pull/447) adds it; merge it, then answer **Met** with the justification in the Basics table. |

Check these before submitting; they are answered Met below, but need an owner
action or confirmation:

- **`vulnerability_report_credit`:** confirm that no vulnerability fixed in the
  last 12 months came from an outside reporter. If one did, credit them in the
  changelog or advisory first.
- **`documentation_current`:** on `main`, the "Supported Versions" section of
  `SECURITY.md` still calls 12.1.2 the latest stable patch; the `v12` copy
  already describes 12.2.0 and 12.3.x. Synchronize it.
- **`documentation_roadmap`:** the roadmap is organized by release lines (v12
  maintenance, v13, ideas after 13.0), not by dates. A reviewer may ask for an
  explicit twelve-month horizon; one dated paragraph in `ROADMAP.md` would
  settle it.
- **`sites_password_security`:** answered N/A because the project's sites store
  no passwords. The README also links demo applications on `rullst.win`, while
  `GOVERNANCE.md` says the project has no registered domains. Decide whether
  those demos count as project sites; if they do, they must hash passwords
  with Argon2id (the `rullst-auth` default) and the answer becomes Met.

## Basics

| Criterion | Answer | Justification to paste | Evidence |
| --- | --- | --- | --- |
| `achieve_passing` | Met | Rullst holds the passing badge (project 13321). | [bestpractices.dev/projects/13321](https://www.bestpractices.dev/projects/13321) |
| `contribution_requirements` (URL required) | Met | CONTRIBUTING.md states the requirements: tests for new functionality and regression tests for fixes, Conventional Commits, `cargo fmt`, strict Clippy and the full test suite, and the pull-request template. | [CONTRIBUTING.md](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md#pull-requests--commit-guidelines) |
| `dco` (URL required) | Unmet | Not adopted. Almost all changes come from the single maintainer, and contributors license their work under MIT through GitHub's terms of service (inbound=outbound), which is not a per-contribution assertion. Adopting it means requiring `git commit -s` in CONTRIBUTING.md and a DCO check on pull requests. | [CONTRIBUTING.md](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md) |
| `governance` (URL required) | Met | GOVERNANCE.md documents who decides, how proposals are accepted, rejected or deferred, the change and release controls, and continuity. | [GOVERNANCE.md](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md) |
| `code_of_conduct` (URL required) | Met | Rullst adopts the Contributor Covenant 2.1 in the standard location. | [CODE_OF_CONDUCT.md](https://github.com/Rullst/Rullst/blob/main/CODE_OF_CONDUCT.md) |
| `roles_responsibilities` (URL required) | Met | GOVERNANCE.md names the maintainer and release decision maker, the successor maintainer and the contributor role, with each role's duties and limits (including AI assistants). | [GOVERNANCE.md, roles](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#roles-and-decisions) |
| `access_continuity` (URL required) | Met | Since 8 October 2026 a successor maintainer holds GitHub organization ownership (with 2FA), release-environment approval, ownership of every crate on crates.io and recovery access to the security inbox, verified through the APIs and a test issue (#440) and re-checked yearly. | [GOVERNANCE.md, continuity](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#continuity), [successor checklist](https://github.com/Rullst/Rullst/blob/main/docs/successor.md) |
| `bus_factor` (URL required) | Unmet | There is one active maintainer. The successor holds every credential needed to continue the project, but does not maintain it day to day, so fewer than two people can currently carry the work. | [GOVERNANCE.md](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#roles-and-decisions) |
| `documentation_roadmap` (URL required) | Met | ROADMAP.md describes what is planned (v12 maintenance, the v13 line and its priorities, ideas after 13.0) and what is not (abandoned milestones such as M39, work assigned to a separate programme). The v13 maintenance scope records what was removed and why. | [ROADMAP.md](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md), [v13 maintenance scope](https://rullst.github.io/Rullst/book/v13-maintenance-scope.html) |
| `documentation_architecture` (URL required) | Met | The framework specification documents the directory conventions, crates and capabilities and the core APIs; the architecture-decisions guide and security architecture describe the main components and their boundaries. | [Framework spec](https://rullst.github.io/Rullst/book/spec.html), [architecture choices](https://rullst.github.io/Rullst/book/architecture-decisions.html) |
| `documentation_security` (URL required) | Met | SECURITY.md, the security architecture, the security-layers guide and the claims ledger state what users can and cannot expect, including the limits of each control. | [Security architecture](https://rullst.github.io/Rullst/book/security-architecture.html), [which layer to use](https://rullst.github.io/Rullst/book/security-layers.html), [SECURITY.md](https://github.com/Rullst/Rullst/blob/main/SECURITY.md) |
| `documentation_quick_start` (URL required) | Met | The README quickstart and the "Start here" and "Zero to a complete app" guides take a new user from installation to a running application. | [README quickstart](https://github.com/Rullst/Rullst#quickstart), [Start here](https://rullst.github.io/Rullst/book/start-here.html) |
| `documentation_current` | Met | Guide code is compiled as doctests, and every documentation change runs the book build, offline link and anchor checks on Markdown and rendered HTML, a browser smoke test and a spell check. Documentation is updated with the behavior it describes. | [documentation.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/documentation.yml), [book doctests](https://github.com/Rullst/Rullst/blob/main/rullst/src/book_doctests.rs) |
| `documentation_achievements` (URL required) | Unmet until PR #447 merges | After PR #447: the README front page shows and links the OpenSSF Best Practices badge next to the OpenSSF Scorecard badge. | [README](https://github.com/Rullst/Rullst#readme) |
| `accessibility_best_practices` | Met | Browser smoke tests of the documentation site check the skip link, ARIA states, keyboard navigation and reduced motion. `cargo rullst audit --report` checks image text alternatives, form labels and page language in generated applications. No formal WCAG conformance is claimed. | [site-browser-smoke.mjs](https://github.com/Rullst/Rullst/blob/main/.github/site-browser-smoke.mjs), [audit report](https://rullst.github.io/Rullst/book/security-report.html) |
| `internationalization` | Unmet | Documentation, CLI and generated starters are English only. The framework has a few locale hooks (account mail in English, Portuguese and Spanish; a configurable HTML `lang`), but no general message-catalog localization. | [account mail](https://rullst.github.io/Rullst/book/account-mail-v12-1.html) |
| `sites_password_security` | N/A | The project's sites (GitHub repository and releases, the static GitHub Pages book, crates.io) store no passwords for external users; sign-in is handled by GitHub and crates.io. | [pages.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pages.yml) |

## Change Control

| Criterion | Answer | Justification to paste | Evidence |
| --- | --- | --- | --- |
| `maintenance_or_update` | Met | v12 is maintained on a protected branch with patch and minor releases while v13 develops on `main`. Deprecated APIs stay for at least one minor release, every major has a migration guide, and `cargo rullst upgrade` previews and applies upgrades. | [Compatibility policy](https://rullst.github.io/Rullst/book/compatibility-policy.html), [v13 migration](https://rullst.github.io/Rullst/book/migration-v13.html) |

## Reporting

| Criterion | Answer | Justification to paste | Evidence |
| --- | --- | --- | --- |
| `report_tracker` | Met | Individual issues are tracked in GitHub Issues with bug and feature templates; the bug template sends security reports to private disclosure. | [GitHub Issues](https://github.com/Rullst/Rullst/issues) |
| `vulnerability_report_credit` (URL required) | Met | The vulnerabilities fixed in the last 12 months were found by the project's own reviews, so there was no outside reporter to credit. SECURITY.md commits to crediting reporters in the changelog and advisories unless they ask for anonymity. | [SECURITY.md](https://github.com/Rullst/Rullst/blob/main/SECURITY.md) |
| `vulnerability_response_process` (URL required) | Met | SECURITY.md documents private reporting (email or GitHub private vulnerability reporting), what to include, response and patch targets by severity, and credit; the advisory-exceptions policy covers severity, ownership and temporary exceptions. | [SECURITY.md](https://github.com/Rullst/Rullst/blob/main/SECURITY.md), [advisory exceptions](https://rullst.github.io/Rullst/book/security-advisory-exceptions.html) |

## Quality

| Criterion | Answer | Justification to paste | Evidence |
| --- | --- | --- | --- |
| `coding_standards` (URL required) | Met | Rust code follows the standard Rust style (rustfmt defaults) and Clippy. CONTRIBUTING.md and AGENTS.md add project rules, such as no panics in production paths, parameterized SQL and Conventional Commits. | [CONTRIBUTING.md](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md#pull-requests--commit-guidelines), [AGENTS.md](https://github.com/Rullst/Rullst/blob/main/AGENTS.md) |
| `coding_standards_enforced` | Met | Every pull request runs `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`, plus zero-panic Clippy gates on library code. | [ci.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml), [zero-panics.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/zero-panics.yml) |
| `build_standard_variables` | Met | Builds use Cargo, which honors `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` and linker settings. There are no first-party build scripts or Makefiles that replace them; the only native C/C++ code (optional bundled DuckDB) is compiled by the `cc` crate, which reads `CC`, `CFLAGS`, `CXX` and `CXXFLAGS`. | [Cargo.toml](https://github.com/Rullst/Rullst/blob/main/Cargo.toml) |
| `build_preserve_debug` | Met | Debug information follows Cargo profiles, and the workspace does not force stripping. It can be requested with standard settings such as `CARGO_PROFILE_RELEASE_DEBUG=true`; the test profile omits DWARF by default to bound disk use and honors `CARGO_PROFILE_TEST_DEBUG`. | [Cargo.toml](https://github.com/Rullst/Rullst/blob/main/Cargo.toml) |
| `build_non_recursive` | Met | One Cargo workspace resolves and builds a single dependency graph; there are no recursive Makefiles or sub-builds. | [Cargo.toml](https://github.com/Rullst/Rullst/blob/main/Cargo.toml) |
| `build_repeatable` | Met | Published `.crate` archives are bit-for-bit reproducible from the same commit and pinned toolchain. CI repackages two crates from a fresh checkout on every pull request, the release job reproduces every archive before publication, and an independent rebuild matched the crates.io checksum of `rullst-macros` 12.2.0. Native CLI binaries are attested but not claimed reproducible. | [Reproducible crate archives](https://rullst.github.io/Rullst/book/reproducible-builds.html) |
| `installation_common` | Met | The CLI installs with `cargo install cargo-rullst --locked` and uninstalls with `cargo uninstall cargo-rullst`; libraries are added as normal Cargo dependencies. | [Getting started](https://rullst.github.io/Rullst/book/1-getting-started.html) |
| `installation_standard_variables` | Met | `cargo install` honors `CARGO_INSTALL_ROOT`, `--root` and `CARGO_HOME`, Cargo's conventions for the installation location. Libraries are never installed system-wide. | [Getting started](https://rullst.github.io/Rullst/book/1-getting-started.html) |
| `installation_development_quick` | Met | After cloning, rustup installs the toolchain pinned in `rust-toolchain.toml`, and `cargo build` and `cargo test` work; CONTRIBUTING.md lists the steps. | [CONTRIBUTING.md, development setup](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md#development-setup) |
| `external_dependencies` (URL required) | Met | The Cargo manifests and `Cargo.lock` list every dependency with exact versions and checksums; each release also attaches a CycloneDX SBOM. | [Cargo.lock](https://github.com/Rullst/Rullst/blob/main/Cargo.lock), [Cargo.toml](https://github.com/Rullst/Rullst/blob/main/Cargo.toml) |
| `dependency_monitoring` | Met | Dependabot checks Cargo and GitHub Actions weekly for `main` and `v12`. `cargo audit` runs on every push and pull request and daily, `cargo deny` on every push and pull request, and a release stops on any advisory. | [dependabot.yml](https://github.com/Rullst/Rullst/blob/main/.github/dependabot.yml), [audit.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/audit.yml), [cargo-deny.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/cargo-deny.yml) |
| `updateable_reused_components` | Met | Dependencies come from crates.io through Cargo and update with `cargo update` or Dependabot. The only vendored files are versioned htmx and Pico CSS browser assets (the version is in the file name), which are updated by hand. | [Cargo.toml](https://github.com/Rullst/Rullst/blob/main/Cargo.toml) |
| `interfaces_current` | Met | CI builds with `-D warnings`, so calling a deprecated dependency API fails the build. Every remaining `allow(deprecated)` is in a test of one of Rullst's own deprecated compatibility APIs. | [ci.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) |
| `automated_integration_testing` | Met | Every push and pull request to `main`, `v12` and `v13` runs the automated test suite in GitHub Actions and reports the result on the commit; a nightly run covers Linux, macOS and Windows. | [ci.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) |
| `regression_tests_added50` | Met | CONTRIBUTING.md requires a regression test for bug fixes. Of 1,139 `fix` commits on `main` and `v12` from 9 April to 9 October 2026, 972 (85%) changed or added tests. | [CONTRIBUTING.md](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md#pull-requests--commit-guidelines) |
| `test_statement_coverage80` | Met | Line coverage measured with cargo-llvm-cov was 92.56% on `main` (Codecov, 9 October 2026), and CI fails below 90%. | [Codecov](https://app.codecov.io/gh/Rullst/Rullst), [coverage.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml) |
| `test_policy_mandated` | Met | CONTRIBUTING.md: "Major new functionality must include automated tests of its public behavior, failure cases and relevant trust boundaries." | [CONTRIBUTING.md](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md#pull-requests--commit-guidelines) |
| `tests_documented_added` | Met | The test requirement is in CONTRIBUTING.md and in the pull-request template checklist. | [PULL_REQUEST_TEMPLATE.md](https://github.com/Rullst/Rullst/blob/main/.github/PULL_REQUEST_TEMPLATE.md) |
| `warnings_strict` | Met | All CI builds use `-D warnings`; Clippy runs on all targets and features with `-D warnings`, Core denies `clippy::unwrap_used` and `clippy::expect_used`, and zero-panic gates deny panicking calls in library code. | [ci.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) |

## Security

| Criterion | Answer | Justification to paste | Evidence |
| --- | --- | --- | --- |
| `implement_secure_design` | Met | The assurance case shows how each Saltzer and Schroeder principle is applied, with code: least privilege in workflows and API tokens, fail-safe defaults, complete mediation by the router-wide security baseline, separation of privilege in the release pipeline. | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html#secure-design-principles) |
| `crypto_weaknesses` | Met | Defaults are AES-256-GCM, HMAC-SHA256, Ed25519, ECDSA P-256 and Argon2id, with TLS through rustls only. SHA-1 appears only as HMAC-SHA1 for TOTP, as RFC 6238 and authenticator apps require. | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html#countering-common-implementation-weaknesses) |
| `crypto_algorithm_agility` | Unmet | Rullst deliberately uses one modern algorithm per purpose (AES-256-GCM, Argon2id, HMAC-SHA256, Ed25519; JWTs use HS256 only). Stored ciphertext carries key IDs, so keys rotate without code changes, but changing an algorithm needs a release; TLS negotiates among several rustls cipher suites. | [Vault](https://rullst.github.io/Rullst/book/tutorials/15-rullst-vault-encryption.html) |
| `crypto_credential_agility` | Met | Keys and credentials are read from the environment or configuration (`APP_KEY`, `RULLST_ENCRYPTION_KEY` or a keyring), never compiled in. Keyrings with key IDs allow rotation, and documented placeholder keys are refused. | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html#countering-common-implementation-weaknesses) |
| `crypto_used_network` | Met | Provider integrations use HTTPS through rustls, AI provider URLs must be HTTPS (or literal loopback for local models), and remote PostgreSQL requires `sslmode=verify-full`; the security headers add HSTS. | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html) |
| `crypto_tls12` | Met | TLS is provided by rustls, which supports TLS 1.2 and 1.3 only. | [Cargo.lock](https://github.com/Rullst/Rullst/blob/main/Cargo.lock) |
| `crypto_certificate_verification` | Met | rustls verifies certificates by default, and no first-party code disables verification (no `danger_accept_invalid_certs` or custom no-op verifier). | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html#countering-common-implementation-weaknesses) |
| `crypto_verification_private` | Met | Because certificate verification is never disabled, the TLS handshake is verified before any API key or bearer token is sent in an HTTP header. | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html) |
| `signed_releases` | Met | Since v12.0.0, every crate archive and evidence file (and, from v12.1.0, every CLI binary) is signed as a GitHub artifact attestation through Sigstore keyless signing, so no long-lived private key exists on any distribution site. From v12.1.2 the Sigstore bundle is attached; SECURITY.md documents verification with `gh attestation verify`. | [SECURITY.md, verifying a release](https://github.com/Rullst/Rullst/blob/main/SECURITY.md#verifying-a-release) |
| `version_tags_signed` | Unmet | Release tags are annotated but not signed; their integrity relies on protected branches, release admission of the exact commit and the attested artifacts. To start, sign tags with `git tag -s` using a GPG or SSH key registered with GitHub (or keyless with gitsign) and add `git verify-tag` to release admission. | [release.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml) |
| `input_validation` | Met | Requests are parsed into typed structures with `ValidatedJson` and `ValidatedForm`; identifiers, paths, storage keys and operators are checked against allowlists, and invalid input is rejected with a typed error. | [Forms and validation](https://rullst.github.io/Rullst/book/tutorials/07-forms-and-validation.html), [assurance case](https://rullst.github.io/Rullst/book/assurance-case.html#countering-common-implementation-weaknesses) |
| `hardening` | Met | Secure headers (HSTS, nonce CSP, frame denial, COOP, COEP, CORP), `unsafe` limited to five reviewed files, `forbid(unsafe_code)` in four crates, zero-panic Clippy gates, bounded JSON size and depth, and fail-closed defaults. | [Security architecture](https://rullst.github.io/Rullst/book/security-architecture.html) |
| `assurance_case` (URL required) | Met | The assurance case summarizes the threat models, lists the trust boundaries, argues the secure design principles and maps common weaknesses to mechanisms and named tests, with an explicit limits section. | [Assurance case](https://rullst.github.io/Rullst/book/assurance-case.html) |

## Analysis

| Criterion | Answer | Justification to paste | Evidence |
| --- | --- | --- | --- |
| `static_analysis_common_vulnerabilities` | Met | CodeQL analyzes Rust and JavaScript/TypeScript after every push to `main`, on pull requests to the maintained lines and weekly. Clippy, `cargo audit`, `cargo deny` and the IDOR route audit also run in CI. | [codeql.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml) |
| `dynamic_analysis_unsafe` | Met | Rust is memory-safe outside `unsafe`, which is limited to five reviewed OS-integration files. AddressSanitizer and ThreadSanitizer run nightly on twelve crates; Miri, Kani and 42 fuzz targets run as on-demand campaigns. | [sanitizers.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml), [fuzzing.yml](https://github.com/Rullst/Rullst/blob/main/.github/workflows/fuzzing.yml) |

## Summary

| Answer | Count | Criteria |
| --- | ---: | --- |
| Met | 48 | All others |
| Unmet | 6 | `documentation_achievements` (MUST, until PR #447 merges), `dco`, `bus_factor`, `internationalization`, `crypto_algorithm_agility` (SHOULD), `version_tags_signed` (SUGGESTED) |
| N/A | 1 | `sites_password_security` |

Once PR #447 is merged, every MUST criterion is answered Met. The unmet
SHOULD and SUGGESTED criteria do not block Silver if their justifications are
filled in. See also the [OpenSSF Scorecard evidence](openssf-scorecard.md).
