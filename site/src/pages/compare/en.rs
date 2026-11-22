//! The English comparison page (`/compare/`): Rullst, Loco and Topcoat topic by topic.

use rullst::html;
use rullst::html::Markup;

use crate::pages::{Context, Lang, blocks, made_with};

/// The English comparison page (`/compare/`): Rullst, Loco and Topcoat topic by topic.
pub fn page(cx: &Context, results: Markup) -> Markup {
    html! {
        <!doctype html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="referrer" content="no-referrer" />
                <meta name="description" content="Rullst, Loco and Topcoat side by side: rendering, data, authentication, jobs, mail, billing, admin, CLI, deployment, stability and maturity, with the version each fact was checked against." />
                <meta http-equiv="Content-Security-Policy" content="default-src 'self'; style-src 'self'; script-src 'none'; img-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'" />
                <meta property="og:title" content="Rullst, Loco and Topcoat compared" />
                <meta property="og:url" content="https://rullst.win/compare/" />
                <meta property="og:locale" content="en_US" />
                <meta property="og:locale:alternate" content="pt_BR" />
                <title>"Rullst, Loco and Topcoat compared — Rullst"</title>
                {blocks::head_links("https://rullst.win/compare/", "https://rullst.win/compare/", "https://rullst.win/pt-br/comparar/", &cx.assets.css)}
            </head>
            <body>
                {blocks::static_nav(Lang::En, "/compare/", "/pt-br/comparar/", "/#performance")}
                <main id="main" class="shell section compare-page">
                    <p class="eyebrow">"Three Rust web frameworks. Checked facts."</p>
                    <h1>"Rullst, Loco and Topcoat."</h1>
                    <p>"All three build web applications in Rust on Tokio, and each one is the better fit for some projects. This page compares them topic by topic, using what each project's own documentation says. Where a project does not document something, the page says so instead of guessing."</p>
                    <p class="compare-checked" id="checked">"Checked on "<time datetime="2026-11-14">"14 November 2026"</time>" against Rullst "<code>"13.0.0-alpha.5"</code>", Loco "<code>"loco-rs 1.3.1"</code>" and Topcoat "<code>"topcoat 0.12.1"</code>". Rullst's maintainer wrote this page. If a fact is wrong or out of date, please "<a href="https://github.com/Rullst/Rullst/issues">"open an issue"</a>"."</p>
                    <section id="fit" aria-labelledby="fit-heading">
                        <h2 id="fit-heading">"Where each one fits"</h2>
                        <ul class="compare-fit">
                            <li>
                                <h3>"Rullst"</h3>
                                <p>"A good fit when you want one coordinated suite on Axum and SQLx that already includes sessions and passkeys, CSRF protection, billing (Stripe and InfinitePay), transactional mail, a model admin and a local developer control room, with server-rendered HTML."</p>
                                <p>"Keep in mind: 13.0 is still a prerelease, and the project was first published in May 2026 by its sole maintainer."</p>
                            </li>
                            <li>
                                <h3>"Loco"</h3>
                                <p>"A good fit when you want a stable 1.x release with Rails-style conventions, Sea-ORM, JWT or cookie authentication, job queues on Redis, PostgreSQL or SQLite, and generators for models, scaffolds and workers. It has the largest community of the three."</p>
                                <p>"Keep in mind: payments and an admin UI are not part of its documentation."</p>
                            </li>
                            <li>
                                <h3>"Topcoat"</h3>
                                <p>"A good fit when you want server-rendered components whose "<code>"$(...)"</code>" expressions also run in the browser as generated JavaScript, with no Wasm bundle and no client build step."</p>
                                <p>"Keep in mind: its README calls it early-stage and experimental; authentication, background jobs and deployment docs are on its roadmap."</p>
                            </li>
                        </ul>
                    </section>
                    <section id="topics" aria-labelledby="topics-heading">
                        <h2 id="topics-heading">"Topic by topic"</h2>
                        {blocks::topic(
                            "topic-rendering",
                            "Rendering model",
                            html! { "Server-rendered HTML from the "<code>"html!"</code>" macro, escaped by default, with htmx and the CSP build of Alpine.js for interactivity. No client bundle by default." },
                            html! { "Server templates with Tera 2 behind a swappable renderer. Also a client-side SPA mode (Vite and React) and a single-binary mode with embedded assets." },
                            html! { "Server rendering with the "<code>"view!"</code>" macro and async components. "<code>"$(...)"</code>" expressions are type-checked Rust, also translated to JavaScript for reactivity in the browser. Optional htmx, Alpine AJAX and Datastar integrations." },
                        )}
                        {blocks::topic(
                            "topic-database",
                            "Database and ORM",
                            html! { "Its own ORM ("<code>"#[derive(Orm)]"</code>") on SQLx for SQLite, PostgreSQL and MySQL/MariaDB, with "<code>"Schema"</code>" migrations. Turso/libSQL, SurrealDB and Redis data structures through separate adapters." },
                            html! { "Sea-ORM 2.0 with migrations. The documented database drivers are SQLite and PostgreSQL." },
                            html! { "No ORM in the "<code>"topcoat"</code>" crate. "<code>"topcoat new --database toasty"</code>" generates an app that uses Toasty on SQLite, with Toasty's migrations." },
                        )}
                        {blocks::topic(
                            "topic-security",
                            "Authentication and security defaults",
                            html! { "Argon2 password hashing, encrypted sessions, passkeys, RBAC and OAuth2/OIDC sign-in. The SaaS starter wires CSRF middleware; "<code>"rullst-security"</code>" also provides secure headers, a WAF layer and a login jail." },
                            html! { "JWT by default, with argon2 hashing and API keys. Opt-in signed or encrypted cookie sessions and OAuth2/OIDC with GitHub and Google presets. Secure-headers middleware exists but is disabled by default; CSRF middleware is not documented." },
                            html! { "Session tokens with storage you provide, in "<code>"__Host-"</code>", Secure, HttpOnly, SameSite=Lax cookies. A default origin policy rejects cross-origin state-changing requests. Authentication is on the roadmap; password hashing, secure headers and CSRF tokens are not documented." },
                        )}
                        {blocks::topic(
                            "topic-jobs",
                            "Background jobs",
                            html! { "Queues in "<code>"rullst-core"</code>" (in-process, SQLite and Redis). "<code>"rullst-messaging"</code>" adds retries, dead-letter queues, an outbox relay and Redis Streams messaging." },
                            html! { "Queues on Redis, PostgreSQL or SQLite, in-process modes, and a cron-like scheduler." },
                            html! { "Not built in. Background jobs are on the roadmap." },
                        )}
                        {blocks::topic(
                            "topic-mail",
                            "Mail",
                            html! { <code>"rullst-mail"</code>": templated mail through Resend, AWS SES, SendPulse or SMTP, delivered in the background, with recipient suppression and a conformance kit for your own transport." },
                            html! { "A built-in SMTP mailer that sends through the job queue, with a stub mode for tests. An opt-in notifications feature adds Slack and webhooks." },
                            html! { "An opt-in mail feature with SMTP, file (.eml) and in-memory transports." },
                        )}
                        {blocks::topic(
                            "topic-billing",
                            "Payments and billing",
                            html! { <code>"rullst-capital"</code>": Stripe and InfinitePay adapters, verified webhooks with replay protection, quotas, entitlements and invoices." },
                            html! { "Not documented." },
                            html! { "Not documented." },
                        )}
                        {blocks::topic(
                            "topic-admin",
                            "Admin and observability",
                            html! { "Nexus, an admin generated for registered models; Studio, a local developer control room; OpenTelemetry export for distributed traces, tested in CI against an OpenTelemetry Collector in a local container." },
                            html! { "No admin UI documented. Logging with "<code>"tracing"</code>", "<code>"/_ping"</code>", "<code>"/_health"</code>" and "<code>"/_readiness"</code>" endpoints, and "<code>"cargo loco doctor"</code>"." },
                            html! { "No admin framework (an admin demo app exists). Logging and health endpoints are not documented; OpenAPI generation arrived in 0.12.1." },
                        )}
                        {blocks::topic(
                            "topic-cli",
                            "CLI and generators",
                            html! { <code>"cargo rullst new"</code>" (Blank, API and SaaS starters) and "<code>"make:*"</code>" generators for models, migrations, controllers, resources, workers, mail, billing, MFA, OAuth and more; "<code>"dev"</code>", "<code>"doctor"</code>" and assisted "<code>"upgrade"</code>"." },
                            html! { <code>"loco new"</code>" (SaaS, REST API, lightweight and advanced templates) and "<code>"cargo loco generate"</code>" for models, migrations, scaffolds, controllers, tasks, workers, mailers and deployment files." },
                            html! { <code>"topcoat new"</code>", "<code>"dev"</code>", "<code>"fmt"</code>", "<code>"ui"</code>" (copies components into the project), "<code>"asset bundle"</code>" and "<code>"l10n check"</code>". Model or controller generators are not documented." },
                        )}
                        {blocks::topic(
                            "topic-deployment",
                            "Deployment guidance",
                            html! { "Guides for DigitalOcean, Hetzner, Oracle Cloud and Google Cloud. CI tests the generated VM deployment of the Blank and SaaS starters, on SQLite, with Docker and Podman on an x86_64 runner; Arm machines are not tested, and the provider guides were written from the providers' documentation, not followed end to end on real accounts." },
                            html! { "A release binary, plus generated Docker, nginx or AWS Lambda setups, and "<code>"loco doctor"</code>" before going live." },
                            html! { "Deployment docs are on the roadmap. A Cloudflare Workers example, and custom hosting through "<code>"serve()"</code>" or "<code>"Router::handle"</code>"." },
                        )}
                        {blocks::topic(
                            "topic-stability",
                            "Stability, versioning and support window",
                            html! { "13.0 is in prerelease; 13.0.0-alpha.6 froze its features and public API, and 13.0.0-rc.1 adds fixes, documentation and the changes the maintainer approved one by one after the freeze, two of them breaking. After 13.0.0, the maintainer intends to keep 13.x the active line for 12 months, then give it security fixes for 1 to 3 more months after 14.0.0 (an intention, not a guarantee). Semantic Versioning; MSRV Rust 1.96." },
                            html! { "1.0.0 (July 2026) was its first stable release. Upgrade notes list breaking changes; a semver statement and a support window are not documented. MSRV Rust 1.94." },
                            html! { "Its README says: early-stage and experimental, expect breaking changes. Its changelog says it follows Semantic Versioning. No support window documented. MSRV Rust 1.99." },
                        )}
                        {blocks::topic(
                            "topic-maturity",
                            "Maturity and community",
                            html! { "First published in May 2026. MIT license. 21 GitHub stars and 7 contributors on 14 November 2026." },
                            html! { "First release in November 2023. Apache-2.0 license. 9,629 GitHub stars and 157 contributors on 14 November 2026." },
                            html! { "Repository created in April 2026, first functional release in July 2026, in the tokio-rs GitHub organization. MIT license. 7,221 GitHub stars and 41 contributors on 14 November 2026." },
                        )}
                    </section>
                    <section id="ui-frameworks" aria-labelledby="ui-frameworks-heading">
                        <h2 id="ui-frameworks-heading">"Why not Leptos or Dioxus?"</h2>
                        <p class="compare-checked" id="checked-ui">"Checked on "<time datetime="2026-11-19">"19 November 2026"</time>" against Leptos "<code>"leptos 0.8.23"</code>" and Dioxus "<code>"dioxus 0.7.10"</code>"."</p>
                        <p>"Leptos and Dioxus are full-stack in a different sense: they are UI frameworks. The same Rust components can render HTML on the server and then run in the browser as WebAssembly, which hydrates that HTML, and server functions let the browser call Rust code that runs only on the server. Dioxus also targets desktop and mobile."</p>
                        <p>"As UI frameworks, neither ships an ORM, migrations or authentication, and neither documents mail, background jobs or an admin panel. The Leptos book says Leptos itself provides neither the HTTP server nor the database; the Dioxus documentation says Dioxus does not provide a built-in way of managing authentication, and its database tutorial uses "<code>"rusqlite"</code>" and names SQLx and SeaORM among the alternatives. Applications assemble these parts from other crates: the Leptos authentication example and the Dioxus authentication guide both use "<code>"axum_session_auth"</code>" with SQLx."</p>
                        <p>"They are not in the Performance section because it measures equivalent applications serving HTTP requests. The server side of a Leptos application runs on Actix Web or Axum through Leptos's integrations, and Dioxus Fullstack is built on Axum, which is already the benchmark's baseline; an equivalent application would also need its ORM and authentication assembled by hand. The approaches are complementary: a Leptos or Dioxus interface can call a Rullst HTTP API."</p>
                    </section>
                    <section id="benchmarks" aria-labelledby="benchmarks-heading">
                        <h2 id="benchmarks-heading">"Performance"</h2>
                        <p>"These numbers are not written by hand: each daily build of this site inserts the latest published results of the "<a href="/Benchmarks/">"Rullst/Benchmarks"</a>" suite for the three frameworks, Actix Web and Gin, plus a minimal Rullst configuration. Rullst's maintainer also maintains that benchmark and wrote every implementation in it."</p>
                        <div class="benchmark-howto">
                            <h3>"How to read these numbers"</h3>
                            <ul>
                                <li><strong>"1.00× is Axum."</strong>" Every score divides a framework's result by the result of Axum, a small Rust framework used as the reference, measured on the same machine in alternating rounds. 0.50× is half of Axum's result and 2.00× twice. Higher is better in every score: for memory and start-up time the ratio is turned around, so a higher score means less RAM or a faster start."</li>
                                <li><strong>"Ties and ±."</strong>" The machines are shared cloud machines, so results vary from run to run. \"±6%\" next to a score is the margin within which two scores cannot be told apart. Scores that close share a range of positions, such as \"3–7\": those implementations are in positions 3 to 7 in no particular order."</li>
                                <li><strong>"The categories answer different questions."</strong>" Hello world: what a framework costs per request when the page is trivial. Database: how it reads, writes and renders rows. Application: what it does with a real request (input, validation, a write, a form with CSRF protection, a signed-in user). Memory: how much RAM it needs under load. Startup: how fast it starts. Stability (pass or fail): whether it keeps working under heavier load."</li>
                                <li><strong>"The database usually comes first."</strong>" In a real application the database is usually the limit before the framework is, so a large gap in Hello world shrinks or disappears once requests read and write data; Database and Application are closer to what an application does."</li>
                                <li><strong>"The overall score is a summary."</strong>" It combines the five category scores, with Hello world, Database and Application counted twice and Memory and Startup once, for quick reading. The categories are the real result: other weights change the order, as the full results show."</li>
                                <li><strong>"Rullst appears twice."</strong>" \"Rullst\" is the application that "<code>"cargo rullst new"</code>" generates, with the production middleware it turns on: WAF, CSRF protection, secure cookies, security headers and an access log; it is the Rullst with a position. \"Rullst (minimal)\" is the same router, ORM and "<code>"html!"</code>" templates without those layers, served like the Axum app; it is a diagnostic, with scores and without a position. The gap between the two rows is what those layers cost."</li>
                            </ul>
                        </div>
                        <div class="benchmark-results" data-benchmark-results>
                            {results}
                        </div>
                    </section>
                    <section id="sources" aria-labelledby="sources-heading">
                        <h2 id="sources-heading">"Sources"</h2>
                        <ul>
                            <li>"Rullst: "<a href="https://github.com/Rullst/Rullst#integrations">"README integrations"</a>", "<a href="/Rullst/book/maturity.html">"maturity tiers"</a>", "<a href="/Rullst/book/compatibility-policy.html#supported-release-window">"supported release window"</a>", "<a href="/Rullst/book/database-support.html">"database support"</a>", "<a href="https://crates.io/crates/rullst">"crates.io"</a>"."</li>
                            <li>"Loco: "<a href="https://loco.rs/docs/">"documentation"</a>", "<a href="https://github.com/loco-rs/loco/tree/v1.3.1">"repository at v1.3.1"</a>", "<a href="https://crates.io/crates/loco-rs">"crates.io"</a>"."</li>
                            <li>"Topcoat: "<a href="https://github.com/tokio-rs/topcoat/tree/v0.12.1">"repository at v0.12.1"</a>" (README, "<code>"docs/"</code>", changelog), "<a href="https://docs.rs/topcoat/0.12.1">"docs.rs"</a>", "<a href="https://crates.io/crates/topcoat">"crates.io"</a>"."</li>
                            <li>"Leptos: "<a href="https://book.leptos.dev/">"the Leptos book"</a>" ("<a href="https://book.leptos.dev/ssr/22_life_cycle.html">"the life of a page load"</a>", "<a href="https://book.leptos.dev/server/25_server_functions.html">"server functions"</a>", "<a href="https://book.leptos.dev/server/26_extractors.html">"extractors"</a>"), "<a href="https://github.com/leptos-rs/leptos/tree/v0.8.23/projects/session_auth_axum">"authentication example at v0.8.23"</a>", "<a href="https://crates.io/crates/leptos">"crates.io"</a>"."</li>
                            <li>"Dioxus: "<a href="https://dioxuslabs.com/learn/0.7/">"documentation for 0.7"</a>" ("<a href="https://dioxuslabs.com/learn/0.7/guides/platforms/web/">"web"</a>", "<a href="https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/">"router and state"</a>", "<a href="https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/">"authentication"</a>", "<a href="https://dioxuslabs.com/learn/0.7/tutorial/databases/">"databases"</a>"), "<a href="https://crates.io/crates/dioxus">"crates.io"</a>"."</li>
                        </ul>
                    </section>
                </main>
                <footer class="shell footer-bottom"><a href="/">"Rullst home"</a>{made_with::short(Lang::En)}<a href="mailto:officialrullst@gmail.com">"Contact the team"</a></footer>
            </body>
        </html>
    }
}
