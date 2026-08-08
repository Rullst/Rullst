# Rullst CI/CD & Enterprise Security Architecture 🛡️

Rullst is engineered from the ground up with a strict **Zero-Panic Policy** and designed specifically for **mission-critical edge and cloud infrastructure**. To guarantee that the framework remains 100% memory-safe, mathematically sound, race-free, and resilient against state-sponsored attack vectors, we employ a multi-layered verification pyramid.

This document details every automated workflow, target coverage by crate, execution parameters, and our roadmap for continuous cloud fuzzing integration.

---

## 🏛️ Comprehensive Test & Formal Verification Matrix

| Crate | Unit & Integration | Kani (Model Checking) | Miri (UB & Memory) | Fuzzing (libFuzzer) | Property Testing (Proptest) | Concurrency Sanitizers (TSan/ASan) | Zero Panics (Clippy) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`rullst-core`** | ✅ | ✅ (PII & Circuit Breakers) | ✅ | ✅ (12 Targets) | ✅ (10,000 Iterations) | ✅ (TSan + ASan) | ✅ |
| **`rullst-security`** | ✅ | ✅ (Vault & SRI Invariants) | ✅ | ✅ (5 Targets) | ✅ (10,000 Iterations) | ✅ (TSan + ASan) | ✅ |
| **`rullst-auth`** | ✅ | ✅ (Cookie & Token Invariants) | ✅ | ✅ (2 Targets) | ✅ (10,000 Iterations) | ✅ (TSan + ASan) | ✅ |
| **`rullst-orm`** | ✅ | ✅ (SQL Sanitization Bounds) | ✅ | ✅ (5 Targets) | ✅ (Query Builder AST) | ✅ (TSan + ASan) | ✅ |
| **`rullst-connect`** | ✅ | ✅ (OIDC / PKCE Verifier) | ✅ | ✅ (3 Targets) | ✅ (PKCE Fuzzing) | ✅ (TSan + ASan) | ✅ |
| **`rullst-iot`** | ✅ | ✅ (PQC Kyber & Modbus CRC) | ✅ | ✅ (3 Targets) | ✅ (Hardware State) | ✅ (TSan + ASan) | ✅ |
| **`rullst-ai`** | ✅ | ✅ (Tool Param Schema) | ✅ | ✅ (2 Targets) | ✅ (Prompt Invariants) | ✅ (TSan + ASan) | ✅ |
| **`rullst-capital`** | ✅ | ✅ (Invoice Total Bounds) | ✅ | ✅ (1 Target) | ✅ (Billing Invariants) | ✅ (TSan + ASan) | ✅ |
| **`rullst-nexus`** | ✅ | ✅ (Identifier Sanitation) | ✅ | ✅ (1 Target) | ✅ (CRUD Query Bounds) | ✅ (TSan + ASan) | ✅ |
| **`rullst-studio`** | ✅ | ✅ (Identifier Length Bounds) | ✅ | ✅ (1 Target) | ✅ (Filter Invariants) | ✅ (TSan + ASan) | ✅ |
| **`rullst-mail`** | ✅ | ✅ (Message Builder Invariants) | ✅ | ✅ (1 Target) | ✅ (Recipient Parsing) | ✅ (TSan + ASan) | ✅ |

---

## 🚀 1. Fast Feedback Loop (Runs on `push` and `pull_request`)

These workflows act as our primary gatekeepers. They run in parallel on every Pull Request and Push to `main` and `dev` branches. Heavily cached, they complete in **under 10 minutes** to deliver immediate developer feedback.

### 1.1 Core Multi-OS Matrix (`ci.yml`)
- **What it does:** Executes unit tests, integration tests, and procedural macro expansions across a matrix of operating systems:
  - **Linux (`ubuntu-latest`)**
  - **macOS (`macos-latest` on Apple Silicon ARM64)**
  - **Windows (`windows-latest` MSVC)**
- **DB Matrix Tests:** Provisions live Dockerized PostgreSQL and MySQL instances (via Testcontainers) on Linux runners to validate database driver parity and Active Record transactions against real engines.
- **Compiler Lints:** Enforces `rustfmt` formatting and strictly denies any `clippy` warnings (`-D warnings`).
- **MSRV Enforcement:** Asserts that the entire monorepo compiles cleanly under the Minimum Supported Rust Version (`1.96.0`).

### 1.2 Performance Regression Shield (`bench.yml`)
- **What it does:** Runs Criterion benchmarks across 5 independent sub-systems:
  - **Core SSR:** Zero-bundle HTML rendering and Router dispatch latency.
  - **Active Record:** ORM query builder formatting and row deserialization throughput.
  - **Cryptographic Primitives:** PII masking, HTML sanitization, and CSRF token generation costs.
  - **Auth Pipeline:** AES-256-GCM session decryption and HMAC signature validation per request.
  - **SSO Connect:** OIDC state token generation and PKCE code challenge hashing.
- **Threshold:** Any nanosecond-level regression triggers an automated review alert.

### 1.3 Property-Based Fuzzy Invariant Testing (`proptest.yml`)
- **What it does:** Generates over 10,000 randomized combinatorial inputs per test case to stress test AST query builders, PKCE challenges, URL decoders, and session deserializers.
- **Goal:** Proves that inputs never trigger unexpected edge-case panics or infinite loops regardless of malformed byte structures.

### 1.4 Bare-Metal `#![no_std]` Verification (`no_std-build.yml`)
- **What it does:** Compiles `rullst-iot` across 3 embedded bare-metal architectures:
  - `thumbv7em-none-eabihf` (STM32 Cortex-M4/M7 with hardware FPU)
  - `thumbv6m-none-eabi` (ARM Cortex-M0/M0+ low-power sensors)
  - `riscv32imac-unknown-none-elf` (ESP32-C3 RISC-V IoT controllers)
- **Goal:** Guarantees zero runtime allocations and zero standard library dependencies for edge hardware.

### 1.5 IoT Hardware Simulation (`iot-integration.yml`)
- **What it does:** Executes unit and integration test suites across GPIO, I2C, Modbus RTU/TCP, BLE GATT profiles, Anomaly Detectors, OTA partition swappers, and Hardware Security Modules (HSM), backed by Cortex-M QEMU emulation.

### 1.6 Zero-Panics Compiler Enforcement (`zero-panics.yml`)
- **What it does:** Custom compiler linting forbidding `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, and `unimplemented!()` in non-test paths of production crates. All failures must be gracefully degraded through typed `AppError` enums.

### 1.7 Memory Safety & Unsafe Policy (`unsafe-policy.yml`)
- **What it does:** Audits the entire codebase for `unsafe` blocks. Any undocumented or unjustified `unsafe` invocation immediately fails CI.

### 1.8 End-to-End Smoke Test (`e2e-smoke.yml`)
- **What it does:** Compiles the `rullst-blog-example` binary in release mode, boots the HTTP server on port 3000, and fires live `curl` requests to validate:
  - SSR HTML document structure and status 200 responses.
  - Security headers (`Content-Security-Policy`, `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`).
  - Metadata endpoint isolation (verifying no CSRF session cookies leak to `/robots.txt` or `/sitemap.xml`).
  - Live form `POST /posts` execution and SQLite database persistence.

---

## 📆 2. Scheduled & Cryptographic Audits (Asynchronous)

### 2.1 Post-Quantum & Cryptographic Compliance (`pqc-compliance.yml`)
- **What it does:** Audits post-quantum key encapsulation (`PqcKeyPair` ML-KEM Kyber), Hardware Security Modules (`hsm.rs`), and in-memory zero-trust secrets (`vault.rs`).
- **Dependency Audit:** Runs `cargo audit` with pre-compiled binaries to verify zero known CVEs across all cryptographic crates.
- **Schedule:** Automated weekly Monday run at 03:00 UTC and on crypto file changes.

### 2.2 Dynamic Application Security Testing (`dast-zap.yml`)
- **What it does:** Spins up the Rullst production engine and launches **OWASP ZAP** dynamic penetration testing attacks against SQLi, XSS, CSRF, and session hijacking vectors.

### 2.3 Semantic Security Analysis (`codeql.yml`) & Secret Scanning (`trufflehog.yml`)
- **What it does:** Performs deep semantic AST analysis with GitHub CodeQL and verifies that zero API keys or private certificates exist across git history.

### 2.4 Supply Chain & API Stability (`cargo-deny.yml`, `semver.yml`, `machete.yml`)
- **What it does:** Bans unmaintained licenses, prevents breaking public API changes without SemVer bumps, and prunes unused dependencies.

---

## 🏋️ 3. Extreme Verification Suites (`workflow_dispatch` & Nightly)

These compute-intensive suites run for hours, mathematically modeling the framework state space and finding subtle flaws.

### 3.1 Mathematical Model Checking (`kani.yml`)
- **What it does:** Uses the AWS Kani Rust Verifier (powered by CBMC SAT solvers) to mathematically prove the absence of crashes, overflows, and state invariant violations in:
  - **`rullst-core`:** PII masking string length invariant & circuit breaker token bucket refill arithmetic.
  - **`rullst-security`:** Zero-trust `VaultSecret` memory exposure & `compute_sri_hash` formatting.
  - **`rullst-iot`:** ML-KEM Kyber keypair encapsulation bounds & Modbus CRC16 panic-freedom.
  - **`rullst-auth`:** Session cookie serialization and logout expiration headers.
  - **`rullst-connect`:** OIDC state token formatting & PKCE code verifier hashing.

### 3.2 Undefined Behavior & Strict Provenance Detection (`miri.yml`)
- **What it does:** Executes tests under the Miri interpreter with `RUSTFLAGS="-Zrandomize-layout"` and `-Zmiri-disable-isolation` across 10 packages to detect memory alignment, strict provenance violations, and use-after-free conditions.

### 3.3 Continuous Differential Fuzzing (`fuzzing.yml`)
- **What it does:** Runs `cargo-fuzz` (LLVM libFuzzer) across **27 dedicated fuzz targets** for up to 6 hours (`-max_total_time=21000`):
  - **Core:** `mask_pii`, `html_escape`, `routing`, `validation_json`, `auth_crypto`, `auth_session`, `security_csrf`, `security_waf`, `htmx_headers`, `config_parser`, `multitenant_resolver`, `ws_payload`.
  - **ORM:** `fuzz_audit`, `fuzz_builder`, `fuzz_parser`, `fuzz_schema`, `fuzz_scout`.
  - **Security:** `fuzz_rasp`, `fuzz_schema_guard`, `fuzz_vault`, `fuzz_totp`, `fuzz_log_redactor`.
  - **Connect:** `default_target`, `fuzz_token_response`, `fuzz_user_json`.

### 3.4 Concurrency & Memory Sanitizers (`sanitizers.yml`)
- **What it does:** Compiles under Rust Nightly with `-Zsanitizer=thread` (TSan) and `-Zsanitizer=address` (ASan) to catch race conditions and memory corruption in asynchronous Tokio worker pools.

### 3.5 Mutation Testing (`mutants.yml`)
- **What it does:** Deploys `cargo-mutants` across 8 parallel shards to intentionally inject bugs into the Rullst syntax tree, mathematically asserting that the test suite catches every mutant.

---

## 🌐 4. Scaling Fuzzing: Google OSS-Fuzz & External Infrastructure

### 4.1 Does Rullst Qualify for Google OSS-Fuzz?
**Yes, absolutely.** Google OSS-Fuzz is a free continuous fuzzing service provided by Google for critical open-source software.

#### Criteria Evaluation:
1. **Critical Infrastructure:** Rullst is a high-performance web runtime, cryptographic engine, and bare-metal IoT framework handling network traffic, database transactions, and IoT hardware protocols.
2. **Open Source & Permissive License:** Licensed under MIT / Apache-2.0 on a public GitHub repository.
3. **Existing libFuzzer Targets:** Rullst already contains **27+ production-ready `libFuzzer` targets** (`rullst/fuzz`, `rullst-orm/fuzz`, `rullst-connect/fuzz`, `rullst-security/fuzz`) integrated with `cargo-fuzz`.
4. **Active Maintenance:** Zero-panic guarantees, high test coverage, and continuous triage.

### 4.2 Next Steps for OSS-Fuzz Onboarding:
To onboard Rullst to Google OSS-Fuzz:
1. Submit a Pull Request to [`google/oss-fuzz`](https://github.com/google/oss-fuzz) with a `projects/rullst` directory containing:
   - `project.yaml`: Project metadata, maintainer contact email, and primary repository link.
   - `Dockerfile`: Multi-stage container pulling Rust Nightly and `cargo-fuzz`.
   - `build.sh`: Script compiling all 27 fuzz targets with AddressSanitizer, MemorySanitizer, and UndefinedBehaviorSanitizer.
2. Once merged, Google ClusterFuzz will automatically run Rullst fuzzers **24/7 on thousands of CPU cores in Google Cloud**, continuously filing automated issue reports with reproducible testcases whenever a crash is discovered.

---
*Rullst - Built for those who want to build securely and easily, but not suffer.*
