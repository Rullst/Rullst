<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>

> [!WARNING]
> **Rullst v12 development snapshot:** `main` contains active, unreleased work
> and remains **NO-GO** for a stable `12.0.0` release until the documented tests,
> audits, package checks, and release-candidate gates pass on the exact commit.
> Use a versioned
> [crates.io](https://crates.io/crates/rullst) release or its matching immutable
> tag in production; do not deploy from the moving `main` branch. The frozen
> `v5` branch preserves legacy source without ongoing maintenance. See the
> [v12 release program](docs/src/v12.md) and
> [compatibility policy](docs/src/compatibility-policy.md).

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
</p>

<h1 align="center">🌐🦀📜 Rullst 📜🦀🌐</h1>
<h3 align="center"><i>Explicit, Productive, Security-Conscious Rust Web Development — Because With Rullst We Rule!</i></h3>

<p align="center">
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/v/rullst?style=for-the-badge&color=10b981&logo=rust" alt="Crates.io"></a>
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/d/rullst?style=for-the-badge&color=blue" alt="Crates.io Downloads"></a>
  <a href="https://docs.rs/rullst"><img src="https://img.shields.io/docsrs/rullst?style=for-the-badge&logo=docsdotrs" alt="Docs.rs"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=for-the-badge&label=Main%20Build" alt="Main Rust CI"></a>
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT">
</p>

<h3 align="center">🛡️ v12 Main Verification Dashboard</h3>

<p align="center">
  Rullst applies layered compile, test, architecture, portability, and security checks.<br/>
  Badges are pinned to the <code>main</code> branch; they report the latest matching run, not a certification or deployment guarantee.
</p>

| Continuous or change-aware gate | v12 `main` status | Actual scope |
| :--- | :---: | :--- |
| **Rust CI** | [![Rust CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=flat-square&label=Rust%20CI)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain) | Format, all-target/all-feature Clippy, tests on Linux/macOS/Windows, strict DB boundaries, feature boundaries, generated-code checks, and MSRV 1.96.0. |
| **GitHub Actions lint** | [![Workflow Lint](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/workflow-lint.yml?branch=main&style=flat-square&label=Workflow%20Lint)](https://github.com/Rullst/Rullst/actions/workflows/workflow-lint.yml?query=branch%3Amain) | Validates workflow syntax, expressions, embedded shell, and full-SHA third-party Action pins. |
| **End-to-end smoke** | [![E2E](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?branch=main&style=flat-square&label=E2E)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml?query=branch%3Amain) | Boots the release blog example and verifies HTTP, security headers, form flow, and SQLite persistence. |
| **LLVM coverage** | [![Coverage](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/coverage.yml?branch=main&style=flat-square&label=Coverage)](https://github.com/Rullst/Rullst/actions/workflows/coverage.yml?query=branch%3Amain) | Generates LCOV from workspace and DB-matrix tests; the OIDC-authenticated Codecov upload is blocking. |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?branch=main&style=flat-square&label=RustSec)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml?query=branch%3Amain) | RustSec advisory scan with only governed, expiring exceptions. |
| **Security exception governance** | [![Security Governance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/security-audit.yml?branch=main&style=flat-square&label=Exception%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/security-audit.yml?query=branch%3Amain) | Cross-checks scanner allowlists against the owner/expiry ledger, then independently reruns Cargo Audit. |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?branch=main&style=flat-square&label=Cargo%20Deny)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml?query=branch%3Amain) | Advisory, license, ban, and source policy. |
| **CodeQL SAST** | [![CodeQL](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?branch=main&style=flat-square&label=CodeQL)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml?query=branch%3Amain) | Rust semantic analysis after an all-target/all-feature build. |
| **Cargo Machete** | [![Machete](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?branch=main&style=flat-square&label=Machete)](https://github.com/Rullst/Rullst/actions/workflows/machete.yml?query=branch%3Amain) | Unused direct dependency detection. |
| **SemVer checks** | [![SemVer](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?branch=main&style=flat-square&label=SemVer)](https://github.com/Rullst/Rullst/actions/workflows/semver.yml?query=branch%3Amain) | Supported library APIs are compared with exact latest non-yanked registry baselines; never-published packages and unsupported proc-macro/binary surfaces are reported explicitly. |
| **Zero-panics policy** | [![Zero Panics](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?branch=main&style=flat-square&label=Zero%20Panics)](https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml?query=branch%3Amain) | Denies panic-family operations in published production targets and generated runtime templates. |
| **Unsafe boundary** | [![Unsafe Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?branch=main&style=flat-square&label=Unsafe%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml?query=branch%3Amain) | Denies new production unsafe code outside the reviewed OS/FFI allowlist. |
| **Secret scanning** | [![TruffleHog](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/trufflehog.yml?branch=main&style=flat-square&label=Secrets)](https://github.com/Rullst/Rullst/actions/workflows/trufflehog.yml?query=branch%3Amain) | Verified-secret scan across the configured Git history range. |
| **Spellcheck** | [![Spellcheck](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/spellcheck.yml?branch=main&style=flat-square&label=Spellcheck)](https://github.com/Rullst/Rullst/actions/workflows/spellcheck.yml?query=branch%3Amain) | Repository-wide typo detection. |
| **Crate architecture policy** | [![Architecture](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/architecture.yml?branch=main&style=flat-square&label=Architecture)](https://github.com/Rullst/Rullst/actions/workflows/architecture.yml?query=branch%3Amain) | Compares the real publishable-crate dependency graph with a versioned, reviewed repository policy. |
| **WebAssembly matrix** | [![Wasm](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/wasm-matrix.yml?branch=main&style=flat-square&label=Wasm)](https://github.com/Rullst/Rullst/actions/workflows/wasm-matrix.yml?query=branch%3Amain) | Compiles Core, the public facade and macros for browser Wasm and WASI Preview 1. |
| **Bare-metal `no_std` matrix** | [![no_std](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/no_std-build.yml?branch=main&style=flat-square&label=no_std)](https://github.com/Rullst/Rullst/actions/workflows/no_std-build.yml?query=branch%3Amain) | Builds IoT helpers for Cortex-M and RISC-V targets; this is compile evidence, not hardware testing. |
| **IoT integration** | [![IoT](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/iot-integration.yml?branch=main&style=flat-square&label=IoT)](https://github.com/Rullst/Rullst/actions/workflows/iot-integration.yml?query=branch%3Amain) | Host tests, signed OTA invariants, and a Cortex-M build. |
| **IoT crypto containment** | [![IoT Crypto](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/pqc-compliance.yml?branch=main&style=flat-square&label=Crypto%20Boundary)](https://github.com/Rullst/Rullst/actions/workflows/pqc-compliance.yml?query=branch%3Amain) | Path-aware signed OTA, Vault, advisory, and simulator-boundary checks; no PQC/HSM certification claim. |
| **Omni desktop matrix** | [![Omni Desktop](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/omni-desktop.yml?branch=main&style=flat-square&label=Omni%20Desktop)](https://github.com/Rullst/Rullst/actions/workflows/omni-desktop.yml?query=branch%3Amain) | Generates fresh web shells and checks their Tauri crates on Linux, macOS and Windows; no installer, signing or store claim. |
| **Omni Android compile** | [![Omni Android](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/omni-android.yml?branch=main&style=flat-square&label=Omni%20Android)](https://github.com/Rullst/Rullst/actions/workflows/omni-android.yml?query=branch%3Amain) | Generates a fresh shell and compiles an unsigned Android debug APK; no physical-device, Play testing/signing or store claim. |
| **Omni iOS simulator** | [![Omni iOS](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/omni-ios.yml?branch=main&style=flat-square&label=Omni%20iOS)](https://github.com/Rullst/Rullst/actions/workflows/omni-ios.yml?query=branch%3Amain) | Path-aware fresh scaffold generation and compilation on a macOS iOS simulator target; no device, signing or App Store claim. |
| **PR security evidence** | [![PR Security](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ai-sentinel-pr.yml?branch=main&style=flat-square&label=PR%20Evidence)](https://github.com/Rullst/Rullst/actions/workflows/ai-sentinel-pr.yml?query=branch%3Amain) | Pull-request-only bounded IDOR/RBAC heuristics and CycloneDX SBOM artifact. |

Deep or irreversible workflows are intentionally not presented as continuously
green main gates:

| Deep evidence | Trigger and enforcement |
| :--- | :--- |
| [OpenSSF Scorecard](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) | Weekly/default-branch supply-chain evidence and SARIF; [scorecard badge](https://securityscorecards.dev/viewer/?uri=github.com/Rullst/Rullst). |
| [Benchmark regression](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Weekly, `main` push, or manual; eight suites emit non-blocking alerts at a 20% regression. |
| [Property testing](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Weekly/manual release-mode invariant testing with 10,000 configured cases. |
| [TSan and ASan](https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml) | Daily/manual nightly-toolchain package matrices. |
| [Fuzzing](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) / [corpus minimization](https://github.com/Rullst/Rullst/actions/workflows/corpus-sync.yml) | Forty manual libFuzzer jobs; weekly/manual corpus maintenance is informational. |
| [OWASP ZAP](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Manual DAST against the blog example only. |
| [Kani](https://github.com/Rullst/Rullst/actions/workflows/kani.yml), [Miri](https://github.com/Rullst/Rullst/actions/workflows/miri.yml), [mutation testing](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml), [cargo-udeps](https://github.com/Rullst/Rullst/actions/workflows/udeps.yml) | Manual or scheduled research signals with explicitly non-blocking portions. |
| [GitHub Pages](https://github.com/Rullst/Rullst/actions/workflows/pages.yml) | Deploys the unreleased v12 documentation preview from `main`; it is not a code-quality gate. |
| [Release and provenance](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Exact version tags only: full verification, package-all, evidence bundle, checksums, attestations, ordered crates.io publish, and release provenance. |

Scheduled events use the repository's default branch, so scheduled and
continuous v12 evidence now refer to `main`. The recommended required-check
profile and the exact scope of all
36 workflow definitions are documented in [WORKFLOWS.md](WORKFLOWS.md).

> 📖 **[Read the detailed breakdown of all CI/CD and security workflows](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md).**
>
> 🧭 **[Capability Status & Vision Decisions](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-ledger.md)** preserves ambitious features that are partial or not implemented, with an explicit recommendation and rationale for each one.
>
> 📋 **[Simple Capability Status](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-status.md)** and the **[per-commit quality scorecard](https://github.com/Rullst/Rullst/blob/main/docs/src/quality-scorecard.md)** keep feature progress separate from SHA-bound engineering evidence.
<br>

## 💡 The Rullst Philosophy

Rullst coordinates routing, auth, ORM, bounded background jobs, and developer
tooling behind explicit APIs. Compile-time generation reduces setup while
keeping deployed security and operational decisions visible for review by
humans and coding agents. Read the complete [design philosophy](docs/src/philosophy.md).

---

### ⚡ Quick Start: From Zero to Hero

New to Rust or Rullst? The complete walkthrough covers Rust installation, a
v12 preview dependency, the first typed route, error handling, and running the
server on Linux, macOS, and Windows:

> 📖 **[Build your first Rullst application with the Zero-to-Hero tutorial](docs/src/tutorials/01-hello-world.md)**

Already have Rust? Start the unreleased v12 preview and continue with the
documented `src/main.rs`:

```bash
cargo new my_app
cd my_app
cargo add rullst --git https://github.com/Rullst/Rullst.git --branch main
cargo add tokio --features full
```

The Git dependency tracks active development and is for evaluation only. Keep
the generated `Cargo.lock`, do not deploy from mutable `main`, and use a versioned crate
release or immutable tag in production.

<h2 align="center"> CLI ⚡ Rullst Framework ⚡ </h2>
<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/gifs/gif.gif" alt="Rullst CLI Initiating LMS Blueprint" width="80%"/>
</p>

<h2 align="center">Click to Watch: How to build a SaaS Blueprint with Rullst </h2>
<p align="center">
<a href="https://www.youtube.com/watch?v=nDXLeNM327g">
  <img src="https://img.youtube.com/vi/nDXLeNM327g/hqdefault.jpg" alt="How to build a SaaS with Rullst" width="60%" />
</a>
</p>

<table align="center" width="100%">
  <tr>
    <th align="center" width="50%"><h2>SaaS Blueprint</h2></th>
    <th align="center" width="50%"><h2>LMS Blueprint</h2></th>
  </tr>
  <tr>
    <td align="center">
      <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/gifs/gif1.gif" alt="SaaS Blueprint" width="100%" />
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/gifs/gif2.gif" alt="LMS Blueprint" width="100%" />
    </td>
  </tr>
</table>

---
<br>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/cargo-rullst-dash.png" alt="Rullst Interactive Terminal Dashboard" width="100%"/>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/Rullst-Omni.png" alt="Rullst Omni Mobile & Desktop Simulator" width="100%"/>
</p>

Rullst Omni follows a **web-first, platform-enhanced** architecture: the Rullst
web application and server remain canonical, while a generated Tauri shell
packages the same experience for desktop, Android and iOS behind an exact-origin
navigation boundary and no privileged remote IPC. The portable
`rullst.client` v1 contract gives web and platform code the same bounded typed
JSON envelopes without trusting client-supplied roles or ownership. The opt-in
native `offline-sync` foundation adds account-bound encrypted snapshots,
idempotent FIFO proposals, explicit conflict/resync/recovery state and a bounded
push/pull coordinator over application-owned authenticated transports; platform
secure-key/storage, concrete HTTP/background integration and device evidence
remain application work, so the shell itself is not advertised as offline-first. See the
[Omni tutorial](docs/src/tutorials/43-omni-web-first.md) and
[offline-sync tutorial](docs/src/tutorials/44-omni-offline-sync.md).

### 🔄 Assisted framework upgrades (v12 preview)

The v12 CLI can plan and apply a backed-up framework upgrade from the
application root:

```bash
cargo rullst upgrade --dry-run
cargo rullst upgrade
```

It updates the coordinated Rullst dependency train, scans known source risks,
applies compiler fixes, runs a Cargo check gate, and restores controlled files
after a failure. It does not migrate production data or invent application
security policy. See the
[assisted upgrade tutorial](docs/src/tutorials/36-assisted-framework-upgrades.md)
and the [v5 → v12 guide](docs/src/migration-v5-to-v12.md).

### 📚 Documentation & Community

The documentation separates released behavior, v12 preview capabilities,
migration guidance, and roadmap boundaries. Explore the guides and evaluate the
features your application intends to enable:

👉 **[Explore the Official Website & Docs](https://rullst.github.io)**

💬 **[Join the Community on Discord](https://discord.gg/2ntKFtsSjw)**

> **Found a bug?** [Report an Issue](https://github.com/Rullst/Rullst/issues)

---

### ⚡ Performance: evidence over slogans

Rullst does not claim to be universally faster than every alternative. Runtime
results depend on enabled features, handlers, databases, deployment settings,
hardware, and workload. The implementation choices that can be inspected today
are:

- **`html!` rendering:** expands into escaped Rust `String` construction without
  a runtime Virtual DOM; dynamic values are escaped by the generated code.
- **`routes!` registration:** expands at compile time into ordinary, typed Axum
  route registrations. Request matching and middleware still execute at runtime.
- **Bounded local primitives:** Ammonia allowlist sanitization, RBAC ownership
  checks, AI heuristics, and the tenant-partitioned in-memory RAG retriever have explicit limits;
  they are not described as zero-cost.
- **Development build tuning:** generated configuration may select an installed
  `mold` or `lld`, and debug hot reload uses a watched dynamic library plus a
  WebSocket browser refresh path. Results vary by host and project.

Eight [Criterion benchmark suites](https://github.com/Rullst/Rullst/actions/workflows/bench.yml)
track selected microbenchmarks. Their shared-runner history is regression
evidence for those inputs, not a production throughput promise. The ORM suite
also runs five equivalent typed-SQLite operations through pinned Rullst,
Diesel and SeaORM dependencies; it preserves unfavorable results and does not
claim universal superiority. The separate
[cross-framework harness](https://github.com/Rullst/Benchmarks) currently
measures a historical Rullst 4.x application and must not be used as v12
performance evidence until its versions and runs are refreshed.

### v12 capability highlights

- 🎨 **Studio & Nexus:** a loopback-first developer dashboard at `:5555` and an
  authenticated `/nexus` model CRUD panel generated from explicit model metadata.
- 🛡️ **Security helpers:** bounded request heuristics for selected XSS, SQLi,
  traversal, and command-injection indicators plus explicit role/ownership
  guards. These controls do not replace application authorization or testing.
- 🔑 **Passkeys/WebAuthn:** one-time challenge handling and ES256 registration
  and assertion verification within the documented `none`-attestation scope;
  deployments own RP/origin policy and atomic counter persistence.
- 🌐 **Guarded AI client:** supported Gemini, Claude, OpenAI, DeepSeek, and
  Ollama transports with bounded injection heuristics and PII masking on the
  high-level client path. Passing a heuristic is not authorization.
- 💳 **Capital adapters:** Stripe and LemonSqueezy checkout/webhook adapters use
  cryptographic HMAC verification; Axum and opt-in Actix middleware share one
  bounded verifier. The included process-local metrics/event buffer is a preview
  helper, not an accounting ledger or authoritative MRR.
- 🔐 **Encryption & memory hygiene:** `#[orm(encrypted)]` transparently protects
  string fields on generated ORM writes/reads using versioned AES-256-GCM,
  authenticated table/column context, key identifiers, and keyrings. Randomized
  fields are intentionally not queryable without a separate blind index;
  key custody and OS/allocator memory exposure remain external.
- 🔄 **Transactions:** `Orm::transaction` scopes generated queries through
  `CURRENT_TX`, commits on success, and rolls back on failure; its current
  closure API returns a boxed future.
- 📬 **Durable outbox:** relational domain changes can enqueue an explicit,
  idempotent event in the same transaction. Lease/token claims, bounded retry
  and dead-letter are covered on SQLite, PostgreSQL, MySQL and MariaDB;
  delivery is at least once and consumers remain responsible for idempotency.
- 🔎 **Scout search:** an opt-in feature provides bounded Meilisearch,
  Elasticsearch and Algolia projections with deterministic offline fallbacks;
  only Meilisearch currently has a live pinned service contract in CI.
- 🧠 **Typed pgvector search:** `orm-pgvector` plus `strict-postgres` provides
  SQLx-compatible vectors and parameterized L2/cosine/inner-product queries,
  exercised against a digest-pinned PostgreSQL + pgvector service. The AI crate
  separately composes bounded tenant-aware retrieval, guarded context/generation,
  sources and audit; datastore authorization and index tuning remain application work.
- 🧭 **Specialized vector and key-value stores:** `orm-qdrant` provides bounded
  dense-cosine collection/upsert/delete/query operations, while `orm-redis`
  provides namespaced Hash, Set and Sorted Set operations. Deterministic
  fallbacks and digest-pinned live lifecycles cover both; hosted availability,
  cluster/failover and application authorization remain explicit.
- 🔄 **Database introspection:** `cargo rullst make:models-from-db` generates
  starter model files through parameterized SQLite/PostgreSQL/MySQL metadata
  queries and fail-closed identifier validation. Table module names are
  normalized, while columns that would require unsupported ORM remapping are
  rejected before files are written; bounded type mappings, keys, relations,
  schemas, and generated code still require review.
- 🔍 **Static project inspection:** `cargo rullst inspect` scans conventional
  `routes!` entries and model declarations or prints the generated JSON schema;
  it is not a runtime route inventory.
- 🛡️ **Zero-panic policy:** published production targets are gated against
  `panic!`, `unwrap`, and `expect`. This is an engineering policy, not a promise
  that applications or dependencies can never terminate unexpectedly.
- ⚡ **Scaffolding and deployment helpers:** generators cover application
  starting points and deployment manifests; provider CLIs, generated output,
  infrastructure, and production rollout remain operator-reviewed boundaries.

---

### 🌐 Frontend modes

Rullst scaffolds several presentation strategies. They are alternatives, not a
claim that every mode has identical maturity, bundle size, or migration cost.

| Mode | Runtime model | Important boundary |
| :--- | :--- | :--- |
| HTMX SSR | Server-rendered HTML with HTMX interactions | HTMX is a browser dependency even though no project-local SPA bundle is required. |
| LiveView | Server state synchronized over WebSockets | Requires connection lifecycle, origin checks, backpressure, and reconnect testing. |
| Wasm Islands | Client-side WebAssembly for selected components | Bundle size and browser compatibility must be measured per application. |
| Pico.css | Semantic server-rendered HTML with an external stylesheet | Styling is external; application behavior remains server-oriented. |
| Tera | File-based server templates | Template context and escaping boundaries require review like any rendering system. |

---

### 🔓 Standard Axum & SQLx Escape Hatches

Rullst is built on **Axum**, **Tokio**, **Tower**, and **SQLx**, and exposes standard routers and pools at important integration points. Some framework helpers and generated structures still require an explicit migration when ejecting:

- **Incremental Adoption:** Mount existing `axum::Router` instances directly into `rullst::server::Server`.
- **Standard SQLx:** Run raw `sqlx::Pool` queries alongside `rullst-orm` without wrappers.
- **Escape Hatch:** Use the CLI eject output as a migration starting point and review the generated code before deployment.
- 📖 Read the full [Axum & SQLx Migration & Escape Hatch Guide](https://github.com/Rullst/Rullst/blob/main/docs/src/axum-sqlx-migration.md).

---

## Choosing Rullst

Rullst is a good fit when an Axum-based application benefits from a coordinated
CLI, ORM, authentication helpers, bounded background workers, provider adapters,
and local developer tooling in one versioned workspace. A smaller HTTP library
may be preferable when the application needs only routing and middleware; a
frontend-first framework may be preferable when client component composition is
the primary architecture.

For a concise, evidence-bounded explanation of the framework's strongest
technical differentiators, read [Why Rullst?](docs/src/why-Rullst.md).

### Where it fits in the Rust ecosystem

This is a positioning guide, not a feature-score leaderboard. The projects solve
different problems and can sometimes be used together.

| Center of gravity | Consider | Why |
| :--- | :--- | :--- |
| A coordinated, backend-oriented application stack | **Rullst v12** | Axum-based routing plus versioned ORM, auth, security helpers, workers, provider adapters, Studio/Nexus, and CLI workflows. v12 is still an unreleased preview. |
| A modular HTTP service assembled from selected libraries | [**Axum**](https://docs.rs/axum/latest/axum/) or [**Actix Web**](https://actix.rs/docs/) | Focused HTTP foundations with their own middleware ecosystems and freedom to choose the rest of the stack. |
| A Rails-inspired, batteries-included Axum application | [**Loco**](https://loco.rs/docs/) | A mature adjacent choice with models, controllers, jobs, mailers, auth, generators, and documented upgrades. |
| A reactive, isomorphic web UI | [**Leptos**](https://book.leptos.dev/) | Fine-grained reactive components spanning browser rendering, SSR, hydration, and server functions. |
| A shared Rust UI across web, desktop, and mobile | [**Dioxus**](https://dioxuslabs.com/learn/0.7/) | Component-centered cross-platform applications with optional full-stack Axum integration. |

Rullst is not presented as universally better. Its distinctive bet is that a
single, explicit CLI and release train can coordinate a broad backend stack
without hiding the underlying Axum, Tokio, Tower, and SQLx integration points.

Evaluate only the features you intend to enable. In particular:

- security middleware is defense-in-depth and does not replace authorization,
  validation, proxy configuration, or penetration testing;
- generated OpenAPI, TypeScript, deployment, and ejection output requires review
  and compilation in the target application;
- Studio is a local developer tool and Nexus requires an explicit authenticated
  access policy;
- live NFS-e, Alipay RSA2, MQTT/HSM/PQC, S3/R2, and Connect message brokers are
  not stable capabilities in version 12.
- persistence support is capability-specific: SQLx Active Record targets
  SQLite/PostgreSQL/MySQL/MariaDB; the bounded blank/API profile can use
  Turso/libSQL as a typed primary; MongoDB, DuckDB, SurrealDB, Qdrant and Redis
  retain explicit capability APIs rather than a fictional universal ORM.

---

### 🏛️ The Rullst Monorepo (v12.0.0+)

Rullst is a unified monorepo. Core, ORM, Connect, and the domain crates are versioned and tested together so compatibility regressions can be caught in one workspace; consumers should still follow SemVer notes and the release matrix for the exact version they use.

**Explore the Monorepo Ecosystem:**
- 🦀 **[rullst-core](https://github.com/Rullst/Rullst/tree/main/rullst-core)**: Runtime-only-by-default HTTP server, routing engine, and telemetry kernel; ORM and SQLite queues are explicit features.
- 💾 **[rullst-orm](https://github.com/Rullst/Rullst/tree/main/rullst-orm)**: Active Record, durable opt-in outbox, Scout, pgvector, bounded Qdrant and namespaced Redis structures for SQLite/PostgreSQL/MySQL/MariaDB, a bounded Turso-primary blank/API profile, and capability APIs for MongoDB, DuckDB and SurrealDB; applications still own tenant predicates and database policy. See [Polyglot Persistence](docs/src/polyglot-persistence.md), [Transactional Outbox](docs/src/tutorials/38-transactional-outbox.md), [Scout Search](docs/src/tutorials/39-scout-search.md) and [RAG/Vector Search](docs/src/tutorials/22-rag-vector-search.md).
- 🛡️ **[rullst-auth](https://github.com/Rullst/Rullst/tree/main/rullst-auth)**: Passkeys/WebAuthn, Argon2id, encrypted cookie sessions, opt-in application JWT policy, and RBAC authorization.
- 🔒 **[rullst-security](https://github.com/Rullst/Rullst/tree/main/rullst-security)**: Bounded RASP request heuristics, honeypot traps, HTML/CSP helpers, and an HMAC-chained audit log.
- 🤖 **[rullst-ai](https://github.com/Rullst/Rullst/tree/main/rullst-ai)**: Guarded provider-agnostic clients (Gemini, OpenAI, Claude, DeepSeek, Ollama), bounded tenant-aware audited RAG, structured output, and authorized local-tool foundations. See [Tenant-Bound RAG](docs/src/tutorials/41-tenant-bound-rag.md).
- 💰 **[rullst-capital](https://github.com/Rullst/Rullst/tree/main/rullst-capital)**: SaaS MRR/ARR analytics, payment-provider adapters, bounded provider-specific coupons/trial extensions, shared idempotent Team/Workspace quotas with opt-in four-dialect SQL accounting, and bounded checksum-pinned NFS-e DPS/XSD/XMLDSig/mTLS preparation; live Alipay RSA2 and official NFS-e authorization remain fail-closed roadmap/external-evidence work.
- 🔌 **[rullst-connect](https://github.com/Rullst/Rullst/tree/main/rullst-connect)**: OAuth2/OIDC social login with strict discovery, rotating JWKS caches, bounded process-local automatic token refresh, offline provider fallbacks, an explicitly mounted signed loopback IdP fixture, and an opt-in one-active-challenge tower-sessions state/PKCE/nonce transaction. Queue transports currently live in Core.
- 📨 **[rullst-messaging](https://github.com/Rullst/Rullst/tree/main/rullst-messaging)**: Versioned bounded envelopes, topic-scoped idempotent publication, consumer-group fan-out, expiring ACK leases, retry/DLQ, and a deterministic process-local broker. Remote Kafka/RabbitMQ/Redis Streams/NATS/cloud adapters remain roadmap work. See [Bounded Brokered Messaging](docs/src/tutorials/49-brokered-messaging.md).
- 📡 **[rullst-iot](https://github.com/Rullst/Rullst/tree/main/rullst-iot)**: `no_std` telemetry/frame helpers and Ed25519-signed OTA manifest verification; MQTT transport, HSM, and PQC remain roadmap work.
- ✉️ **[rullst-mail](https://github.com/Rullst/Rullst/tree/main/rullst-mail)**: Transactional email drivers with message validation, bounded secret-pattern checks, background delivery, and safe scaffolds including evidence-aware fiscal receipts and explicit dunning stages.
- 📊 **[rullst-studio](https://github.com/Rullst/Rullst/tree/main/rullst-studio)**: Developer Control Room (`:5555`) with live telemetry and data browser.
- ⚙️ **[rullst-nexus](https://github.com/Rullst/Rullst/tree/main/rullst-nexus)**: Registered-model Admin CMS (`/nexus`) and a local security-event view.
- 🛠️ **[cargo-rullst](https://github.com/Rullst/Rullst/tree/main/cargo-rullst)**: CLI scaffolding, bounded AST IDOR checks, and deployment helpers.

---

**Rullst** is an opinionated, Axum-based full-stack framework for teams that
want coordinated Rust tooling without hiding application security boundaries.

---

<br>

<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>
