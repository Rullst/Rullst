# Rullst CI/CD & Security Workflows 🛡️

Rullst is built with a **"Zero-Panic Policy"** and designed for **production edge infrastructure**. To guarantee that the framework remains 100% memory-safe, blazing fast, and secure, we employ an industrial-grade CI/CD pipeline. 

This document explains every single automated workflow, what it protects, and when it runs.

---

## 🚀 Fast Feedback Loop (Runs on `push` and `pull_request`)

These workflows act as our primary gatekeepers. They run in parallel on every Pull Request and Push to the `main` branch. They are heavily cached and designed to finish in **under 10 minutes**, ensuring developers get immediate feedback without blocking the review process.

### 1. Core Integration & Tests (`ci.yml`)
- **What it does:** The foundation of our pipeline. It runs `cargo test` across the entire workspace, including unit tests, integration tests, and macro expansion tests.
- **DB Matrix:** Spins up Dockerized PostgreSQL and MySQL containers (via Testcontainers) to validate `rullst-orm` queries against real databases.
- **Lints & Style:** Enforces `rustfmt` formatting and strictly denies `clippy` warnings.
- **MSRV:** Verifies that Rullst compiles on our Minimum Supported Rust Version (currently `1.96.0`).

### 2. Performance Regression (`bench.yml`)
- **What it does:** Runs `cargo bench` (Criterion) across 5 independent benchmark suites:
  - **Core Framework:** HTML rendering and Router overhead.
  - **ORM:** Query builder and row mapping speeds.
  - **Primitives:** PII masking, HTML escaping, and CSRF token generation.
  - **Auth:** AES-256-GCM session encryption/decryption costs per request.
  - **Connect:** OAuth provider construction and PKCE hashing.
- **Goal:** Ensures zero nanosecond-level regressions ever reach the main branch. Any performance drop alerts the team automatically.

### 3. Code Coverage (`coverage.yml`)
- **What it does:** Uses `cargo-llvm-cov` to generate a comprehensive execution trace of our test suite.
- **Goal:** Ensures that new features have adequate test coverage before being merged. Results are uploaded to Codecov.

### 4. Zero-Panics Policy (`zero-panics.yml`)
- **What it does:** Uses custom Clippy configurations to forbid `.unwrap()`, `.expect()`, and `panic!()` in production crates (`rullst`, `rullst-orm`, `rullst-core`, etc.).
- **Goal:** Guarantees that the framework handles all errors gracefully via the typed `AppError` system, preventing the server from ever crashing at runtime.

### 5. Memory Safety & Unsafe Policy (`unsafe-policy.yml`)
- **What it does:** Scans the codebase for `unsafe` blocks. If an `unsafe` block is found without a corresponding `// SAFETY:` justification comment, the CI fails immediately.
- **Goal:** Rullst is 100% memory-safe. This workflow ensures any interaction with FFI or raw pointers is heavily audited and documented.

### 6. Semantic Security Analysis (`codeql.yml`)
- **What it does:** Uses GitHub's Advanced Security (CodeQL) engine to semantically analyze the Rust AST for logical bugs, memory leaks, and injection vectors.

### 7. Secret Scanning (`trufflehog.yml`)
- **What it does:** Scans the entire git history for accidentally committed API keys, secrets, or passwords. Runs in seconds.

### 8. Dependency Health (`cargo-deny.yml` & `machete.yml`)
- **What it does:** Validates the licenses of all third-party crates, bans unmaintained dependencies, checks the RustSec vulnerability database, and prunes unused crates from `Cargo.toml`.

### 9. Architecture Linter (`tangleguard.yml`)
- **What it does:** Runs `cargo-tangleguard` against `tangleguard.toml` rules.
- **Goal:** Analyzes our crates to ensure there are no inverse or circular dependencies between our internal modules (e.g. `rullst-core` must never depend on `rullst-orm`), preventing "spaghetti code" from accumulating.

---

## 📆 Scheduled & Release Operations (Asynchronous)

These workflows handle continuous validation and artifact generation without blocking developer PRs.

### 1. Property-Based Testing (`proptest.yml`)
- **What it does:** Runs our `proptest` suite, generating over 10,000 random inputs for complex string formatters and HTML macros to find obscure edge-case panics.
- **When it runs:** Every Sunday morning (Cron).

### 2. Documentation & Benchmark Dashboards (`pages.yml`)
- **What it does:** Compiles the mdBook documentation and the visual Chart.js dashboards for our 5 benchmark suites, deploying them to GitHub Pages.
- **When it runs:** Automatically on every merge to `main`.

### 3. Release Publication (`release.yml`)
- **What it does:** Orchestrates the secure publication of all 7 workspace crates to `crates.io`.
- **When it runs:** Manually triggered when a version tag (`v*.*.*`) is pushed.

---

## 🏋️ Extreme Verification (Manual `workflow_dispatch` Only)

These workflows perform rigorous, formal, and mathematically exhaustive verification of the Rullst architecture. Because they can take anywhere from **30 minutes to multiple hours**, they do not run on every push. They are executed manually by maintainers before major stable releases.

### 1. Fuzzing (`fuzzing.yml`)
- **What it does:** Uses `cargo-fuzz` and `libFuzzer` to feed infinite streams of garbage data into our HTTP parser, ORM schema builder, and JSON serializers.
- **Goal:** Ensures that maliciously crafted network packets cannot cause DoS attacks or panic the web server.

### 2. Symbolic Execution (`kani.yml`)
- **What it does:** Uses the AWS Kani Rust Verifier to perform **Model Checking**. It explores every possible execution path of our cryptography and memory-handling code.
- **Goal:** Proves mathematically that our core functions are free of overflow, out-of-bounds, and logical state errors.

### 3. Undefined Behavior Detection (`miri.yml`)
- **What it does:** Runs the entire test suite inside the Miri interpreter, tracking memory allocations at the byte level.
- **Goal:** Detects subtle Undefined Behavior (UB), use-after-free, and strict provenance violations in any asynchronous or unsafe code.

### 4. Mutation Testing (`mutants.yml`)
- **What it does:** Uses `cargo-mutants` to deliberately inject bugs into the Rullst source code (e.g., changing `==` to `!=` or removing function calls) and recompiles the project.
- **Goal:** Proves that our test suite is actually capable of catching bugs. If a mutant survives (i.e., the tests still pass), it means we have a gap in our coverage.

### 5. Dynamic App Security Testing (`dast-zap.yml`)
- **What it does:** Spins up the full framework in a Docker container and uses **OWASP ZAP** to spider the application, launching active attacks (SQLi, XSS, CSRF).
- **Goal:** Simulates a real-world black-box penetration test against the framework's default configurations.

---
*Rullst - Built for those who want to build securely and easily, but not suffer.*
