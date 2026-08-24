# Rullst CI/CD & Enterprise Security Architecture 🛡️

Rullst is engineered from the ground up with a strict **Zero-Panic Policy** and designed specifically for **mission-critical edge and cloud infrastructure**. To guarantee that the framework remains 100% memory-safe, mathematically sound, race-free, and resilient against state-sponsored attack vectors, we employ a multi-layered verification pyramid.

This document details every automated workflow, target coverage by crate, execution parameters, and our roadmap for continuous cloud fuzzing integration.

---

## 🧠 Verification Paradigms & Core Technology Comparison

To ensure mission-critical resilience, Rullst does not rely on a single verification vector. Instead, 8 distinct, complementary testing paradigms are employed simultaneously:

| Mechanism | Primary Question | Verification Target | Engine / Tool |
| :--- | :--- | :--- | :--- |
| **Code Coverage** | *"Which code branches were executed?"* | **Quantity** of tested branches & lines | `cargo-llvm-cov` + Codecov |
| **Mutation Testing** | *"Do our assertions actually catch bugs?"* | **Quality** and sensitivity of test assertions | `cargo-mutants` (8 shards) |
| **Formal Verification** | *"Is there any input in the universe that breaks invariants?"* | **Mathematical Proof** of safety contracts | `kani-verifier` (SAT/SMT) |
| **Undefined Behavior** | *"Are unsafe blocks and pointer aliasing 100% sound?"* | **Memory Safety** & Stacked Borrows | `cargo miri` (MIR Interpreter) |
| **Memory Safety Audit** | *"Are dependencies and code free of unsafe blocks?"* | **Zero-Unsafe Invariant** & Supply Chain Safety | `cargo-geiger` + `cargo rullst audit --geiger` |
| **Fuzz Testing & Corpus** | *"How does the system react to chaotic/malicious payloads?"* | **Parser Robustness** & Crash Resistance | `cargo-fuzz` / `libFuzzer` + `cmin` |
| **Property Testing** | *"Do state invariants hold under thousands of random combinations?"* | **State Machine Integrity** | `proptest` (QuickCheck-style) |
| **Supply Chain & Provenance** | *"Are build artifacts signed and dependencies free of CVEs?"* | **Cryptographic Attestation & SBOM** | `CycloneDX 1.5 JSON`, `Sigstore Cosign`, `SLSA 3` |

### 📐 The Multi-Layer Safety Pyramid

```
         ▲  [8. SIGSTORE / SLSA 3] Cryptographic Provenance & CycloneDX 1.5 SBOM
        ▲▲  [7. KANI] Formal Mathematical Model Checking (Zero Panics / Invariants)
       ▲▲▲  [6. MIRI] Undefined Behavior & Strict Pointer Provenance
      ▲▲▲▲  [5. MUTANTS] Test Assertion Mutation Quality & Sensitivity
     ▲▲▲▲▲  [4. FUZZING & CORPUS SYNC] Continuous Chaos Ingestion & Compaction
    ▲▲▲▲▲▲  [3. PROPTEST] 10,000+ Iteration Combinatorial Property Validation
   ▲▲▲▲▲▲▲  [2. SANITIZERS (ASan/TSan)] Memory & Concurrency Data-Race Detection
  ▲▲▲▲▲▲▲▲  [1. CODECOV] 90%+ Deterministic LLVM Multi-OS Code Coverage
```

> **Coverage Target Guard:** Rullst enforces a strict **90% code coverage** threshold (project and patch) with a **1% tolerance** via Codecov. All AI provider integrations (`rullst-ai`) implement deterministic offline mock fallbacks per [AGENTS.md §3.5](AGENTS.md), ensuring 100% test reliability without live API credentials.

---

## 🏛️ Comprehensive Test & Formal Verification Matrix

| Crate | Unit & Integration | Kani (Model Checking) | Miri (UB & Memory) | Fuzzing (libFuzzer) | Property Testing (Proptest) | Concurrency & Race-Freedom | AST Security & IDOR | Pure-Rustls Native |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`rullst-core`** | ✅ | ✅ (PII & Circuit Breakers) | ✅ | ✅ (12 Targets) | ✅ (10,000 Iterations) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-security`** | ✅ | ✅ (Vault & SRI Invariants) | ✅ | ✅ (5 Targets) | ✅ (10,000 Iterations) | ✅ (High-Contention) | ✅ | ✅ |
| **`rullst-auth`** | ✅ | ✅ (Cookie & Token Invariants) | ✅ | ✅ (2 Targets) | ✅ (10,000 Iterations) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-orm`** | ✅ | ✅ (SQL Sanitization Bounds) | ✅ | ✅ (5 Targets) | ✅ (Query Builder AST) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-connect`** | ✅ | ✅ (OIDC / PKCE Verifier) | ✅ | ✅ (3 Targets) | ✅ (PKCE Fuzzing) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-iot`** | ✅ | ✅ (PQC Kyber & Modbus CRC) | ✅ | ✅ (3 Targets) | ✅ (Hardware State) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-ai`** | ✅ | ✅ (Tool Param Schema) | ✅ | ✅ (3 Targets) | ✅ (Prompt Invariants) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-capital`** | ✅ | ✅ (Invoice Total Bounds) | ✅ | ✅ (1 Target) | ✅ (Billing Invariants) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-nexus`** | ✅ | ✅ (Identifier Sanitation) | ✅ | ✅ (1 Target) | ✅ (CRUD Query Bounds) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-studio`** | ✅ | ✅ (Identifier Length Bounds) | ✅ | ✅ (1 Target) | ✅ (Filter Invariants) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`rullst-mail`** | ✅ | ✅ (Message Builder Invariants) | ✅ | ✅ (4 Targets) | ✅ (Recipient Parsing) | ✅ (TSan + ASan) | ✅ | ✅ |
| **`cargo-rullst`** | ✅ | ✅ (Scaffold Validation) | ✅ | ✅ (AST Generator) | ✅ (Parser Invariants) | ✅ (Multi-Threaded) | ✅ | ✅ |
| **`rullst`** | ✅ | ✅ (Umbrella Integration) | ✅ | ✅ (Full Stack) | ✅ (End-to-End AST) | ✅ (TSan + ASan) | ✅ | ✅ |

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
- **What it does:** Runs `cargo-fuzz` (LLVM libFuzzer) across **33 dedicated fuzz targets** for up to 6 hours (`-max_total_time=21000`):
  - **Core:** `mask_pii`, `html_escape`, `routing`, `validation_json`, `auth_crypto`, `auth_session`, `security_csrf`, `security_waf`, `htmx_headers`, `config_parser`, `multitenant_resolver`, `ws_payload`.
  - **ORM:** `fuzz_audit`, `fuzz_builder`, `fuzz_parser`, `fuzz_schema`, `fuzz_scout`.
  - **Security:** `fuzz_rasp`, `fuzz_schema_guard`, `fuzz_vault`, `fuzz_totp`, `fuzz_log_redactor`, `fuzz_dlp`, `fuzz_sanitizer`.
  - **Connect:** `default_target`, `fuzz_token_response`, `fuzz_user_json`.
  - **Mail:** `fuzz_mail`, `fuzz_email_validator`, `fuzz_email_tracking`, `fuzz_email_security`.
  - **AI:** `fuzz_ai_tools`, `fuzz_rag`, `fuzz_message_serde`.
  - **IoT:** `fuzz_kyber`, `fuzz_modbus`, `fuzz_sensor_packet`.

### 3.4 Concurrency & Memory Sanitizers (`sanitizers.yml`)
- **What it does:** Compiles under Rust Nightly with `-Zsanitizer=thread` (TSan) and `-Zsanitizer=address` (ASan) to catch race conditions and memory corruption in asynchronous Tokio worker pools.
- **Triggers:** Automated weekly schedule, on-demand manual trigger via `workflow_dispatch`, and pull requests affecting core runtime crates.

### 3.5 Mutation Testing (`mutants.yml`)
- **What it does:** Deploys `cargo-mutants` across 8 parallel shards to intentionally inject bugs into the Rullst syntax tree, mathematically asserting that the test suite catches every mutant.

---

## 🌐 4. Scaling Fuzzing: Google OSS-Fuzz & External Infrastructure

### 4.1 Does Rullst Qualify for Google OSS-Fuzz?
**Yes, absolutely.** Google OSS-Fuzz is a free continuous fuzzing service provided by Google for critical open-source software.

#### Criteria Evaluation:
1. **Critical Infrastructure:** Rullst is a high-performance web runtime, cryptographic engine, and bare-metal IoT framework handling network traffic, database transactions, and IoT hardware protocols.
2. **Open Source & Permissive License:** Licensed under MIT on a public GitHub repository.
3. **Existing libFuzzer Targets:** Rullst already contains **33+ production-ready `libFuzzer` targets** (`rullst/fuzz`, `rullst-orm/fuzz`, `rullst-security/fuzz`, `rullst-mail/fuzz`, `rullst-connect/fuzz`, `rullst-ai/fuzz`, `rullst-iot/fuzz`) integrated with `cargo-fuzz`.
4. **Active Maintenance:** Zero-panic guarantees, high test coverage, and continuous triage.

### 4.2 Standard 3-Step OSS-Fuzz Onboarding Strategy

To guarantee a 100% first-pass acceptance rate when onboarding Rullst to Google OSS-Fuzz, follow this structured execution plan:

#### Step 1: GitHub Actions CI/CD Baseline Validation (Internal)
Before initiating any external submission, ensure that all internal CI checks and fuzz targets pass 100% cleanly on GitHub:
- Verify that [`.github/workflows/fuzzing.yml`](.github/workflows/fuzzing.yml) completes without a single compilation error or unexpected panic across all 33 targets.
- Ensure that `cargo fmt --all -- --check` and `cargo clippy --workspace --all-features -- -D warnings` pass with 0 warnings.

#### Step 2: Local Pre-Flight Docker Verification with `helper.py`
Google OSS-Fuzz requires that the build configuration conforms to their internal Clang sanitizers environment. Test the build locally before opening the PR:

1. Clone the official Google OSS-Fuzz repository:
   ```bash
   git clone https://github.com/google/oss-fuzz.git
   cd oss-fuzz
   ```

2. Add the Rullst integration directory at `projects/rullst/` containing:
   - `project.yaml`: Maintainer contact email (`officialrullst@gmail.com`), primary language (`rust`), and repository URL.
   - `Dockerfile`: Pulls `gcr.io/oss-fuzz-base/base-builder-rust` and repository dependencies.
   - `build.sh`: Compiles all 33 fuzz targets using `cargo fuzz build -O --debug-assertions` and copies the resulting binaries to `$OUT/`.

3. Run the official Google helper verification commands locally with Docker:
   ```bash
   # Build the container image
   python infra/helper.py build_image rullst

   # Compile the fuzz targets under AddressSanitizer (ASan)
   python infra/helper.py build_fuzzers --sanitizer address rullst

   # Run automated pre-flight checks (asserts all targets boot and run without initial crashes)
   python infra/helper.py check_build --sanitizer address rullst
   ```

#### Step 3: Submit Pull Request & Automated ClusterFuzz Ingestion
1. Submit a Pull Request to [`google/oss-fuzz`](https://github.com/google/oss-fuzz) with the `projects/rullst` directory.
2. The Google CI bot will automatically execute `check_build` on the PR and approve the merge.
3. Once merged, Google ClusterFuzz will automatically run Rullst fuzzers **24/7 on thousands of CPU cores in Google Cloud**, continuously filing automated issue reports with reproducible testcases whenever a crash is discovered.

---

## 📊 5. Complete CI/CD Workflows Classification Matrix

To maintain ultra-fast feedback loops on daily pushes while retaining military-grade formal verification, the 32 GitHub Actions workflows in Rullst are organized into 4 distinct execution tiers:

| Workflow | File | Execution Mode | Trigger Events | Primary Responsibility | Typical Duration |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Rust CI Matrix** | [`ci.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ci.yml) | ⚡ **Automatic** | `push`, `pull_request` | Multi-OS build (Ubuntu, macOS, Windows), Clippy `-D warnings`, Rustfmt | ~1.5 min |
| **TangleGuard** | [`tangleguard.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/tangleguard.yml) | ⚡ **Automatic** | `push`, `pull_request` | Architecture linter enforcing zero circular dependencies | ~20s |
| **Zero Panics Policy** | [`zero-panics.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/zero-panics.yml) | ⚡ **Automatic** | `push`, `pull_request` | Forbids `.unwrap()`, `.expect()`, and `panic!()` in non-test paths | ~45s |
| **Unsafe Policy** | [`unsafe-policy.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/unsafe-policy.yml) | ⚡ **Automatic** | `push`, `pull_request` | Enforces `#![forbid(unsafe_code)]` compliance across workspace | ~30s |
| **E2E Smoke Tests** | [`e2e-smoke.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/e2e-smoke.yml) | ⚡ **Automatic** | `push`, `pull_request`, `dispatch` | Full-stack SSR, live server boot, and security header verification | ~1 min |
| **IoT Edge Pipeline** | [`iot-integration.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/iot-integration.yml) | ⚡ **Automatic** | `push`, `pull_request` | MQTT 5.0 broker and industrial sensor ingestion telemetry | ~1 min |
| **Bare-Metal `no_std`** | [`no_std-build.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/no_std-build.yml) | ⚡ **Automatic** | `push`, `pull_request` | Embedded ARM Cortex-M4 (`thumbv7em`) compilation checks | ~45s |
| **TruffleHog Scanner** | [`trufflehog.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/trufflehog.yml) | ⚡ **Automatic** | `push`, `pull_request` | Deep git commit secret and credential leak scanning | ~20s |
| **Cargo Machete** | [`machete.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/machete.yml) | ⚡ **Automatic** | `push`, `pull_request` | Scans and rejects unused dependencies in all `Cargo.toml` files | ~25s |
| **Spellcheck** | [`spellcheck.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/spellcheck.yml) | ⚡ **Automatic** | `push`, `pull_request` | Typo and documentation spelling checks | ~15s |
| **Security Audit** | [`security-audit.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/security-audit.yml) | 🔄 **Hybrid** | `push` (main), Daily schedule, `dispatch` | RustSec CVE vulnerability scan across Cargo.lock | ~1 min |
| **Cargo Audit** | [`audit.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/audit.yml) | 🔄 **Hybrid** | `push` (main), Daily schedule, `dispatch` | Automated advisory database synchronization and audits | ~1 min |
| **PQC & HSM Compliance** | [`pqc-compliance.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pqc-compliance.yml) | 🔄 **Hybrid** | `push`, `pull_request`, Weekly, `dispatch` | Post-Quantum (ML-KEM/ML-DSA) and crypto invariant verification | ~1 min |
| **Code Coverage (LLVM)** | [`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml) | 🔄 **Hybrid** | `push` (main), `pull_request`, `dispatch` | LLVM source-based code coverage report and badge generator | ~2 min |
| **CodeQL Analysis** | [`codeql.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml) | 🔄 **Hybrid** | `push` (main), `pull_request`, Weekly schedule | GitHub Advanced Security deep SAST static code analyzer | ~3 min |
| **SemVer Check** | [`semver.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/semver.yml) | 🔄 **Hybrid** | `push` (main), `pull_request`, `dispatch` | Cargo Semver checks for breaking API signature changes | ~1.5 min |
| **Cargo Deny** | [`cargo-deny.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/cargo-deny.yml) | 🔄 **Hybrid** | Weekly schedule (Sunday), `dispatch` | Supply chain, license compatibility (MIT/Apache), and duplicate deps | ~45s |
| **OpenSSF Scorecards** | [`scorecards.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/scorecards.yml) | 🔄 **Hybrid** | `push` (main), Weekly schedule, `dispatch` | Supply Chain Levels for Software Artifacts (SLSA) compliance | ~1 min |
| **DAST ZAP Scanner** | [`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml) | 🔄 **Hybrid** | `push` (main), `dispatch` | OWASP ZAP dynamic application vulnerability penetration scan | ~3 min |
| **Clang Sanitizers** | [`sanitizers.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml) | 🔄 **Hybrid** | `push`, `pull_request`, Weekly, `dispatch` | Clang ASan, TSan, and MSan undefined behavior scanners | ~4 min |
| **Fuzzing Matrix (33 Targets)** | [`fuzzing.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/fuzzing.yml) | ⏱️ **Manual** | Manual `workflow_dispatch` | Continuous libFuzzer execution across all 33 crate targets | ~5.8h / target |
| **Property Testing (Proptest)** | [`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml) | ⏱️ **Manual / Scheduled** | Manual `workflow_dispatch`, Weekly (Sun 06:00 UTC) | 10,000+ iteration state-machine & invariant property tests | ~20 min |
| **Benchmark Regression** | [`bench.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/bench.yml) | ⏱️ **Manual / Scheduled** | Manual `workflow_dispatch`, Weekly (Mon 04:00 UTC) | Criterion throughput & memory micro-benchmarks regression suite | ~10 min |
| **Formal Verification (Kani)** | [`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml) | ⏱️ **Manual / Scheduled** | Manual `workflow_dispatch`, Weekly (Sun 00:00 UTC) | AWS Kani model checker for mathematical proofs on crypto/bounds | ~15 min |
| **Miri UB Interpreter** | [`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml) | ⏱️ **Manual / Scheduled** | Manual `workflow_dispatch`, Weekly (Sun 02:00 UTC) | Stacked Borrows memory model & unaligned memory access interpreter | ~12 min |
| **Mutation Testing** | [`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml) | ⏱️ **Manual / Scheduled** | Manual `workflow_dispatch`, Weekly (Sun 04:00 UTC) | Cargo-mutants injecting AST mutants across 8 parallel shards | ~25 min |
| **Wasm & Edge Matrix** | [`wasm-matrix.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/wasm-matrix.yml) | ⚡ **Automatic** | `push`, `pull_request` | Validates `wasm32-unknown-unknown` and `wasm32-wasip1` targets | ~1 min |
| **AI PR Sentinel Audit** | [`ai-sentinel-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/ai-sentinel-pr.yml) | ⚡ **Automatic** | `pull_request` | Automated PR security analysis, IDOR route audit, and SBOM generation | ~1 min |
| **Unused Deps & Features** | [`udeps.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/udeps.yml) | 🔄 **Hybrid** | Weekly schedule (Tuesday), `dispatch` | Cargo-udeps compiler AST unused dependency & feature scanner | ~1.5 min |
| **Fuzz Corpus Minimization** | [`corpus-sync.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/corpus-sync.yml) | 🔄 **Hybrid** | Weekly schedule (Sunday), `dispatch` | Automated `cargo fuzz cmin` seed corpus synchronization | ~5 min |
| **GitHub Pages** | [`pages.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pages.yml) | 🚀 **Release / Deploy** | `push` (main) | Compiles and deploys documentation website to GitHub Pages | ~1.5 min |
| **Release & Publish** | [`release.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml) | 🚀 **Release / Deploy** | Tag push (`v*`), `dispatch` | Multi-arch binary builder, Sigstore attestation, and Crates.io publisher | ~4 min |

---

## 🚀 6. Next-Generation Verification Roadmap (The 6 Pillars of Excellence)

To ensure that Rullst remains at the forefront of software reliability, memory safety, and enterprise compliance, we maintain an active verification roadmap structured across **6 Pillars of World-Class Rust Verification**:

### 🏛️ Pillar 1: Concurrency & Data-Races (Tokio / Asynchronous Invariants)
- **`Loom` (Tokio Concurrency Permutation Engine):**
  - Exhaustively tests atomic ordering (`Acquire`/`Release`/`SeqCst`) and race conditions in lock-free buffers, sliding-window rate limiters, and `LoginGuard` tarpits.
- **`Shuttle` (Probabilistic Async Scheduling):**
  - Simulates thousands of asynchronous request interleavings in memory without spawning physical network listeners.

### 📦 Pillar 2: Supply Chain Governance & Cryptographic Provenance
- **`CycloneDX 1.5 JSON SBOM` (`cargo rullst audit --sbom`):**
  - Generates standardized Software Bill of Materials with SHA-256 package checksums and license compliance records for SOC 2, ISO 27001, and FedRAMP audits.
- **`cargo-vet` (Mozilla Supply Chain Attestation):**
  - Cryptographically verifies that every third-party crate in `Cargo.lock` has been audited and signed by trusted maintainers.
- **`SLSA Level 3` + `Sigstore Cosign`:**
  - Enforces cryptographic build provenance and binary signing on all release artifacts.

### 🔬 Pillar 3: Static Analysis & Compilation Invariants (SAST)
- **`cargo-careful`:**
  - Executes test suites against an instrumented standard library (`std`) to catch undefined pointer arithmetic and slice aliasing bugs with minimal overhead.
- **`assert_no_alloc` (Zero-Allocation Hot-Paths):**
  - Verifies at test time that router path matching, SRI hash verifications, and RASP rule evaluations trigger 0 heap allocations.
- **`cargo-semver-checks` & `cargo-llvm-cov`:**
  - Guarantees 100% backward API compatibility and line-by-line LLVM coverage.

### 🧬 Pillar 4: Mutation Testing & Test Suite Sensitivity
- **`cargo-mutants` (8 Parallel Shards):**
  - Injects AST mutations across all workspace crates to mathematically assert that test suites detect logic inversions and edge-case regressions.

### ⚡ Pillar 5: Binary Optimization & Zero-Cost Telemetry
- **`cargo-bloat` & `DHAT` Heap Profiling:**
  - Audits generic monomorphization costs and memory footprints for edge/IoT deployments (`rullst-iot`).
- **Profile-Guided Optimization (PGO + BOLT):**
  - Automates release binary recompilation based on real CPU execution telemetry, boosting request throughput by +15% to +25%.

### 🧪 Pillar 6: Chaos Engineering & Multi-Engine Fuzzing
- **`fail-rs` (Chaos & Fault Injection):**
  - Injects simulated network drops, socket timeouts, and database resets during transactions to validate circuit breaker recovery.
- **Multi-Engine Differential Fuzzing (`libFuzzer` + `AFL.rs` + `honggfuzz`):**
  - Continuous chaos testing with automated seed corpus harvesting across 33 dedicated fuzz targets.
- **100% Pure-Rustls Mandate:**
  - Strict zero-OpenSSL C-bindings policy across all network and cryptographic crates for total memory safety.

---
*Rullst - Built for those who want to build securely and easily, but not suffer.*
