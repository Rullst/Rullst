<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
</p>

<h1 align="center">🌐🦀📜 Rullst 📜🦀🌐</h1>
<h3 align="center"><i>Intelligent, Security-Conscious, and Designed for Effortless Productivity — Because With Rullst, We Rule!</i></h3>

<p align="center">An open-source, Axum-based framework suite.<br>Build beyond the endpoint—with code you can inspect and boundaries you control.</p>

<p align="center">
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/v/rullst?style=for-the-badge&color=10b981&logo=rust" alt="Crates.io"></a>
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/d/rullst?style=for-the-badge&color=blue" alt="Crates.io Downloads"></a>
  <a href="https://docs.rs/rullst"><img src="https://img.shields.io/docsrs/rullst?style=for-the-badge&logo=docsdotrs" alt="Docs.rs"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=for-the-badge&label=Main%20Build" alt="Main Rust CI"></a>
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT">
</p>

<p align="center">
  <a href="https://codecov.io/gh/Rullst/Rullst"><img src="https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg" alt="Whole-repository coverage"></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst"><img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard" alt="OpenSSF Scorecard"></a>
  <a href="https://rullst.github.io/Rullst/book/compatibility-policy.html"><img src="https://img.shields.io/badge/MSRV-1.96.0-f74c00?logo=rust" alt="MSRV 1.96.0"></a>
</p>

<p align="center">
  <a href="https://rullst.github.io/Rullst/book/start-here.html"><strong>Start building</strong></a> ·
  <a href="#live-examples"><strong>Try live examples</strong></a> ·
  <a href="https://rullst.github.io/Rullst/book/"><strong>Documentation</strong></a> ·
  <a href="https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md"><strong>Contribute</strong></a>
</p>

> **v12.0.0 is published.** Install the stable package below. `main` receives
> v12 maintenance; next-major work lives on `v13`. The legacy `v5` line is
> no longer maintained. [Release record](https://rullst.github.io/Rullst/book/v12.html)
> · [Compatibility policy](https://rullst.github.io/Rullst/book/compatibility-policy.html).

## 🚀 Start building

Generate a project, choose a blueprint and database, then start the development loop:

```bash
cargo install cargo-rullst --version 12.0.0 --locked
cargo rullst new my_app
cd my_app
cargo rullst dev
```

Choose **Blank / API, Blog, SaaS, LMS, Portfolio or ERP**. Generated projects
contain ordinary Rust you can inspect and change.

[Installation and prerequisites](https://rullst.github.io/Rullst/book/1-getting-started.html)
· [Zero-to-Hero tutorial](https://rullst.github.io/Rullst/book/tutorials/01-hello-world.html)
· [Build a JSON REST API](https://rullst.github.io/Rullst/book/tutorials/rest-api-quickstart.html)
· [CLI reference](https://rullst.github.io/Rullst/book/cli_reference.html)

<a id="live-examples"></a>

## 🧪 Try the live examples

Explore applications hosted on Azure Container Apps, with their code and
deployment recipes in [Rullst/examples](https://github.com/Rullst/examples):

| Application | Explore |
| :--- | :--- |
| 🌐 **Showcase** — blog, SSR and selected integration demonstrations | [Open Showcase ↗](https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/) |
| 🎓 **LMS** — course catalog and learning-platform example | [Open LMS ↗](https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/) |
| 💼 **Portfolio** — projects, skills and experience presentation | [Open Portfolio ↗](https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/) |

These are independently maintained demo snapshots and may lag the stable release.
Showcase payment fixtures are **not live checkout**. Deployment availability,
native downloads and provider approval are separate from framework test evidence;
use test data, not sensitive information, in public demos.

## ✨ What you can build on

Rullst coordinates application foundations in one versioned Rust workspace:

- **Product-shaped starting points:** six blueprints, inspectable generators,
  supervised rebuild/restart, a terminal dashboard and assisted upgrades.
- **Data and background work:** Active Record, transactions and an outbox for
  supported relational databases, plus explicitly scoped specialized stores
  and durable local messaging.
- **Identity and security:** sessions, Argon2id, passkey and OAuth2/OIDC helpers,
  ownership checks, CSRF, secure headers and bounded request defenses.
- **AI and service integrations:** local/cloud AI clients, tenant-aware RAG,
  transactional email and provider-specific payment/webhook adapters.
- **Developer visibility:** Studio for local runtime telemetry and Nexus for
  explicitly authorized registered-model administration.
- **A web-first foundation:** SSR with HTMX, JSON APIs and generated Omni/Tauri
  shells. Native toolchains, signing and device validation remain separate steps.

**Explicit boundaries are part of the design.** Database capabilities are not
interchangeable; security middleware does not replace application authorization;
live fiscal authorization and remote message-broker adapters remain roadmap work.
Choose the features you need and review their
[documented capabilities](https://rullst.github.io/Rullst/book/capability-ledger.html).

Rullst builds on **Axum, Tokio, Tower and SQLx**, with standard routers and pools
available at documented integration points. You can adopt it incrementally.
[Why Rullst?](https://rullst.github.io/Rullst/book/why-Rullst.html)
· [Axum & SQLx escape hatches](https://rullst.github.io/Rullst/book/axum-sqlx-migration.html)
· [Omni's web-first contract](https://rullst.github.io/Rullst/book/tutorials/43-omni-web-first.html)

<details>
<summary><strong>See the CLI and blueprint gallery</strong></summary>

<p>Recorded repository previews, not live release evidence. The current CLI can differ in menu options, labels and layout.</p>

<h2 align="center">CLI · From idea to inspectable Rust</h2>
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

</details>

<details>
<summary><strong>Open the development dashboard preview</strong></summary>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/cargo-rullst-dash.png" alt="Recorded Rullst terminal dashboard with project information, logs and controls" width="100%"/>
</p>

Repository screenshot, not live telemetry. Layout and available controls can
differ by version. [Development workflow](https://rullst.github.io/Rullst/book/tutorials/51-authenticated-hot-reload.html).

</details>

## 🔄 Upgrade with a preview

From an existing application's root:

```bash
cargo rullst upgrade --dry-run
cargo rullst upgrade
```

The CLI coordinates dependency updates, backs up controlled files and runs
compiler checks. Review the plan and application behavior; it does not migrate
production data or guarantee compatibility with an unreleased major version.

[Assisted upgrade tutorial](https://rullst.github.io/Rullst/book/tutorials/36-assisted-framework-upgrades.html)
· [v5 → v12 migration guide](https://rullst.github.io/Rullst/book/migration-v5-to-v12.html)

<a id="the-rullst-ecosystem"></a>

## 🏛️ Explore the ecosystem

Sixteen publishable crates share one release train. Select what your application
needs; detailed feature and provider boundaries live in the
[specification](https://rullst.github.io/Rullst/book/spec.html).

<details>
<summary><strong>Browse the crate directory</strong></summary>

| Crate | Focus |
| :--- | :--- |
| [rullst](https://github.com/Rullst/Rullst/tree/main/rullst) | Public framework facade and feature selection |
| [rullst-core](https://github.com/Rullst/Rullst/tree/main/rullst-core) | HTTP runtime, routing, lifecycle and telemetry |
| [rullst-orm](https://github.com/Rullst/Rullst/tree/main/rullst-orm) | Relational models, transactions and capability-specific persistence |
| [rullst-auth](https://github.com/Rullst/Rullst/tree/main/rullst-auth) | Passwords, sessions, passkeys and authorization helpers |
| [rullst-security](https://github.com/Rullst/Rullst/tree/main/rullst-security) | Defense-in-depth middleware, guards and audit helpers |
| [rullst-connect](https://github.com/Rullst/Rullst/tree/main/rullst-connect) | OAuth2/OIDC identity integrations |
| [rullst-ai](https://github.com/Rullst/Rullst/tree/main/rullst-ai) | Guarded local/cloud clients and tenant-aware retrieval |
| [rullst-capital](https://github.com/Rullst/Rullst/tree/main/rullst-capital) | Payment/payout adapters, webhooks and bounded billing helpers |
| [rullst-mail](https://github.com/Rullst/Rullst/tree/main/rullst-mail) | Transactional email and delivery controls |
| [rullst-messaging](https://github.com/Rullst/Rullst/tree/main/rullst-messaging) | Broker-neutral contracts and durable local messaging |
| [rullst-studio](https://github.com/Rullst/Rullst/tree/main/rullst-studio) | Local developer control room |
| [rullst-nexus](https://github.com/Rullst/Rullst/tree/main/rullst-nexus) | Registered-model admin with explicit access policy |
| [rullst-iot](https://github.com/Rullst/Rullst/tree/main/rullst-iot) | Bounded no_std helpers and signed OTA verification, not device integration |
| [rullst-macros](https://github.com/Rullst/Rullst/tree/main/rullst-macros) | Compile-time HTML and application macros |
| [rullst-orm-macros](https://github.com/Rullst/Rullst/tree/main/rullst-orm-macros) | Typed ORM code generation |
| [cargo-rullst](https://github.com/Rullst/Rullst/tree/main/cargo-rullst) | Project scaffolding, development and upgrade CLI |

</details>

## ⚡ Performance you can inspect

The [benchmark hub](https://rullst.github.io/Rullst/benches/) publishes eight
Criterion groups backed by nine benchmark binaries. They measure specific
workloads and regressions—not universal speed, application throughput or a
ranking of frameworks. Read the
[methodology](https://rullst.github.io/Rullst/book/tutorials/35-high-performance-benchmarking.html)
alongside the results.

## 🛡️ Verification, with visible scope

Explore the [stable release audit](https://rullst.github.io/Rullst/book/v12-release-audit.html),
[current capability status](https://rullst.github.io/Rullst/book/capability-status.html)
and [quality scorecard](https://rullst.github.io/Rullst/book/quality-scorecard.html).
Badges and test results are evidence for their stated scope, not a security
certification of every application built with the framework.

<details>
<summary><strong>🛡️ Open the full v12 verification dashboard (37 workflows)</strong></summary>

<h3 align="center">🛡️ v12 Main Verification Dashboard</h3>

<p align="center">
  Rullst applies layered compile, test, architecture, portability, and security checks.<br/>
  Badges are pinned to the <code>main</code> branch; they report the latest matching run, not a certification or deployment guarantee.
</p>

| Continuous or change-aware gate | v12 `main` status | Actual scope |
| :--- | :---: | :--- |
| **Rust CI** | [![Rust CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=flat-square&label=Rust%20CI)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain) | Format, all-target/all-feature Clippy, tests on Linux/macOS/Windows, Cargo-aware doctests sourced from all 52 public tutorials, strict DB boundaries, feature boundaries, generated-code checks, and MSRV 1.96.0. |
| **Declared MSRV** | [![MSRV 1.96.0](https://img.shields.io/badge/MSRV-1.96.0-f74c00?style=flat-square&logo=rust)](https://rullst.github.io/Rullst/book/compatibility-policy.html) | Every publishable v12 manifest declares Rust 1.96.0 and CI runs an explicit workspace all-feature check with that toolchain. |
| **GitHub Actions lint** | [![Workflow Lint](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/workflow-lint.yml?branch=main&style=flat-square&label=Workflow%20Lint)](https://github.com/Rullst/Rullst/actions/workflows/workflow-lint.yml?query=branch%3Amain) | Validates workflow syntax, expressions, embedded shell, and full-SHA third-party Action pins. |
| **Documentation** | [![Documentation](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/documentation.yml?branch=main&style=flat-square&label=Docs)](https://github.com/Rullst/Rullst/actions/workflows/documentation.yml?query=branch%3Amain) | Builds the mdBook and rejects broken local links and anchors; scheduled/manual runs also preserve an informational external-link report. |
| **End-to-end smoke** | [![E2E](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?branch=main&style=flat-square&label=E2E)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml?query=branch%3Amain) | Boots the release blog example and verifies HTTP, security headers, form flow, and SQLite persistence. |
| **Codecov — whole repository** | [![Whole-repository coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg)](https://codecov.io/gh/Rullst/Rullst) | The badge reports the current branch aggregate. The [stable-source LLVM run](https://github.com/Rullst/Rullst/actions/runs/34895523751) at `eb11f892` measured **90.3220%** (79,813/88,365 lines) before Codecov upload. The enforced repository floor is **≥90%** with zero tolerance. |
| **Codecov — framework libraries** | [![Framework library coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries)](https://codecov.io/gh/Rullst/Rullst) | Runtime libraries are enforced separately at **≥90%**. CLI and proc-macro components stay separately visible; [Coverage CI](https://github.com/Rullst/Rullst/actions/workflows/coverage.yml?query=branch%3Amain) uploads their real LCOV evidence with OIDC. |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?branch=main&style=flat-square&label=RustSec)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml?query=branch%3Amain) | RustSec advisory scan with only governed, expiring exceptions. |
| **Security exception governance** | [![Security Governance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/security-audit.yml?branch=main&style=flat-square&label=Exception%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/security-audit.yml?query=branch%3Amain) | Cross-checks scanner allowlists against the owner/expiry ledger, then independently reruns Cargo Audit. |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?branch=main&style=flat-square&label=Cargo%20Deny)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml?query=branch%3Amain) | Advisory, license, ban, and source policy. |
| **CodeQL SAST** | [![CodeQL](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?branch=main&style=flat-square&label=CodeQL)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml?query=branch%3Amain) | Rust semantic analysis after an all-target/all-feature build. |
| **OpenSSF Scorecard** | [![OpenSSF Scorecard](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square)](https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst) | The badge renders the score from the official public Scorecard JSON report; the pinned [Scorecard workflow](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) publishes OIDC-authenticated results on each `main` push and weekly. A score is evidence, not a security certification. |
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
| **PR security evidence** | [![PR-only evidence](https://img.shields.io/badge/trigger-PR--only-2563eb?style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/ai-sentinel-pr.yml?query=event%3Apull_request) | Pull-request-only bounded IDOR/RBAC heuristics and CycloneDX SBOM artifact. It intentionally has no continuous `main` status. |

Deep or irreversible workflows are intentionally not presented as continuously
green main gates:

| Deep evidence | Trigger and enforcement |
| :--- | :--- |
| [Benchmark regression](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Weekly, `main` push, or manual; eight published groups backed by nine Criterion benchmark binaries emit non-blocking alerts at a 20% regression. |
| [Property testing](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Weekly/manual release-mode invariant testing with 10,000 configured cases. |
| [TSan and ASan](https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml) | Daily/manual package matrices on a pinned verifier-only nightly. |
| [Fuzzing](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) / [corpus minimization](https://github.com/Rullst/Rullst/actions/workflows/corpus-sync.yml) | Forty manual libFuzzer jobs; weekly/manual corpus maintenance is informational. |
| [OWASP ZAP](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Manual baseline over three release surfaces: generated REST API and complete LMS are blocking with no ignored alerts; the deliberately CDN-backed blog showcase remains an explicitly informational boundary. |
| [Kani](https://github.com/Rullst/Rullst/actions/workflows/kani.yml), [Miri](https://github.com/Rullst/Rullst/actions/workflows/miri.yml), [mutation testing](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml), [cargo-udeps](https://github.com/Rullst/Rullst/actions/workflows/udeps.yml) | Manual or scheduled research signals: selected Kani/Miri scopes are strict, while mutation and unused-dependency findings remain explicitly informational. |
| [GitHub Pages](https://github.com/Rullst/Rullst/actions/workflows/pages.yml) | Deploys the v12 documentation from `main`; it is not a code-quality gate. |
| [Release and provenance](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Exact version tags only: full verification, package-all, evidence bundle, checksums, GitHub build-provenance attestation, changelog-derived release notes, and ordered crates.io publication. This does **not** claim a project-wide SLSA level or independent certification. |

Scheduled events use the repository's default branch, so scheduled and
continuous v12 evidence now refer to `main`. The recommended required-check
profile and the exact scope of all
37 workflow definitions are documented in [WORKFLOWS.md](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md).

> 📖 **[Read the detailed breakdown of all CI/CD and security workflows](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md).**
>
> 🧭 **[Capability Status & Vision Decisions](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-ledger.md)** preserves ambitious features that are partial or not implemented, with an explicit recommendation and rationale for each one.
>
> 📋 **[Simple Capability Status](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-status.md)** and the **[per-commit quality scorecard](https://github.com/Rullst/Rullst/blob/main/docs/src/quality-scorecard.md)** keep feature progress separate from SHA-bound engineering evidence.

</details>

## 🤝 Build Rullst with us

Try a blueprint, report a reproducible bug, improve a tutorial or contribute a
focused change with tests. Documentation, accessibility and integration feedback
matter as much as new features.

[Contributing](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md)
· [Issues](https://github.com/Rullst/Rullst/issues)
· [Discord](https://discord.com/invite/2ntKFtsSjw)
· [Community links](https://rullst.github.io/Rullst/#community)
· [Our story and philosophy](https://rullst.github.io/Rullst/book/philosophy.html)

**What's next?** v12 receives compatible maintenance. The
[v13 roadmap](https://github.com/Rullst/Rullst/blob/v13/ROADMAP.md) guides
next-major development; planned capabilities are not shipped features.

[MIT license](https://github.com/Rullst/Rullst/blob/main/LICENSE)
· [Report a vulnerability privately](https://github.com/Rullst/Rullst/security/policy)
· [Website privacy notice](https://rullst.github.io/Rullst/#privacy)

<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>
