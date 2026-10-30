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
  <a href="https://www.bestpractices.dev/projects/13321"><img src="https://www.bestpractices.dev/projects/13321/badge" alt="OpenSSF Best Practices"></a>
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
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/terminal.svg" alt="Terminal: cargo rullst new asks for the project name, starter, database and optional features, creates a SaaS starter with SQLite, then cargo rullst dev builds it and serves it at http://localhost:3000." width="92%">
</p>

<a id="quickstart"></a>

## ⚡ Rullst in 30 seconds

With Rust installed ([rustup.rs](https://rustup.rs/)):

```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
cargo rullst new my_app    # choose the Blank, API or SaaS starter
cd my_app
cargo rullst dev           # rebuilds and restarts every time you save
```

That's it: a running application, generated as readable Rust you can change —
no hidden runtime magic. The first build compiles the framework's dependencies
and can take a few minutes; later rebuilds are much faster
([build times](https://rullst.github.io/Rullst/book/build-times.html)). The
version is named because 13.0 is still a prerelease ([why](#versions)).
Want a bigger starting point? Copy an application from
[`examples/`](https://github.com/Rullst/Rullst/tree/main/examples): a complete
LMS, ERP or portfolio, or the blog showcase
([where the examples live](#where-the-examples-live)).

[Installation and prerequisites](https://rullst.github.io/Rullst/book/1-getting-started.html)
· [Zero-to-Hero tutorial](https://rullst.github.io/Rullst/book/tutorials/01-hello-world.html)
· [Build a JSON REST API](https://rullst.github.io/Rullst/book/tutorials/rest-api-quickstart.html)
· [CLI reference](https://rullst.github.io/Rullst/book/cli_reference.html)

Stuck? `cargo rullst doctor` checks your toolchain and free disk space and,
inside a project, its configuration, database and migrations. `cargo rullst --help`
lists every command, `cargo rullst tour` walks through the main ones, and
`cargo rullst` alone opens an interactive menu.

<details>
<summary><strong>Prefer an interactive dashboard? Run <code>cargo rullst dash</code></strong></summary>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/cargo-rullst-dash.png" alt="Rullst terminal dashboard with project information, logs and controls" width="100%"/>
</p>

Recorded screenshot; layout and controls can differ by version. [Development workflow](https://rullst.github.io/Rullst/book/tutorials/51-authenticated-hot-reload.html).

</details>

## 🦀 Code that says what it does

```rust
use rullst::{html, routes, Markup, Server};

async fn home() -> Markup {
    html! {
        <div class="min-h-screen bg-slate-900 text-emerald-400 flex flex-col items-center justify-center font-sans">
            <h1 class="text-5xl font-extrabold mb-4">"Hello, Rullst! 📜🦀"</h1>
            <p class="text-slate-400 text-lg">"Your first typed route is running."</p>
        </div>
    }
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
and CI compiles it straight from that page.

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
    <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/features-dark.webp" alt="One framework, the whole product. Secure by default: Argon2id, sessions, passkeys, OAuth2/OIDC, CSRF, strict headers, WAF and login jail. Data that stays correct: Active Record, transactions, migrations and an outbox on SQLite, PostgreSQL and MySQL. Starters and examples: Blank, API and SaaS starters, plus LMS, ERP, Portfolio and blog examples, all ordinary Rust you own. Made for AI coding: explicit APIs, compile-time macros, typed errors, no runtime reflection. Payments and email: Stripe billing with signed webhooks; Resend, AWS SES, SendPulse and SMTP. AI built in: OpenAI, Claude, Gemini, DeepSeek and Ollama with prompt-injection filtering and PII masking. See inside your app: Studio telemetry and the Nexus admin with a security radar. Web first, installable: HTMX, JSON APIs, and installable PWAs for Google Play and the Microsoft Store." width="100%">
  </picture>
</p>

Every capability has a documented boundary — security middleware does not
replace your authorization rules, and databases are not interchangeable. See the
[capability ledger](https://rullst.github.io/Rullst/book/capability-ledger.html)
· [Why Rullst?](https://rullst.github.io/Rullst/book/why-Rullst.html)
· [An AI assistant's candid evaluation](https://rullst.github.io/Rullst/book/ai-evaluation.html)
· [Axum & SQLx escape hatches](https://rullst.github.io/Rullst/book/axum-sqlx-migration.html)

<a id="live-examples"></a>

## 🌍 Built with Rullst

Real applications running today, every one of them built with Rullst. Click to explore.

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

<a id="where-the-examples-live"></a>

### Where the examples live

Rullst's examples have two homes:

- **The live demos above** — their code and deployment recipes are in the
  [Rullst/examples](https://github.com/Rullst/examples) repository. They are
  upgraded on their own schedule, so a demo can run an earlier Rullst release
  than the one this page describes.
- **[`examples/`](https://github.com/Rullst/Rullst/tree/main/examples) in this
  repository** — applications built and tested with the current framework
  source: complete [LMS](https://github.com/Rullst/Rullst/tree/main/examples/lms),
  [ERP](https://github.com/Rullst/Rullst/tree/main/examples/erp) and
  [Portfolio](https://github.com/Rullst/Rullst/tree/main/examples/portfolio)
  applications (the former starters of `cargo rullst new`), the
  [blog showcase](https://github.com/Rullst/Rullst/tree/main/examples/blog) and
  a [WebGPU wave demo](https://github.com/Rullst/Rullst/tree/main/examples/webgpu).
  The [SaaS recipe](https://github.com/Rullst/Rullst/tree/main/examples/saas)
  generates the SaaS starter and runs its sign-in and tenant-scoped notes on a
  disposable SQLite database (Linux). Copy any of them to start from it; the
  [examples guide](https://rullst.github.io/Rullst/book/examples.html) shows
  how to run each one.

<a id="the-rullst-ecosystem"></a>

## 🏗️ Architecture

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/architecture-dark.webp">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/architecture-light.webp">
    <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/readme/architecture-dark.webp" alt="Rullst architecture. Your app, from the Blank, API or SaaS starter or copied from an example (LMS, ERP, Portfolio, Blog), sits on four layers: Product (rullst-capital payments, rullst-mail email, rullst-ai AI and RAG, rullst-messaging queues, rullst-connect OAuth2/OIDC); Trust (rullst-auth identity and sessions, rullst-security WAF, headers, CSRF and RASP); Data (rullst-orm Active Record, migrations and transactions, rullst-orm-macros); Runtime (rullst-core HTTP runtime and routing, rullst-macros html!). Everything runs on Axum, Tokio, Tower and SQLx. Tools: cargo-rullst CLI, rullst-studio control room, rullst-nexus admin. One facade crate, rullst, enables only the features you need." width="100%">
  </picture>
</p>

Rullst is a family of focused crates in one versioned workspace: v13 publishes
20 packages with the same version, and the `rullst` facade compiles only the
features you select. Each package has a maturity tier. *Core* packages are
needed by every application. *Extensions* are optional and supported.
*Experimental* packages are optional, may change between 13.x releases and are
not validated in every scenario — experimental does not mean broken. The
[maturity tiers](https://rullst.github.io/Rullst/book/maturity.html) page
defines each promise, and the
[specification](https://rullst.github.io/Rullst/book/spec.html) records the
detailed feature and provider boundaries.

<details>
<summary><strong>Browse the 20 packages</strong></summary>

| Crate | Focus | Tier |
| :--- | :--- | :--- |
| [rullst](https://github.com/Rullst/Rullst/tree/main/rullst) | Public framework facade and feature selection | Core |
| [rullst-core](https://github.com/Rullst/Rullst/tree/main/rullst-core) | HTTP runtime, routing, lifecycle and telemetry | Core |
| [rullst-orm](https://github.com/Rullst/Rullst/tree/main/rullst-orm) | Relational models, transactions and capability-specific persistence | Core |
| [rullst-security](https://github.com/Rullst/Rullst/tree/main/rullst-security) | Defense-in-depth middleware, guards and audit helpers | Core |
| [rullst-macros](https://github.com/Rullst/Rullst/tree/main/rullst-macros) | Compile-time HTML and application macros | Core |
| [rullst-orm-macros](https://github.com/Rullst/Rullst/tree/main/rullst-orm-macros) | Typed ORM code generation | Core |
| [cargo-rullst](https://github.com/Rullst/Rullst/tree/main/cargo-rullst) | Project scaffolding, development and upgrade CLI | Core |
| [rullst-auth](https://github.com/Rullst/Rullst/tree/main/rullst-auth) | Passwords, sessions, passkeys and authorization helpers | Extension |
| [rullst-connect](https://github.com/Rullst/Rullst/tree/main/rullst-connect) | OAuth2/OIDC identity integrations | Extension |
| [rullst-ai](https://github.com/Rullst/Rullst/tree/main/rullst-ai) | Guarded local/cloud clients and tenant-aware retrieval | Extension |
| [rullst-capital](https://github.com/Rullst/Rullst/tree/main/rullst-capital) | Billing contracts, Stripe and experimental InfinitePay adapters, webhooks | Extension |
| [rullst-mail](https://github.com/Rullst/Rullst/tree/main/rullst-mail) | Transactional email with Resend, AWS SES, SendPulse and SMTP transports, and delivery controls | Extension |
| [rullst-messaging](https://github.com/Rullst/Rullst/tree/main/rullst-messaging) | Broker-neutral contracts and durable local messaging | Extension |
| [rullst-studio](https://github.com/Rullst/Rullst/tree/main/rullst-studio) | Local developer control room | Extension |
| [rullst-nexus](https://github.com/Rullst/Rullst/tree/main/rullst-nexus) | Registered-model admin with explicit access policy | Extension |
| [rullst-iot](https://github.com/Rullst/Rullst/tree/main/rullst-iot) | Bounded no_std helpers and signed OTA verification, not device integration | Experimental |
| [rullst-privacy](https://github.com/Rullst/Rullst/tree/main/rullst-privacy) | Proportional age-assurance policies and optional consent records | Experimental |
| [rullst-supervision](https://github.com/Rullst/Rullst/tree/main/rullst-supervision) | Transparent exam and parental supervision contracts | Experimental |
| [rullst-media](https://github.com/Rullst/Rullst/tree/main/rullst-media) | Private course video with Bunny Stream and an optional S3/R2 adapter | Experimental |
| [rullst-labs](https://github.com/Rullst/Rullst/tree/main/rullst-labs) | Programming-exercise contracts; you bring the runner | Experimental |

</details>

## 🤖 For AI assistants

The documentation site publishes [`llms.txt`](https://rullst.github.io/Rullst/llms.txt)
(an index of every book page in reading order) and
[`llms-full.txt`](https://rullst.github.io/Rullst/llms-full.txt) (the full
Markdown of those pages in one file). Projects created with Rullst get their own
`AGENTS.md`, `.llms.txt` and `.rullst/context-map.json`
([`cargo rullst generate:ai-context`](https://rullst.github.io/Rullst/book/project-context.html)),
and commands whose result is data accept `--json`
([machine-readable output](https://rullst.github.io/Rullst/book/cli_reference.html#machine-readable-output)).
Use them with the coding assistant you already have: 13.0 has no built-in
terminal assistant (the `cargo rullst ai` command of earlier prereleases was
removed). The `rullst-ai` crate, which adds AI features to your application, is
not affected.

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
- **At least 90% line coverage, enforced**, plus 42 fuzz targets, property
  tests, sanitizers and Kani/Miri checks on selected critical code.
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
of every application built with Rullst.

<details>
<summary><strong>🛡️ Open the verification dashboard (37 workflows)</strong></summary>

<p align="center">
  Badges are pinned to the <code>main</code> branch; they report the latest matching run, not a certification or deployment guarantee.
</p>

| Continuous or change-aware gate | Development `main` status | Actual scope |
| :--- | :---: | :--- |
| **Rust CI** | [![Rust CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?branch=main&style=flat-square&label=Rust%20CI)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml?query=branch%3Amain) | Format, all-target/all-feature Clippy, a Linux gate on pull requests, every Linux shard after merge and the complete Linux/macOS/Windows matrix nightly; Cargo-aware doctests sourced from every tutorial and the main guides, strict DB boundaries, feature boundaries, generated-code checks, and MSRV 1.96.0. |
| **Declared MSRV** | [![MSRV 1.96.0](https://img.shields.io/badge/MSRV-1.96.0-f74c00?style=flat-square&logo=rust)](https://rullst.github.io/Rullst/book/compatibility-policy.html) | Every publishable manifest declares Rust 1.96.0 and CI runs an explicit workspace all-feature check with that toolchain. |
| **GitHub Actions lint** | [![Workflow Lint](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/workflow-lint.yml?branch=main&style=flat-square&label=Workflow%20Lint)](https://github.com/Rullst/Rullst/actions/workflows/workflow-lint.yml?query=branch%3Amain) | Validates workflow syntax, expressions, embedded shell, and full-SHA third-party Action pins. |
| **Documentation** | [![Documentation](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/documentation.yml?branch=main&style=flat-square&label=Docs)](https://github.com/Rullst/Rullst/actions/workflows/documentation.yml?query=branch%3Amain) | Builds the mdBook and rejects broken local links and anchors; scheduled/manual runs also preserve an informational external-link report. |
| **End-to-end smoke** | [![E2E](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?branch=main&style=flat-square&label=E2E)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml?query=branch%3Amain) | Boots the release blog example and verifies HTTP, security headers, form flow, and SQLite persistence. |
| **Codecov — whole repository** | [![Whole-repository coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg)](https://codecov.io/gh/Rullst/Rullst) | The badge reports the current branch aggregate. The enforced repository floor is **≥90%** with zero tolerance. For reference, the [LLVM run](https://github.com/Rullst/Rullst/actions/runs/34895523751) on the v12.0.0 release commit `eb11f892` measured **90.3220%** (79,813/88,365 lines) before Codecov upload. |
| **Codecov — framework libraries** | [![Framework library coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries)](https://codecov.io/gh/Rullst/Rullst) | Runtime libraries are enforced separately at **≥90%**. CLI and proc-macro components stay separately visible; [Coverage CI](https://github.com/Rullst/Rullst/actions/workflows/coverage.yml?query=branch%3Amain) uploads their real LCOV evidence with OIDC. |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?branch=main&style=flat-square&label=RustSec)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml?query=branch%3Amain) | RustSec advisory scan with only governed, expiring exceptions; the daily run also audits the `v12` locks. |
| **Security exception governance** | [![Security Governance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/security-audit.yml?branch=main&style=flat-square&label=Exception%20Policy)](https://github.com/Rullst/Rullst/actions/workflows/security-audit.yml?query=branch%3Amain) | Cross-checks scanner allowlists against the owner/expiry ledger, then independently reruns Cargo Audit. |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?branch=main&style=flat-square&label=Cargo%20Deny)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml?query=branch%3Amain) | Advisory, license, ban, and source policy. |
| **CodeQL SAST** | [![CodeQL](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?branch=main&style=flat-square&label=CodeQL)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml?query=branch%3Amain) | Rust semantic analysis after an all-target/all-feature build, plus JavaScript/TypeScript analysis. |
| **OpenSSF Scorecard** | [![OpenSSF Scorecard](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square)](https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst) | The badge renders the score from the official public Scorecard JSON report; the pinned [Scorecard workflow](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) publishes OIDC-authenticated results on each `main` push and weekly. A score is evidence, not a security certification. [Evidence and improvements](https://rullst.github.io/Rullst/book/openssf-scorecard.html). |
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
| **PR security evidence** | [![PR-only evidence](https://img.shields.io/badge/trigger-PR--only-2563eb?style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/ai-sentinel-pr.yml?query=event%3Apull_request) | Pull-request-only bounded IDOR/RBAC heuristics and CycloneDX SBOM artifact. It intentionally has no continuous `main` status. |
| **PR mutation diff** | [![PR-only evidence](https://img.shields.io/badge/trigger-PR--only-2563eb?style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/mutants-pr.yml?query=event%3Apull_request) | Pull-request-only cargo-mutants run over the changed Rust lines; a missed mutant fails it. It intentionally has no continuous `main` status. |

Deep or irreversible workflows are intentionally not presented as continuously
green main gates:

| Deep evidence | Trigger and enforcement |
| :--- | :--- |
| [Benchmark regression](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Weekly, `main` push, or manual; eight published groups backed by nine Criterion benchmark binaries emit non-blocking alerts at a 20% regression. |
| [Property testing](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Weekly/manual release-mode invariant testing with 10,000 configured cases. |
| [TSan and ASan](https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml) | Daily/manual package matrices on a pinned verifier-only nightly. |
| [Fuzzing](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) / [corpus minimization](https://github.com/Rullst/Rullst/actions/workflows/corpus-sync.yml) | Forty-two manual libFuzzer jobs, one per declared target, required as release evidence; weekly/manual corpus maintenance is informational. |
| [OWASP ZAP](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Manual baseline over three release surfaces: the generated REST API and the LMS example are blocking with no ignored alerts; the deliberately CDN-backed blog showcase remains an explicitly informational boundary. |
| [Kani](https://github.com/Rullst/Rullst/actions/workflows/kani.yml), [Miri](https://github.com/Rullst/Rullst/actions/workflows/miri.yml), [mutation testing](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml), [cargo-udeps](https://github.com/Rullst/Rullst/actions/workflows/udeps.yml) | Manual or scheduled research signals: selected Kani/Miri scopes are strict, while mutation and unused-dependency findings remain explicitly informational. |
| [v13 Verus pilot](https://github.com/Rullst/Rullst/blob/main/.github/workflows/verus.yml) | Optional production-linked age-policy proof with pinned tooling and three negative controls. Hosted registration/acceptance remains pending; no framework-wide correctness claim. |
| [GitHub Pages](https://github.com/Rullst/Rullst/actions/workflows/pages.yml) | Deploys development documentation from `main`; it is not a code-quality gate. |
| [Release and provenance](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Exact version tags only: exact-SHA admission of the full CI matrix, package-all, evidence bundle, checksums, GitHub build-provenance attestation, changelog-derived release notes, and ordered crates.io publication. This does **not** claim a project-wide SLSA level or independent certification. |

Scheduled events use the repository's default branch, so scheduled and
continuous development evidence refer to `main`; the frozen v12 line has its own branch.
The branch-protection profiles and the exact scope of all
37 workflow definitions in this source branch are documented in
[WORKFLOWS.md](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md).

</details>

- 📖 **[CI/CD and security workflows](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md)**:
  the exact scope of all 37 workflow definitions and the branch-protection profiles.
- 🧰 **[How the repository is maintained](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md)**:
  workstation rules, CI gates, fuzzing evidence and release admission.
- 📋 [Capability status](https://rullst.github.io/Rullst/book/capability-status.html)
  · [capability ledger](https://rullst.github.io/Rullst/book/capability-ledger.html)
  · [per-commit quality scorecard](https://rullst.github.io/Rullst/book/quality-scorecard.html)
  · [OpenSSF Scorecard evidence](https://rullst.github.io/Rullst/book/openssf-scorecard.html)
  · [v12 release audit](https://rullst.github.io/Rullst/book/v12-release-audit.html)

## 🔄 Upgrade with a preview

To move an existing application to a newer Rullst, install the newer CLI and
run, from the application's root:

```bash
cargo rullst upgrade --dry-run
cargo rullst upgrade
```

The CLI coordinates dependency updates, reports the source changes a release
needs at `file:line`, backs up controlled files and runs compiler checks.
Review the plan and your application's behavior; it does not migrate
production data. [Assisted upgrade tutorial](https://rullst.github.io/Rullst/book/tutorials/36-assisted-framework-upgrades.html)
· [v12 → v13 adoption guide](https://rullst.github.io/Rullst/book/migration-v13.html)

<a id="versions"></a>

## 🚀 Versions: v13 is a prerelease

`13.0.0-alpha.4` is the latest v13 prerelease and the 13.0 feature freeze;
`13.0.0-rc.1` (fixes and documentation only) and then `13.0.0` follow. Cargo
never selects a prerelease on its own, so `cargo install cargo-rullst` and
`cargo add rullst` without a version still pick the end-of-life 12.x. Until
13.0.0 is released, name the exact version:

```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
```

Projects generated by `cargo rullst new` already pin the framework with
`=13.0.0-alpha.4`. In a hand-written `Cargo.toml`, use `=` too: a plain
`"13.0.0-alpha.4"` also accepts later prereleases, whose API may change.
`rullst` has no default features, so name the database profile you use and
capabilities such as `auth` or `nexus`
([feature matrix](https://rullst.github.io/Rullst/book/feature-matrix.html)):

```toml
[dependencies]
rullst = { version = "=13.0.0-alpha.4", features = ["strict-postgres"] }
```

**The v12 line reached end of life on 16 October 2026**: 12.3.0 is its last
release and the [`v12`](https://github.com/Rullst/Rullst/tree/v12) branch is
frozen (v5 is no longer maintained either). crates.io has since yanked the
`zip` releases that 12.x's Studio needs, so a new 12.x project no longer
resolves its dependencies; existing projects with a `Cargo.lock` keep
building. Start new work on v13, and move existing applications with the
[v12 → v13 adoption guide](https://rullst.github.io/Rullst/book/migration-v13.html).

### What is new in v13

Since 12.x. The [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md)
and the [v13 adoption guide](https://rullst.github.io/Rullst/book/migration-v13.html)
list every change.

- **Three starters and complete examples:** `cargo rullst new` offers Blank,
  API and SaaS through a wizard that previews the files and commands before
  writing anything (`--dry-run` without prompts) and initializes a Git
  repository (`--vcs none` skips it). The LMS, ERP, Portfolio and blog
  applications are in [`examples/`](https://github.com/Rullst/Rullst/tree/main/examples)
  to copy from.
- **A friendlier CLI:** `cargo rullst tour` (a guided walkthrough of the main
  commands), a searchable command list on the `cargo rullst` home screen, live
  request metrics in `cargo rullst dash`, toolchain, project, database,
  migration and disk checks with fixes in `cargo rullst doctor`,
  `cargo rullst completions <shell>`, and `cargo rullst add <capability>` to
  enable mail, auth, AI, Nexus or Studio in an existing project.
- **One deployment path:** `cargo rullst deploy` writes a Dockerfile,
  `compose.yaml` and `Caddyfile` for any Linux VM with Docker or Podman, with
  guides for Hetzner Cloud, DigitalOcean, Oracle Cloud and Google Cloud
  ([guide](https://rullst.github.io/Rullst/book/vm-deployment.html)).
- **Security evidence report:** `cargo rullst audit --report` writes a
  Markdown, HTML or JSON report mapped to OWASP ASVS 5.0 Level 1, with a
  personal-data inventory and accessibility checks — evidence for a reviewer,
  not a certification ([guide](https://rullst.github.io/Rullst/book/security-report.html)).
- **Efficiency figures (Experimental):** `cargo rullst green` reports measured
  binary and image size, idle memory, dependency count and CPU time per
  request, each with its method, and a carbon figure only as a labelled SCI
  estimate from your own inputs ([guide](https://rullst.github.io/Rullst/book/green.html)).
- **For AI assistants:** every `--json` output follows one contract with a
  `schema` id, and the site publishes `llms.txt` and `llms-full.txt`.
- **Faster development builds:** generated projects get a development profile
  that shortened their first build by 15 to 19 % and a rebuild after an edit by
  about half on the measuring machine
  ([build times](https://rullst.github.io/Rullst/book/build-times.html)).
- **Progressive web apps:** `cargo rullst make:pwa` (manifest, service worker,
  offline page), a Google Play Trusted Web Activity configuration and a
  Microsoft Store guide ([guide](https://rullst.github.io/Rullst/book/tutorials/43-progressive-web-apps.html)).
- **Studio production monitoring:** a read-only monitoring endpoint, and a
  standalone Studio for remote projects with a project switcher, history and
  alerts ([guide](https://rullst.github.io/Rullst/book/production-monitoring.html)).
- **OAuth login:** a login challenge store and `cargo rullst make:oauth` for
  Google, GitHub, Microsoft and Apple.
- **i18n:** translations, CLDR plural rules and number, currency and date
  formatting.
- **Typed transactions:** `Orm::transaction` returns the error of its closure.
- **Minimal default features:** `rullst` and `rullst-orm` compile no database
  code until you select a profile.
- **Views and data:** `html!` returns `Markup` and has `@if`, `@for` and
  `@match`; dirty-tracking `save()`; `Schema` constraints and date/time
  columns; revocable sessions; multipart uploads.
- **Assisted upgrades:** `cargo rullst upgrade` reports v12 → v13 source
  findings at `file:line` and applies per-release rules.
- **Opt-in candidates:** email login, scoped API tokens, active-session
  management with remote logout, private S3/R2 storage with resumable uploads,
  durable outgoing webhooks and recurring publications, Redis Streams
  messaging, distributed tracing and a recoverable Live UI are in the 13.0
  prereleases behind their own features; each guide states what has been
  validated and what is still open.

<details>
<summary><strong>What v13 removed from 12.x</strong></summary>

- The Omni Tauri shells (`make:omni`, `omni`) and `offline-sync`: use a PWA.
- `rullst-capital` providers other than Stripe and the experimental
  InfinitePay, payouts and NFS-e: use those or an application-owned adapter on
  the provider traits.
- `rullst-mail` providers other than Resend, AWS SES, SendPulse and SMTP: write
  a `MailDriver` in your application.
- The `orm-polyglot` and `polyglot` features, which enabled every storage
  adapter at once: name the adapters you use.
- The CLI's Foundry (`foundry:init`, `foundry:deploy`), Nix (`nixify`,
  `new --nix`), Buildah (`generate:buildah`, `new --buildah`) and gRPC
  (`make:grpc`) generators: files they wrote stay yours, and the
  [Advanced deployment](https://rullst.github.io/Rullst/book/advanced-deployment.html)
  page shows Nix, Buildah, Podman Quadlet and Kubernetes setups.
- The Fly.io, Render and Railway targets of `cargo rullst deploy`: it writes
  the Dockerfile, `compose.yaml` and `Caddyfile` for a Linux VM; existing
  provider files keep working with the provider's own CLI.
- The self-updater's download and binary replacement (`update check`,
  `update verify`, `update stage`, `update install`): install a newer CLI with
  `cargo install cargo-rullst --version =<version> --locked`, which a daily
  notice prints.
- The LMS, Blog, Portfolio and ERP starters of `cargo rullst new`: they are
  applications in [`examples/`](https://github.com/Rullst/Rullst/tree/main/examples).

The [v13 adoption guide](https://rullst.github.io/Rullst/book/migration-v13.html)
gives each change its replacement, and `cargo rullst upgrade --dry-run`
reports the ones your application uses.

</details>

[v13 roadmap](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md)
· [Compatibility policy](https://rullst.github.io/Rullst/book/compatibility-policy.html)
· [v12 release record](https://rullst.github.io/Rullst/book/v12.html)

## ⚡ Performance you can inspect

The [benchmark hub](https://rullst.github.io/Rullst/benches/) publishes eight
Criterion groups backed by nine benchmark binaries. They measure specific
workloads and regressions — not universal speed or a ranking of frameworks.
Read the [methodology](https://rullst.github.io/Rullst/book/tutorials/35-high-performance-benchmarking.html)
alongside the results.

## 🤝 Build Rullst with us

Try a starter or an example, report a reproducible bug, improve a tutorial or contribute a
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
