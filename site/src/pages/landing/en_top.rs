//! The English landing page (`/`): the document, navigation, first screen, Performance and Security.

use rullst::html;
use rullst::html::Markup;

use super::{art, en_bottom as bottom};
use crate::pages::{Context, INSTALL_VERSION, Lang, blocks};

/// The English landing, with the benchmark blocks `proof` and `summary` (or their pending text).
pub fn page(cx: &Context, proof: Markup, summary: Markup) -> Markup {
    html! {
        <!doctype html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="description" content="Rullst is an open-source full-stack Rust framework suite: web apps on Axum and SQLx with Blank, API and SaaS starters, plus optional packages for AI, payments, mail, messaging, IoT devices, learning platforms and privacy. v13 is in prerelease (13.0.0-rc.1, the first release candidate): see its measured performance, its security defaults and what it includes, each with its source." />
                <meta name="theme-color" content="#070b15" />
                <meta name="referrer" content="no-referrer" />
                <meta http-equiv="Content-Security-Policy" content="default-src 'self'; img-src 'self'; style-src 'self'; script-src 'self'; connect-src 'none'; font-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'" />
                <meta property="og:title" content="Rullst — Intelligent, Security-Conscious, and Designed for Effortless Productivity" />
                <meta property="og:description" content="A full-stack Rust web framework on Axum and SQLx: measured performance, security defaults and what it includes, each with its source." />
                <meta property="og:url" content="https://rullst.win/" />
                <meta property="og:image" content="https://rullst.win/Rullst/Rullst.png" />
                <meta property="og:type" content="website" />
                <meta property="og:locale" content="en_US" />
                <meta property="og:locale:alternate" content="pt_BR" />
                <meta name="twitter:card" content="summary_large_image" />
                <meta name="twitter:title" content="Rullst — Intelligent, Security-Conscious, and Designed for Effortless Productivity" />
                <meta name="twitter:description" content="A full-stack Rust web framework on Axum and SQLx, with Blank, API and SaaS starters. Explore v13, its source and its evidence." />
                <meta name="twitter:image" content="https://rullst.win/Rullst/Rullst.png" />
                <title>"Rullst — Intelligent, Security-Conscious, and Designed for Effortless Productivity"</title>
                {blocks::head_links("https://rullst.win/", "https://rullst.win/", "https://rullst.win/pt-br/", &cx.assets.css)}
                <script src={cx.assets.js.as_str()} defer></script>
            </head>
            <body>
                <a class="skip-link" href="#main">"Skip to content"</a>" "
                <div class="ambient-scene" aria-hidden="true"><span></span><span></span><span></span></div>
                <aside class="release-banner" aria-label="Rullst release status"><span class="status-dot" aria-hidden="true"></span>" "<strong>"Rullst v13 prerelease"</strong>" "<span>"13.0.0-rc.1, published on 26 November 2026, is the first release candidate, after the 13.0 feature freeze in 13.0.0-alpha.6; 13.0.0 follows, so install the exact version. v12 reached end of life on 16 October 2026."</span>" "<a href="/Rullst/book/compatibility-policy.html#source-branches">"Release status ↗"</a></aside>
                {nav()}
                <main id="main">
                    {hero(proof)}
                    {bottom::live()}
                    <div class="foundation-strip shell" aria-label="Framework foundations" data-reveal><span>"BUILT ON FAMILIAR GROUND"</span>" "<strong>"Axum"</strong><strong>"Tokio"</strong><strong>"Tower"</strong><strong>"SQLx"</strong>" "<a href="/Rullst/book/axum-sqlx-migration.html">"Keep your escape hatches ↗"</a></div>
                    {performance(summary)}
                    {security()}
                    {bottom::included()}
                    {bottom::integrations()}
                    {bottom::examples()}
                    {bottom::academy()}
                    {bottom::community()}
                    {bottom::release()}
                </main>
                {bottom::footer(cx)}
                <p class="sr-only" role="status" aria-live="polite" data-copy-status></p>
            </body>
        </html>
    }
}

/// The primary navigation and the language switcher.
fn nav() -> Markup {
    html! {
        <nav class="site-nav" aria-label="Primary navigation">
            <div class="shell nav-inner">
                <a class="brand" href="https://rullst.win/" aria-label="Rullst home"><img src="/Rullst/Rullst.png" alt="" width="40" height="40" />" "<span>"rullst"<span class="brand-period">"."</span></span></a>" "
                {blocks::lang_switch(Lang::En, "/", "/pt-br/", true)}" "
                <button class="nav-toggle" type="button" aria-label="Toggle navigation" aria-controls="navigation" aria-expanded="false" data-nav-toggle>"☰"</button>" "
                <div class="nav-links" id="navigation" data-navigation><a href="#performance">"Performance"</a>" "<a href="/compare/">"Compare"</a>" "<a href="#security">"Security"</a>" "<a href="#included">"Included"</a>" "<a href="#examples">"Examples"</a>" "<a class="nav-academy" href="#academy">"Academy "<span aria-hidden="true">"↗"</span></a>" "<a href="/Rullst/book/">"Documentation"</a>" "<a href="#community">"Community"</a>" "<a class="nav-source" href="https://github.com/Rullst/Rullst">"Source code ↗"</a></div>
            </div>
        </nav>
    }
}

/// The first screen: what Rullst is, the install command, the concept illustration and three proof points.
fn hero(proof: Markup) -> Markup {
    html! {
        <header class="hero shell" data-reveal>
            <div class="hero-content">
                <p class="eyebrow"><span class="short-line" aria-hidden="true"></span>" The open-source Rust framework suite"</p>
                <h1 class="hero-wordmark">"Rullst"<span class="wordmark-period">"."</span></h1>
                <p class="hero-lede"><strong>"Because With Rullst, We Rule!"</strong>" A full-stack Rust framework suite: web apps on "<strong>"Axum"</strong>" and "<strong>"SQLx"</strong>" with "<strong>"Blank"</strong>", "<strong>"API"</strong>" and "<strong>"SaaS"</strong>" starters, plus optional packages for "<a href="/Rullst/book/crates/ai.html">"AI"</a>", "<a href="/Rullst/book/crates/capital.html">"payments"</a>", "<a href="/Rullst/book/crates/mail.html">"mail"</a>", "<a href="/Rullst/book/crates/messaging.html">"messaging"</a>", "<a href="/Rullst/book/crates/iot.html">"IoT devices"</a>", "<a href="/Rullst/book/labs-runner-contract.html">"learning platforms"</a>" and "<a href="https://github.com/Rullst/Rullst/tree/main/rullst-privacy">"privacy"</a>"."</p>
                <p class="hero-why"><strong>"Made for applications with sign-in and payments."</strong>" CSRF protection and secure headers are on from the first commit, with a WAF that blocks in staging and production and logs in development; optional packages in the same release add sign-in, Stripe payments, Pix through InfinitePay, mail and an admin for your data. "<a href="/compare/">"When it fits, and when Loco or Topcoat fits better"</a>"."</p>
                <p class="hero-slogan">"Intelligent. Security-Conscious."<br />"Designed for "<span>"Effortless Productivity."</span></p>
                <div class="hero-actions"><a class="button button-primary" href="/Rullst/book/start-here.html">"Start building "<span aria-hidden="true">"↗"</span></a>" "<a class="button button-secondary" href="/compare/">"Compare with Loco and Topcoat "<span aria-hidden="true">"↗"</span></a></div>
                <div class="hero-tags" aria-label="Project facts"><span>"20 packages, one version"</span><span>"Server-rendered HTML + htmx"</span><span>"MIT licensed"</span></div>" "
                <a class="hero-academy-link" href="#academy"><span aria-hidden="true">"✦"</span>" A new place to learn. Meet Rullst Academy "<span aria-hidden="true">"↓"</span></a>
            </div>
            <div class="workbench">
                <div class="workbench-orbit" aria-hidden="true"></div>
                {art::signal_map()}
                <div class="workbench-label"><span class="status-dot" aria-hidden="true"></span>" YOUR IDEA. A CONNECTED TOOLKIT."<button type="button" class="motion-control" data-intro-toggle aria-pressed="false" hidden>"Replay intro"</button></div>
                <div class="engine" aria-label="Concept illustration: connect your application's data, identity and AI with Rullst. Not live telemetry.">
                    <div class="engine-halo" aria-hidden="true"></div>
                    {art::engine_orbits()}
                    <div class="engine-core" aria-hidden="true">
                        <div class="engine-emblem"><span>"R"</span><span class="engine-corner">"↗"</span></div>
                        <strong>"Your next idea"</strong>
                        <span>"POWERED BY RULLST"</span>
                    </div>
                    <div class="engine-module module-data">
                        <span class="module-symbol" aria-hidden="true">"{ }"</span>
                        <div><strong>"Your data"</strong><span>"Models → queries"</span></div>
                    </div>
                    <div class="engine-module module-identity">
                        <span class="module-symbol" aria-hidden="true">"⌘"</span>
                        <div><strong>"Your boundaries"</strong><span>"Identity + policy"</span></div>
                    </div>
                    <div class="engine-module module-ai">
                        <span class="module-symbol" aria-hidden="true">"✦"</span>
                        <div><strong>"Your intelligence"</strong><span>"Local + cloud AI"</span></div>
                    </div>" "
                    <span class="engine-caption">"CONCEPT MAP / NOT A LIVE MONITOR"</span>
                </div>
            </div>
            <div class="hero-install">
                <p class="install-label"><span class="status-dot" aria-hidden="true"></span>" INSTALL THE CURRENT PRERELEASE"</p>
                <div class="terminal" aria-label="Install commands for the current prerelease">
                    <div class="terminal-bar"><span class="terminal-dots" aria-hidden="true"><span></span><span></span><span></span></span>" "<span>"~/your-next-idea"</span><span class="terminal-mode">"Rust 1.96+"</span></div>
                    <div class="terminal-body">
                        <p class="terminal-comment">"# 13.0 is a prerelease, so name the version"</p>
                        <div class="command-row"><code><span class="prompt">"$"</span>" "<span id="install-command">"cargo install cargo-rullst --version ="{INSTALL_VERSION}" --locked"</span></code><button class="copy-command" type="button" data-copy-command="install-command" aria-label="Copy CLI install command">"Copy"</button></div>
                        <p class="terminal-comment">"# Then choose the Blank, API or SaaS starter"</p>
                        <div class="command-row"><code><span class="prompt">"$"</span>" "<span id="new-command">"cargo rullst new my_app"</span></code><button class="copy-command" type="button" data-copy-command="new-command" aria-label="Copy project creation command">"Copy"</button></div>
                        <details class="workflow-details">
                            <summary>"See what comes next "<span aria-hidden="true">"↗"</span></summary>
                            <div class="project-tree" aria-label="Example layout generated by the SaaS starter">
                                <div><span class="tree-branch">"├─"</span>" src/controllers/ "<span class="tree-note">"your routes"</span></div>
                                <div><span class="tree-branch">"├─"</span>" src/models/ "<span class="tree-note">"your data"</span></div>
                                <div><span class="tree-branch">"├─"</span>" src/security_tests.rs "<span class="tree-note">"your baseline, tested"</span></div>
                                <div><span class="tree-branch">"└─"</span>" Rullst.toml "<span class="tree-note">"your choices"</span></div>
                            </div>
                            <p><code><span class="prompt">"$"</span>" cd my_app"<br /><span class="prompt">"$"</span>" cargo rullst dev"</code></p>
                            <div class="dev-loop"><span>"EDIT"</span><b aria-hidden="true">"→"</b><span>"BUILD"</span><b aria-hidden="true">"→"</b><span>"RESTART"</span><b aria-hidden="true">"→"</b><span>"REFRESH"</span></div>
                            <p class="terminal-comment terminal-caption">"Workflow illustration · SaaS starter layout · the first build compiles the dependencies and can take a few minutes"</p>
                        </details>
                    </div>
                </div>
            </div>
            <ul class="proof-list" aria-label="Three facts, each with its source">
                <li class="proof-card proof-security">
                    <span class="proof-index">"ON IN EVERY GENERATED APP"</span>" "
                    <p class="proof-title">"Security layers on by default"</p>
                    <p class="proof-figure">"Secure headers with a per-request CSP nonce, CSRF protection and a WAF, mounted by "<code>"Server"</code>" in every starter; each project's "<code>"src/security_tests.rs"</code>" checks them. Without "<code>"RULLST_ENV=production"</code>" (or staging) an application runs as development, where the WAF only logs."</p>" "
                    <a href="#security">"Security "<span aria-hidden="true">"↓"</span></a>
                </li>
                <li class="proof-card proof-memory">
                    <span class="proof-index">"MEASURED AGAINST AXUM"</span>" "
                    <p class="proof-title">"Memory and speed, measured"</p>
                    <div class="proof-data" data-landing-proof>
                        {proof}
                    </div>" "
                    <a href="#performance">"Performance "<span aria-hidden="true">"↓"</span></a>
                </li>
                <li class="proof-card proof-verification">
                    <span class="proof-index">"CHECKED IN CI"</span>" "
                    <p class="proof-title">"Automated verification"</p>
                    <p class="proof-figure">"42 fuzz targets, mutation testing, Miri, Kani, sanitizers, OWASP ZAP, CodeQL and dependency audits, across 36 CI workflows."</p>" "
                    <a href="#verification">"Verification "<span aria-hidden="true">"↓"</span></a>
                </li>
            </ul>
        </header>
    }
}

/// 01 / Performance: the Rullst/Benchmarks summary and who maintains that benchmark.
fn performance(summary: Markup) -> Markup {
    html! {
        <section id="performance" class="shell section performance-section" aria-labelledby="performance-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"01 / Performance, measured"</p>
                <h2 id="performance-heading">"Measured against Axum."<br /><span class="accent-orange">"Strong and weak results alike."</span></h2>
                <p>"The scores below are those of Rullst as "<code>"cargo rullst new"</code>" generates it, with its security middleware on. The "<a href="/Benchmarks/">"Rullst/Benchmarks"</a>" cross-framework comparison measures each new Rullst release against Axum (1.00×) and other frameworks on GitHub-hosted runners, and this summary is rebuilt from its latest results every day; the results page says whether they pass the benchmark's own publication gate."</p>
            </div>
            <div class="landing-explain">
                <h3>"What the numbers mean for a real application"</h3>
                <p>"Axum is the reference: a small, fast Rust web framework with no database layer, no sessions and no security middleware. A score of 1.00× means the same result as Axum and 0.50× means half of it; for memory and start-up the ratio is turned around, so higher always means better. Rullst is Axum plus an ORM, "<code>"html!"</code>" templates, sessions and the security layers a generated application turns on, so some cost against bare Axum is expected. The question is where it shows."</p>
                <ul>
                    <li><strong>"Where to look first: "</strong>"memory and start-up, which matter when you pay for a small machine or restart often. How Rullst did on both in the current results, and its memory once users are signed in (a figure in no score), is in the plain-words list below, rebuilt with every run."</li>
                    <li><strong>"The trade-off: "</strong>"throughput. "<code>"Server"</code>"'s production stack (the WAF, CSRF protection, secure cookies and secure headers) is on in every generated application, and each request pays for it. That is why the benchmark has a second Rullst row, served without "<code>"Server"</code>": the gap between the two rows is roughly what that stack costs, "<code>"Server"</code>" itself included. The access log, also on by default, is off in the benchmark, as per-request logging is for every implementation, so its cost is not in that gap."</li>
                    <li><strong>"The database pool: "</strong>"since 13.0.0-rc.1, Rullst's pool pings a PostgreSQL connection before handing it out only when it has been idle for at least 500 ms, while the Axum reference pings on every acquisition (SQLx's default). Rullst's Database score against Axum includes that difference: in the 27 November 2026 run, Axum without the ping scored 1.24× in Database ("<a href="/Benchmarks/diagnostics/#pool">"database-layer defaults"</a>")."</li>
                    <li><strong>"What 13.1 works on: "</strong>"raw requests per second on trivial pages, rendering long lists with "<code>"html!"</code>" and writes that open one transaction per row, where earlier runs measured Rullst behind. The 13.1 performance work must show before-and-after numbers and may not weaken a security layer: "<a href="https://github.com/Rullst/Rullst/issues/832">"issue #832"</a>"."</li>
                </ul>
            </div>
            <p class="landing-disclosure"><strong>"Who measured this."</strong>" Rullst's maintainer also maintains that benchmark, wrote every implementation in it and designed the method. Its README lists what it does not measure: logins, large requests, TLS, a database on another host and scaling beyond 2 vCPUs among them."</p>
            <div class="landing-bench" data-landing-benchmarks>
                {summary}
            </div>
            <a class="compare-callout" href="/compare/"><span class="compare-callout-index">"SIDE BY SIDE"</span><strong>"Rullst, Loco and Topcoat compared"</strong><span>"What each one chose, the advantage and the cost of each choice, with the version and date every fact was checked "<span aria-hidden="true">"↗"</span></span></a>
            <div class="performance-links"><a href="/compare/">"Rullst, Loco and Topcoat compared, with the full table "<span aria-hidden="true">"↗"</span></a>" "<a href="/Benchmarks/">"Full results, method and limitations "<span aria-hidden="true">"↗"</span></a>" "<a href="/Rullst/benches/">"Microbenchmark history on "<code>"main"</code>" (regression checks, not a ranking) "<span aria-hidden="true">"↗"</span></a></div>
        </section>
    }
}

/// 02 / Security: on by default, opt-in and verified in CI.
fn security() -> Markup {
    html! {
        <section id="security" class="shell section security-section" aria-labelledby="security-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"02 / Security, by default and by test"</p>
                <h2 id="security-heading">"More protections on by default."<br /><span class="accent-green">"More automated verification."</span></h2>
                <p>"What a generated application starts with, what you add when you need it, and what CI runs against the framework's own code. None of it proves that an application is secure: its authorization rules, provider settings and deployment stay yours."</p>
            </div>
            <div class="security-grid">
                <article class="security-column security-default">
                    <span class="security-index">"01 / ON BY DEFAULT"</span>" "
                    <h3>"In every generated application"</h3>
                    <ul>
                        <li><b>"Secure headers"</b>" with a per-request CSP nonce, in every environment."</li>
                        <li><b>"CSRF protection"</b>" (a double-submit token) on write routes, in every environment."</li>
                        <li>"A "<b>"WAF"</b>" that refuses suspicious requests in staging and production and only logs them in development. The generated Dockerfile, Compose and Kubernetes files set "<code>"RULLST_ENV=production"</code>"."</li>
                        <li><b>"Security tests"</b>": each project's "<code>"src/security_tests.rs"</code>" checks the headers, CSRF and the WAF under the production baseline with "<code>"cargo test"</code>"."</li>
                        <li><b>"Production checks"</b>": the API starter does not start without "<code>"API_TOKEN"</code>", and the generated billing code refuses an empty payment key in production. Since 13.0.0-rc.1, the "<code>"Mail"</code>" facade and "<code>"AiClient::auto()"</code>" refuse an empty setting in staging and production instead of using an offline mock, newly generated Turso starters refuse an empty "<code>"TURSO_DATABASE_URL"</code>" there, and newly generated billing code also refuses empty keys in staging (code generated before rc.1 keeps its behaviour), and CI fails on an unreviewed fallback that matches its known patterns."</li>
                    </ul>" "
                    <a href="/Rullst/book/security-layers.html#what-generated-applications-already-have">"What each starter mounts "<span aria-hidden="true">"↗"</span></a>
                </article>
                <article class="security-column security-opt-in">
                    <span class="security-index">"02 / OPT-IN"</span>" "
                    <h3>"When your application needs them"</h3>
                    <ul>
                        <li><b>"Passkeys"</b>" (WebAuthn) in "<code>"rullst-auth"</code>", with an optional shared PostgreSQL ceremony store."</li>
                        <li><b>"OAuth2/OIDC sign-in"</b>": "<code>"cargo rullst make:oauth"</code>" for Google, GitHub, Microsoft or Apple."</li>
                        <li><b>"Two-step verification"</b>" with TOTP and recovery codes: "<code>"cargo rullst make:mfa"</code>"."</li>
                        <li>"A "<b>"login jail"</b>" ("<code>"LoginGuard"</code>") and more layers in "<code>"rullst-security"</code>": honeypot paths, response secret masking, a WebSocket origin guard and JSON schema guards."</li>
                    </ul>" "
                    <a href="/Rullst/book/security-layers.html#recommended-production-composition">"Recommended production stack "<span aria-hidden="true">"↗"</span></a>
                </article>
                <article id="verification" class="security-column security-verification">
                    <span class="security-index">"03 / VERIFIED IN CI"</span>" "
                    <h3>"Automated checks on the framework's code"</h3>
                    <ul>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml">"42 fuzz targets"</a>" (libFuzzer), required as release evidence."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/mutants-pr.yml">"Mutation testing"</a>" of each pull request's changed lines (not a required check), plus "<a href="https://github.com/Rullst/Rullst/actions/workflows/mutants.yml">"manual campaigns"</a>"."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/miri.yml">"Miri"</a>" and "<a href="https://github.com/Rullst/Rullst/actions/workflows/kani.yml">"Kani"</a>" on selected critical code, weekly and for each release."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml">"ThreadSanitizer and AddressSanitizer"</a>", daily."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml">"OWASP ZAP"</a>" scans of the generated REST API, SaaS starter and LMS example: blocking, with seven explained rules reported as INFO."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/codeql.yml">"CodeQL"</a>" analysis of the Rust and JavaScript code."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/supply-chain.yml">"Dependency audit"</a>": RustSec advisories, licenses and sources on every change and twice a day."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml">"No "<code>"unwrap"</code>", "<code>"expect"</code>" or "<code>"panic!"</code></a>" in production code, denied by Clippy."</li>
                    </ul>" "
                    <a href="https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md">"All 36 workflows and their exact scope "<span aria-hidden="true">"↗"</span></a>
                </article>
            </div>
            <p class="security-compare" id="checked-security">"How the defaults compare: Loco ("<code>"loco-rs 1.3.1"</code>") documents secure-headers middleware that is disabled by default and no CSRF middleware; Topcoat ("<code>"topcoat 0.12.1"</code>") rejects cross-origin state-changing requests by default and documents no secure headers or CSRF tokens. Checked on "<time datetime="2026-11-27">"27 November 2026"</time>"; "<a href="/compare/#topic-security">"see them side by side"</a>"."</p>
            <p class="security-review"><strong>"Review."</strong>" Rullst has one maintainer, and pull requests need no second reviewer. AI-assisted review and automated checks are not independent human review, and no external audit has been completed. "<a href="/Rullst/book/assurance-case.html">"Security assurance case "<span aria-hidden="true">"↗"</span></a>" "<a href="/Rullst/book/external-audit-kit.html">"External audit kit, for reviewers "<span aria-hidden="true">"↗"</span></a>" "<a href="https://github.com/Rullst/Rullst/security/policy">"Report a vulnerability "<span aria-hidden="true">"↗"</span></a></p>
        </section>
    }
}
