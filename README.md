<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/hero-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/hero-light.svg">
    <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/hero-dark-static.webp" alt="Rullst — build secure apps in Rust without the suffering. Batteries included, secure by default, designed for humans and AI." width="100%">
  </picture>
</p>

<h1 align="center">🌐🦀📜 Rullst 📜🦀🌐</h1>
<h3 align="center"><i>Intelligent, Security-Conscious, and Designed for Effortless Productivity — Because With Rullst, We Rule!</i></h3>

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
  <a href="#quickstart"><strong>Quickstart</strong></a> ·
  <a href="https://rullst.github.io/Rullst/book/start-here.html"><strong>Start building</strong></a> ·
  <a href="https://academy.rullst.win/"><strong>Rullst Academy</strong></a> ·
  <a href="#live-examples"><strong>Live examples</strong></a> ·
  <a href="https://rullst.github.io/Rullst/book/"><strong>Documentation</strong></a> ·
  <a href="https://discord.com/invite/2ntKFtsSjw"><strong>Discord</strong></a>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/terminal.svg" alt="Terminal: cargo rullst new asks for the app name, blueprint and database, creates a SaaS starter with SQLite, then cargo rullst dev compiles it and serves it at http://localhost:3000." width="92%">
</p>

<a id="quickstart"></a>

## ⚡ Rullst in 30 seconds

```bash
cargo install cargo-rullst --version '^12' --locked
cargo rullst new my_app    # choose Blank/API, Blog, SaaS, LMS, Portfolio or ERP
cd my_app
cargo rullst dev           # rebuilds and restarts every time you save
```

That's it: a running application, generated as readable Rust you can change —
no hidden runtime magic. The selector installs the latest stable v12 CLI; use a
full version such as `--version 12.1.2` to reproduce a specific release.

[Installation and prerequisites](https://rullst.github.io/Rullst/book/1-getting-started.html)
· [Zero-to-Hero tutorial](https://rullst.github.io/Rullst/book/tutorials/01-hello-world.html)
· [Build a JSON REST API](https://rullst.github.io/Rullst/book/tutorials/rest-api-quickstart.html)
· [CLI reference](https://rullst.github.io/Rullst/book/cli_reference.html)

<details>
<summary><strong>Prefer an interactive dashboard? Run <code>cargo rullst dash</code></strong></summary>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/cargo-rullst-dash.png" alt="Rullst terminal dashboard with project information, logs and controls" width="100%"/>
</p>

Recorded screenshot; layout and controls can differ by version. [Development workflow](https://rullst.github.io/Rullst/book/tutorials/51-authenticated-hot-reload.html).

</details>

## 🦀 Code that says what it does

```rust
use rullst::{html, response::Html, routes, Server};

async fn home() -> Html<String> {
    Html(html! {
        <div class="min-h-screen bg-slate-900 text-emerald-400 flex flex-col items-center justify-center font-sans">
            <h1 class="text-5xl font-extrabold mb-4">"Hello, Rullst! 📜🦀"</h1>
            <p class="text-slate-400 text-lg">"Your first typed route is running."</p>
        </div>
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![
        get("/" => home)
    ];

    Server::new(app).run(3000).await?;
    Ok(())
}
```

HTML lives in a compile-time `html!` macro, routes are typed and pages are
server-rendered — HTMX-ready, with no JavaScript bundle to build. This snippet
comes from the [Zero-to-Hero tutorial](https://rullst.github.io/Rullst/book/tutorials/01-hello-world.html),
and every tutorial's Rust code is compiled in CI.

<details>
<summary><strong>Prefer a JSON API?</strong></summary>

```rust
use rullst::{Server, ServerError, routes, server::Json};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    framework: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        framework: "Rullst",
    })
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let app = routes![
        get("/api/health" => health),
    ]
    .layer(rullst::server::from_fn(
        rullst::security::headers_middleware,
    ));

    Server::new(app).run(3000).await
}
```

```bash
curl http://127.0.0.1:3000/api/health
# {"status":"ok","framework":"Rullst"}
```

[Build your first REST API step by step](https://rullst.github.io/Rullst/book/tutorials/rest-api-quickstart.html).

</details>

<a id="features"></a>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/features-dark.webp">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/features-light.webp">
    <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/features-dark.webp" alt="One framework, the whole product. Secure by default: Argon2id, sessions, passkeys, OAuth2/OIDC, CSRF, strict headers, WAF and login jail. Data that stays correct: Active Record, transactions, migrations and an outbox on SQLite, PostgreSQL and MySQL. Six real blueprints: API, Blog, SaaS, LMS, Portfolio and ERP. Made for AI coding: explicit APIs, compile-time macros, typed errors, no runtime reflection. Payments and email: Stripe billing with signed webhooks; Resend, SendGrid, Postmark and SMTP. AI built in: OpenAI, Claude, Gemini, DeepSeek and Ollama with prompt-injection filtering and PII masking. See inside your app: Studio telemetry and the Nexus admin with a security radar. Web first, native too: HTMX, JSON APIs, and Tauri desktop and mobile shells via Omni." width="100%">
  </picture>
</p>

Every capability has a documented boundary — security middleware does not
replace your authorization rules, and databases are not interchangeable. See the
[capability ledger](https://rullst.github.io/Rullst/book/capability-ledger.html)
· [Why Rullst?](https://rullst.github.io/Rullst/book/why-Rullst.html)
· [Axum & SQLx escape hatches](https://rullst.github.io/Rullst/book/axum-sqlx-migration.html)

<a id="live-examples"></a>

## 🌍 Built with Rullst

Real applications running today, every one of them built with Rullst. Click to explore — their code and
deployment recipes live in [Rullst/examples](https://github.com/Rullst/examples).

<table>
  <tr>
    <td width="50%" align="center" valign="top">
      <a href="https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/"><img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/demo-showcase.webp" alt="Showcase built with Rullst: SSR, LiveView, Wasm, ORM, billing, security and AI demos" width="100%"></a>
      <br><b>🌐 Showcase</b> — SSR, LiveView, Wasm, ORM, billing, security and AI demos · <a href="https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/">Open ↗</a>
    </td>
    <td width="50%" align="center" valign="top">
      <a href="https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/"><img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/demo-lms.webp" alt="LMS built with Rullst: course catalog and learning platform" width="100%"></a>
      <br><b>🎓 LMS</b> — course catalog and learning platform · <a href="https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/">Open ↗</a>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center" valign="top">
      <a href="https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/"><img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/demo-portfolio.webp" alt="Portfolio built with Rullst: projects, experience, Nexus and Studio" width="100%"></a>
      <br><b>💼 Portfolio</b> — projects, experience, Nexus and Studio · <a href="https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/">Open ↗</a>
    </td>
    <td width="50%" align="center" valign="top">
      <a href="https://saas.rullst.win/"><img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/demo-saas.webp" alt="SaaS in production built with Rullst: real purchases with Stripe" width="100%"></a>
      <br><b>🛒 SaaS in production</b> — real purchases with Stripe · <a href="https://saas.rullst.win/">Open ↗</a>
    </td>
  </tr>
</table>

> The demos scale to zero when idle, so the first visit can take a few seconds
> to wake up — that is hosting startup, not Rullst's request time. The **SaaS
> checkout is live and charges real money**; the Showcase payment demos are not.

▶️ **[Watch: how to build a SaaS with Rullst](https://www.youtube.com/watch?v=nDXLeNM327g)**

Prefer to run something locally? The [reproducible SaaS example](https://github.com/Rullst/Rullst/tree/main/examples/saas)
covers generation, login and tenant-scoped notes on a disposable SQLite database,
and the [WebGPU wave example](https://github.com/Rullst/Rullst/tree/main/examples/webgpu)
serves browser graphics from Rullst.

<a id="the-rullst-ecosystem"></a>

## 🏗️ Architecture

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/architecture-dark.webp">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/architecture-light.webp">
    <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/architecture-dark.webp" alt="Rullst architecture. Your app (API, Blog, SaaS, LMS, Portfolio, ERP) sits on four layers: Product (rullst-capital payments, rullst-mail email, rullst-ai AI and RAG, rullst-messaging queues, rullst-connect OAuth2/OIDC); Trust (rullst-auth identity and sessions, rullst-security WAF, headers, CSRF and RASP); Data (rullst-orm Active Record, migrations and transactions, rullst-orm-macros); Runtime (rullst-core HTTP runtime and routing, rullst-macros html!). Everything runs on Axum, Tokio, Tower and SQLx. Tools: cargo-rullst CLI, rullst-studio control room, rullst-nexus admin. One facade crate, rullst, enables only the features you need." width="100%">
  </picture>
</p>

Rullst is a family of focused crates in one versioned workspace — select only
what your application needs. The stable v12 release publishes sixteen crates;
detailed feature and provider boundaries live in the
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

## 🎓 Learn with Rullst Academy

[Rullst Academy](https://academy.rullst.win/) is a free learning platform — built
with Rullst — where you can learn Rust, Rullst, Git/GitHub, web development,
databases and more through short lessons and practical projects, in Portuguese,
English or Spanish.

**[Start learning at Rullst Academy ↗](https://academy.rullst.win/)**

## 🛡️ Engineered like critical infrastructure

Rullst is maintained with the rigor you expect from security software:

- **No panics in production code**, enforced by CI across the published runtime crates.
- **No new `unsafe`** outside a reviewed OS/FFI allowlist.
- **At least 90% line coverage, enforced**, plus fuzzing, property tests,
  sanitizers and Kani/Miri checks on selected critical code.
- **Supply-chain hygiene**: RustSec and license policy on every change,
  SHA-pinned Actions, and an SBOM with build-provenance attestation for every release.
- **A fast Linux gate on every pull request**, with the complete
  Linux/macOS/Windows matrix every night and before every release.

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/security-radar.webp" alt="Nexus SOC Threat Radar with WAF, honeypot, prompt-injection and audit counters" width="100%"/>
  <br>
  <sub>The Nexus security radar (recorded screenshot).</sub>
</p>

These results are evidence for their stated scope, not a security certification
of every application built with Rullst. Explore the
[release audit](https://rullst.github.io/Rullst/book/v12-release-audit.html),
[capability status](https://rullst.github.io/Rullst/book/capability-status.html)
and [quality scorecard](https://rullst.github.io/Rullst/book/quality-scorecard.html).

<details>
<summary><strong>🛡️ Open the verification dashboard (39 workflows)</strong></summary>

<p align="center">
  Badges are pinned to the <code>main</code> branch; they report the latest matching run, not a certification or deployment guarantee.
</p>

| Continuous or change-aware gate | Development `main` status | Actual scope |
| :--- | :---: | :--- |
| **Rust CI** | [![Rust CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=flat-square&label=Rust%20CI)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain) | Format, all-target/all-feature Clippy, a Linux gate on pull requests, every Linux shard after merge and the complete Linux/macOS/Windows matrix nightly; Cargo-aware doctests sourced from all 52 public tutorials, strict DB boundaries, feature boundaries, generated-code checks, and MSRV 1.96.0. |
| **Declared MSRV** | [![MSRV 1.96.0](https://img.shields.io/badge/MSRV-1.96.0-f74c00?style=flat-square&logo=rust)](https://rullst.github.io/Rullst/book/compatibility-policy.html) | Every publishable manifest declares Rust 1.96.0 and CI runs an explicit workspace all-feature check with that toolchain. |
| **GitHub Actions lint** | [![Workflow Lint](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/workflow-lint.yml?branch=main&style=flat-square&label=Workflow%20Lint)](https://github.com/Rullst/Rullst/actions/workflows/workflow-lint.yml?query=branch%3Amain) | Validates workflow syntax, expressions, embedded shell, and full-SHA third-party Action pins. |
| **Documentation** | [![Documentation](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/documentation.yml?branch=main&style=flat-square&label=Docs)](https://github.com/Rullst/Rullst/actions/workflows/documentation.yml?query=branch%3Amain) | Builds the mdBook and rejects broken local links and anchors; scheduled/manual runs also preserve an informational external-link report. |
| **End-to-end smoke** | [![E2E](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?branch=main&style=flat-square&label=E2E)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml?query=branch%3Amain) | Boots the release blog example and verifies HTTP, security headers, form flow, and SQLite persistence. |
| **Codecov — whole repository** | [![Whole-repository coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg)](https://codecov.io/gh/Rullst/Rullst) | The badge reports the current branch aggregate. The [stable-source LLVM run](https://github.com/Rullst/Rullst/actions/runs/34895523751) at `eb11f892` measured **90.3220%** (79,813/88,365 lines) before Codecov upload. The enforced repository floor is **≥90%** with zero tolerance. |
| **Codecov — framework libraries** | [![Framework library coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries)](https://codecov.io/gh/Rullst/Rullst) | Runtime libraries are enforced separately at **≥90%**. CLI and proc-macro components stay separately visible; [Coverage CI](https://github.com/Rullst/Rullst/actions/workflows/coverage.yml?query=branch%3Amain) uploads their real LCOV evidence with OIDC. |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?branch=main&style=flat-square&label=RustSec)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml?query=branch%3Amain) | RustSec advisory scan with only governed, expiring exceptions; the daily run also audits the stable `v12` locks. |
| **Security exception governance** | [![Security Governance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/security-audit.yml?branch=main&style=flat-square&label=Exception%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/security-audit.yml?query=branch%3Amain) | Cross-checks scanner allowlists against the owner/expiry ledger, then independently reruns Cargo Audit. |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?branch=main&style=flat-square&label=Cargo%20Deny)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml?query=branch%3Amain) | Advisory, license, ban, and source policy. |
| **CodeQL SAST** | [![CodeQL](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?branch=main&style=flat-square&label=CodeQL)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml?query=branch%3Amain) | Rust semantic analysis after an all-target/all-feature build. |
| **OpenSSF Scorecard** | [![OpenSSF Scorecard](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square)](https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst) | The badge renders the score from the official public Scorecard JSON report; the pinned [Scorecard workflow](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) publishes OIDC-authenticated results on each `main` push and weekly. A score is evidence, not a security certification. [Evidence and improvements](https://github.com/Rullst/Rullst/blob/main/docs/src/openssf-scorecard.md). |
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
| [v13 Verus pilot](https://github.com/Rullst/Rullst/blob/main/.github/workflows/verus.yml) | Optional production-linked age-policy proof with pinned tooling and three negative controls. Hosted registration/acceptance remains pending; no framework-wide correctness claim. |
| [GitHub Pages](https://github.com/Rullst/Rullst/actions/workflows/pages.yml) | Deploys development documentation from `main`; it is not a code-quality gate. |
| [Release and provenance](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Exact version tags only: full verification, package-all, evidence bundle, checksums, GitHub build-provenance attestation, changelog-derived release notes, and ordered crates.io publication. This does **not** claim a project-wide SLSA level or independent certification. |

Scheduled events use the repository's default branch, so scheduled and
continuous development evidence refer to `main`; stable v12 has its own branch.
The branch-protection profiles and the exact scope of all
39 workflow definitions in this source branch are documented in
[WORKFLOWS.md](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md).

> 📖 **[Read the detailed breakdown of all CI/CD and security workflows](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md).**
>
> 🧭 **[Capability Status & Vision Decisions](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-ledger.md)** preserves ambitious features that are partial or not implemented, with an explicit recommendation and rationale for each one.
>
> 📋 **[Simple Capability Status](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-status.md)** and the **[per-commit quality scorecard](https://github.com/Rullst/Rullst/blob/main/docs/src/quality-scorecard.md)** keep feature progress separate from SHA-bound engineering evidence.

</details>

## 🔄 Upgrade with a preview

From an existing application's root:

```bash
cargo rullst upgrade --dry-run
cargo rullst upgrade
```

The CLI coordinates dependency updates, backs up controlled files and runs
compiler checks. Review the plan and your application's behavior; it does not
migrate production data. [Assisted upgrade tutorial](https://rullst.github.io/Rullst/book/tutorials/36-assisted-framework-upgrades.html)
· [v5 → v12 migration guide](https://rullst.github.io/Rullst/book/migration-v5-to-v12.html)

## 🚀 What's next: v13

This `main` branch is where **v13** is being built (`13.0.0-alpha.1`). The
commands above install the stable **v12** line, maintained on the
[`v12`](https://github.com/Rullst/Rullst/tree/v12) branch; v5 is no longer maintained.

In development for v13: email login and scoped API tokens, active-session
management with remote logout, private S3/R2 storage with resumable uploads,
durable outgoing webhooks and recurring jobs, Redis Streams messaging,
distributed tracing and a recoverable Live UI. Until v13 is released, these are
development candidates, not shipped features.

[v13 roadmap](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md)
· [v13 adoption guide](https://github.com/Rullst/Rullst/blob/main/docs/src/migration-v13.md)
· [Compatibility policy](https://rullst.github.io/Rullst/book/compatibility-policy.html)
· [v12 release record](https://rullst.github.io/Rullst/book/v12.html)

## ⚡ Performance you can inspect

The [benchmark hub](https://rullst.github.io/Rullst/benches/) publishes eight
Criterion groups backed by nine benchmark binaries. They measure specific
workloads and regressions — not universal speed or a ranking of frameworks.
Read the [methodology](https://rullst.github.io/Rullst/book/tutorials/35-high-performance-benchmarking.html)
alongside the results.

## 🤝 Build Rullst with us

Try a blueprint, report a reproducible bug, improve a tutorial or contribute a
focused change with tests. Documentation, accessibility and integration feedback
matter as much as new features.

[Contributing](https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md)
· [Issues](https://github.com/Rullst/Rullst/issues)
· [Discord](https://discord.com/invite/2ntKFtsSjw)
· [Community links](https://rullst.github.io/Rullst/#community)
· [Our story and philosophy](https://rullst.github.io/Rullst/book/philosophy.html)

<p align="center">
  ⭐ <b>If Rullst sparks your curiosity, a star helps more developers discover it.</b>
</p>

[MIT license](https://github.com/Rullst/Rullst/blob/main/LICENSE)
· [Report a vulnerability privately](https://github.com/Rullst/Rullst/security/policy)
· [Website privacy notice](https://rullst.github.io/Rullst/#privacy)

<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>
