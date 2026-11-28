//! The English comparison page (`/compare/`): Rullst, Loco and Topcoat topic by topic.

use rullst::html;
use rullst::html::Markup;

use crate::pages::blocks::{self, Choice};
use crate::pages::{Context, Lang, made_with};

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
                    <p class="eyebrow"><span class="short-line" aria-hidden="true"></span>" Three Rust web frameworks. Checked facts."</p>
                    <h1>"Rullst, Loco and Topcoat,"<br /><span class="accent-green">"what each one chose and what it costs."</span></h1>
                    <p class="compare-lede">"All three build web applications in Rust on Tokio, and each one is the better fit for some projects. For every topic this page says in plain words what it is, then what each framework chose, the advantage of that choice and its cost. The choice comes from each project's own documentation; where a project does not document something, the page says so instead of guessing. The advantage and the cost are the author's reading of that choice, not a measurement."</p>
                    <p class="compare-checked" id="checked">"Checked on "<time datetime="2026-11-27">"27 November 2026"</time>" against Rullst "<code>"13.0.0-rc.1"</code>" (the source on "<code>"main"</code>"), Loco "<code>"loco-rs 1.3.1"</code>" and Topcoat "<code>"topcoat 0.12.1"</code>", the latest release of each that day. Loco's documentation website still showed its 1.2.0 pages that day, so what concerns sessions, OAuth and notifications comes from the 1.3.1 sources in its repository. Rullst's maintainer wrote this page. If a fact is wrong or out of date, please "<a href="https://github.com/Rullst/Rullst/issues">"open an issue"</a>"."</p>
                    <ul class="compare-jump" aria-label="Topics">
                        <li><a href="#fit">"Where each fits"</a></li><li><a href="#topic-rendering">"Rendering"</a></li><li><a href="#topic-database">"Database"</a></li><li><a href="#topic-security">"Security"</a></li><li><a href="#topic-jobs">"Jobs"</a></li><li><a href="#topic-mail">"Mail"</a></li><li><a href="#topic-billing">"Payments"</a></li><li><a href="#topic-admin">"Admin"</a></li><li><a href="#topic-cli">"CLI"</a></li><li><a href="#topic-deployment">"Deployment"</a></li><li><a href="#topic-stability">"Stability"</a></li><li><a href="#topic-maturity">"Community"</a></li><li><a href="#benchmarks">"Performance"</a></li>
                    </ul>
                    <section id="fit" aria-labelledby="fit-heading">
                        <h2 id="fit-heading">"Where each one fits"</h2>
                        <p>"The table sums up what the topics below document, and the cards say when each framework is the better choice. Not documented means the project's documentation does not describe it; you can still add it yourself."</p>
                        <p class="compare-glance-caption" id="glance-caption">"At a glance: what each framework documents, checked on 27 November 2026."</p>
                        <div class="compare-glance" role="region" aria-labelledby="glance-caption" tabindex="0">
                            <table aria-labelledby="glance-caption">
                                <thead><tr><th scope="col">"You need"</th><th scope="col">"Rullst"</th><th scope="col">"Loco"</th><th scope="col">"Topcoat"</th></tr></thead>
                                <tbody>
                                    <tr><th scope="row">"CSRF protection"</th><td>"On in every generated starter"</td><td>"Not documented"</td><td>"An origin check, on by default; CSRF tokens are not documented"</td></tr>
                                    <tr><th scope="row">"Secure headers"</th><td>"On in every generated starter"</td><td>"Available, off by default"</td><td>"Not documented"</td></tr>
                                    <tr><th scope="row">"Filter for hostile requests (WAF)"</th><td>"On: refuses them in staging and production, only logs them in development, off in tests"</td><td>"Not documented"</td><td>"Not documented"</td></tr>
                                    <tr><th scope="row">"User accounts and passwords"</th><td>"Argon2 and encrypted sessions; opt-in passkeys and OAuth2/OIDC"</td><td>"JWT and Argon2id; opt-in sessions and OAuth2"</td><td>"Sessions with storage you provide; authentication is on its roadmap, password hashing is not documented"</td></tr>
                                    <tr><th scope="row">"Payments and their webhooks"</th><td>"Stripe (signed webhooks, replay checks) and InfinitePay for Pix (each notification confirmed with InfinitePay)"</td><td>"Not documented"</td><td>"Not documented"</td></tr>
                                    <tr><th scope="row">"Admin for your data"</th><td>"Generated for the models you register (Nexus)"</td><td>"Not documented"</td><td>"Not built in (an admin demo app exists)"</td></tr>
                                    <tr><th scope="row">"Background jobs"</th><td>"In-process, SQLite or Redis queues and a cron scheduler; retries and dead-letter queues in rullst-messaging"</td><td>"In-process, Redis, PostgreSQL or SQLite, with a scheduler"</td><td>"On its roadmap"</td></tr>
                                    <tr><th scope="row">"Mail services"</th><td>"Resend, AWS SES, SendPulse, SMTP"</td><td>"SMTP"</td><td>"SMTP"</td></tr>
                                    <tr><th scope="row">"Databases"</th><td>"SQLite, PostgreSQL, MySQL/MariaDB"</td><td>"SQLite, PostgreSQL"</td><td>"SQLite, through Toasty"</td></tr>
                                    <tr><th scope="row">"Browser behaviour written in Rust"</th><td>"No: htmx and Alpine.js"</td><td>"No: Tera templates, or a React SPA"</td><td>"Yes: "<code>"$(...)"</code>" expressions become JavaScript"</td></tr>
                                    <tr><th scope="row">"Stable release"</th><td>"Not yet: 13.0 is in release candidates"</td><td>"Yes: 1.x since July 2026"</td><td>"No: 0.x, which its README calls experimental"</td></tr>
                                    <tr><th scope="row">"Community"</th><td>"The smallest of the three; one maintainer"</td><td>"The largest of the three"</td><td>"Growing, in the tokio-rs organization"</td></tr>
                                </tbody>
                            </table>
                        </div>
                        <ul class="compare-fit">
                            <li>
                                <h3>"Choose Rullst when"</h3>
                                <ul>
                                    <li>"your application has sign-in and forms, and you want CSRF protection and secure headers on from the first commit, a WAF that blocks in staging and production, and a generated test that checks them;"</li>
                                    <li>"you charge customers with Stripe, with webhook signatures and replays checked for you, or with Pix through InfinitePay, with each payment confirmed with InfinitePay;"</li>
                                    <li>"staff need an admin over the application's data and you would rather not build one;"</li>
                                    <li>"you need MySQL or MariaDB, or mail through Resend, AWS SES or SendPulse (the last two tested offline only);"</li>
                                    <li>"you want htmx and the CSP build of Alpine.js already set up, with "<code>"html!"</code>" escaping text by default."</li>
                                </ul>
                                <p>"Keep in mind: 13.0 is still a prerelease, the project was first published in May 2026 by its sole maintainer, it has had no external audit, and every request pays for those security layers (see "<a href="#benchmarks">"Performance"</a>")."</p>
                            </li>
                            <li>
                                <h3>"Choose Loco when"</h3>
                                <ul>
                                    <li>"you need a stable 1.x release today;"</li>
                                    <li>"you want Rails-style conventions and generators for models, scaffolds, workers and deployment files;"</li>
                                    <li>"you prefer Sea-ORM, or JWT authentication for API clients;"</li>
                                    <li>"you want job queues on PostgreSQL as well as Redis or SQLite, and the largest community of the three."</li>
                                </ul>
                                <p>"Keep in mind: payments, an admin UI and CSRF protection are not part of its documentation, and its secure headers stay off until you enable them."</p>
                            </li>
                            <li>
                                <h3>"Choose Topcoat when"</h3>
                                <ul>
                                    <li>"you want browser behaviour written in Rust: its "<code>"$(...)"</code>" expressions run in the browser as generated JavaScript, with no Wasm bundle and no client build step;"</li>
                                    <li>"you want strict session cookies and a cross-origin check on by default, or OpenAPI generation for your API;"</li>
                                    <li>"you want a small framework maintained in the tokio-rs organization."</li>
                                </ul>
                                <p>"Keep in mind: its README calls it early-stage and experimental; authentication and background jobs are on its roadmap, and so are its deployment docs."</p>
                            </li>
                        </ul>
                    </section>
                    <section id="topics" aria-labelledby="topics-heading">
                        <h2 id="topics-heading">"Topic by topic"</h2>
                        {blocks::topic(Lang::En,
                            "topic-rendering",
                            "Rendering model",
                            html! { <strong>"In plain words: "</strong>"how a page reaches the browser: as finished HTML built on the server, or as a program that builds the page in the browser. It decides how much JavaScript you write and ship." },
                            [
                                Choice {
                                    chose: html! { "Server-rendered HTML from the "<code>"html!"</code>" macro, escaped by default, with htmx and the CSP build of Alpine.js for interactivity. No client bundle by default." },
                                    plus: html! { "Pages work with little or no JavaScript, and text from users is escaped unless you mark it as trusted." },
                                    minus: html! { "Rich in-browser behaviour beyond htmx and Alpine is more work, and rendering long lists with "<code>"html!"</code>" is one of the 13.1 performance targets." },
                                },
                                Choice {
                                    chose: html! { "Server templates with Tera 2 behind a swappable renderer. Also a client-side SPA mode (React) and an option to embed assets in the binary." },
                                    plus: html! { "A familiar template language, and a choice between server pages and a React SPA when you create the project." },
                                    minus: html! { "A React SPA needs its own JavaScript tooling, so the two styles are a decision to make early." },
                                },
                                Choice {
                                    chose: html! { "Server rendering with the "<code>"view!"</code>" macro and async components. "<code>"$(...)"</code>" expressions are type-checked Rust, also translated to JavaScript for reactivity in the browser, with no Wasm bundle and no client build step. Optional htmx, Alpine AJAX and Datastar integrations." },
                                    plus: html! { "Browser behaviour written in Rust, without a separate client build." },
                                    minus: html! { "Its README calls it early-stage and experimental: expect breaking changes." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-database",
                            "Database and ORM",
                            html! { <strong>"In plain words: "</strong>"most applications keep their data in a database. An ORM maps tables to Rust types so you write less SQL, and the databases it supports decide where you can run." },
                            [
                                Choice {
                                    chose: html! { "Its own ORM ("<code>"#[derive(Orm)]"</code>") on SQLx for SQLite, PostgreSQL and MySQL/MariaDB, with "<code>"Schema"</code>" migrations. Turso/libSQL, SurrealDB and Redis data structures through separate adapters." },
                                    plus: html! { "One ORM and one migration system, over more database types than the others document, with parameterized SQLx underneath." },
                                    minus: html! { "An ORM written for Rullst rather than an established third-party one, so you learn its API, and it has one maintainer." },
                                },
                                Choice {
                                    chose: html! { "Sea-ORM 2.0 with migrations. Loco 1.3.1 has database driver features for SQLite and PostgreSQL ("<code>"db-sqlite"</code>", "<code>"db-postgres"</code>")." },
                                    plus: html! { "It builds on an ORM that is a project of its own." },
                                    minus: html! { "MySQL has no driver feature in Loco 1.3.1, although Sea-ORM itself supports it." },
                                },
                                Choice {
                                    chose: html! { "No ORM in the "<code>"topcoat"</code>" crate. "<code>"topcoat new --database toasty"</code>" generates an app that uses Toasty on SQLite, with Toasty's migrations." },
                                    plus: html! { "A new project starts with a database layer and migrations already wired." },
                                    minus: html! { "Documented only for SQLite through Toasty; better Toasty integration is on its roadmap." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-security",
                            "Authentication and security defaults",
                            html! { <strong>"In plain words: "</strong>"hashing stores passwords so a stolen database does not reveal them (Argon2 is a deliberately slow one). An encrypted session means a stolen cookie cannot be read or changed. A passkey replaces the password with a key on your device. RBAC lets roles decide who may do what, and OAuth2/OIDC is \"sign in with Google\". Secure headers, CSRF protection and a WAF (a filter for hostile requests) block common attacks. A protection that is on for every request also costs time on every request." },
                            [
                                Choice {
                                    chose: html! { "Argon2 password hashing and encrypted sessions in "<code>"rullst-auth"</code>"; passkeys, RBAC and OAuth2/OIDC sign-in when you need them. In every generated starter, "<code>"Server"</code>" mounts secure headers and double-submit CSRF protection in every environment, and a WAF that refuses suspicious requests in staging and production and only logs them in development (it does not run in the test environment); "<code>"rullst-security"</code>" adds a login jail and more layers." },
                                    plus: html! { "More protections on by default: a new application starts with them and a generated test checks them, instead of you finding and enabling each one." },
                                    minus: html! { "Every request pays for them, which lowers throughput (see "<a href="#benchmarks">"Performance"</a>"). One maintainer, no external audit, and the authorization rules of your application stay yours." },
                                },
                                Choice {
                                    chose: html! { "JWT by default, with Argon2id hashing and API keys. Opt-in sessions, kept in a signed or encrypted cookie or, since 1.3.0, in a database or Redis store with remember-me, and OAuth2 with GitHub and Google presets. Secure-headers middleware (presets such as "<code>"github"</code>" and "<code>"owasp"</code>") exists but is disabled by default; CSRF middleware is not documented. On by default: a payload limit, panic catching, ETag, request ids, logging, an "<code>"X-Powered-By"</code>" header and, outside production, a fallback page." },
                                    plus: html! { "Stateless tokens suit API clients, and secure headers have ready presets you can switch on." },
                                    minus: html! { "Secure headers stay off until you enable them, and no CSRF protection or WAF is documented, so an application that uses cookie sessions adds its own." },
                                },
                                Choice {
                                    chose: html! { "Session tokens with storage you provide (sign-in and sign-out, sliding expiration and token rotation), in "<code>"__Host-"</code>", Secure, HttpOnly, SameSite=Lax cookies. A default origin policy rejects cross-origin state-changing requests. Authentication is on the roadmap; password hashing, secure headers and CSRF tokens are not documented." },
                                    plus: html! { "Strict cookie settings and an origin check that is on by default, in a small surface." },
                                    minus: html! { "No user accounts or password handling yet: you bring the session storage, the password handling and the headers. The origin policy does not protect a state-changing GET request." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-jobs",
                            "Background jobs",
                            html! { <strong>"In plain words: "</strong>"work that should not make a visitor wait, such as sending email or running a nightly task. A queue stores these jobs and runs them in the background, retrying when one fails." },
                            [
                                Choice {
                                    chose: html! { "Queues in "<code>"rullst-core"</code>" (in-process, SQLite and Redis). "<code>"rullst-messaging"</code>" adds retries, dead-letter queues, an outbox relay and Redis Streams messaging." },
                                    plus: html! { "Retries and dead-letter queues (a place for jobs that keep failing) come with the suite." },
                                    minus: html! { "Redis Streams is the only remote broker adapter, and the outbox relay does not make the database and the broker one atomic transaction." },
                                },
                                Choice {
                                    chose: html! { "Queues on Redis, PostgreSQL or SQLite, in-process modes, and a cron-like scheduler." },
                                    plus: html! { "A scheduler for recurring jobs, and a choice of three queue stores." },
                                    minus: html! { "Other brokers are not documented." },
                                },
                                Choice {
                                    chose: html! { "Not built in. Background jobs are on the roadmap." },
                                    plus: html! { "Nothing to configure if you need no jobs." },
                                    minus: html! { "Bring your own queue, or wait for the roadmap item." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-mail",
                            "Mail",
                            html! { <strong>"In plain words: "</strong>"password resets and receipts are email. A mail engine fills a template and hands the message to a sending service." },
                            [
                                Choice {
                                    chose: html! { <code>"rullst-mail"</code>": templated mail through Resend, AWS SES, SendPulse or SMTP, delivered in the background, with recipient suppression and a conformance kit for your own transport." },
                                    plus: html! { "Several services behind one interface, with a suppression list for addresses you choose not to mail again." },
                                    minus: html! { "Resend and SMTP are used by the maintainer's production applications; AWS SES and SendPulse have only been tested offline." },
                                },
                                Choice {
                                    chose: html! { "A built-in SMTP mailer that sends through the job queue, with a stub mode for tests. An opt-in notifications feature adds Slack and webhooks." },
                                    plus: html! { "Mail goes through the job queue, and the stub makes tests easy." },
                                    minus: html! { "SMTP is the transport documented for real delivery." },
                                },
                                Choice {
                                    chose: html! { "An opt-in mail feature with SMTP, file (.eml) and in-memory transports." },
                                    plus: html! { "The file and in-memory transports let you test mail without sending any." },
                                    minus: html! { "SMTP is the only one of the three that sends real mail." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-billing",
                            "Payments and billing",
                            html! { <strong>"In plain words: "</strong>"charging customers, receiving the payment provider's notifications (webhooks) safely, and tracking what each customer may use." },
                            [
                                Choice {
                                    chose: html! { <code>"rullst-capital"</code>": Stripe and InfinitePay adapters, verified webhooks with replay protection, quotas, entitlements and invoices." },
                                    plus: html! { "Payment code you do not write yourself: Stripe webhooks with signature checks and replay protection, and InfinitePay payments confirmed with InfinitePay." },
                                    minus: html! { "Stripe has real purchases in the maintainer's SaaS but no automated test against a live account; InfinitePay was validated with one real Pix payment. Another gateway means writing an adapter." },
                                },
                                Choice {
                                    chose: html! { "Not documented." },
                                    plus: html! { "No payment code in the framework: use any provider's own library." },
                                    minus: html! { "You build and secure the integration, webhooks included." },
                                },
                                Choice {
                                    chose: html! { "Not documented." },
                                    plus: html! { "No payment code in the framework: use any provider's own library." },
                                    minus: html! { "You build and secure the integration, webhooks included." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-admin",
                            "Admin and observability",
                            html! { <strong>"In plain words: "</strong>"an admin screen lets staff edit data without code. Observability is how you see what the running application is doing: logs, health checks and traces." },
                            [
                                Choice {
                                    chose: html! { "Nexus, an admin generated for registered models; Studio, a local developer control room; OpenTelemetry export for distributed traces, tested in CI against an OpenTelemetry Collector in a local container." },
                                    plus: html! { "An admin and live telemetry come with the framework, and the traces use a standard format." },
                                    minus: html! { "Studio is a local development tool, not a production dashboard, and the admin is one more surface to protect with roles." },
                                },
                                Choice {
                                    chose: html! { "No admin UI documented. Logging with "<code>"tracing"</code>", "<code>"/_ping"</code>", "<code>"/_health"</code>" and "<code>"/_readiness"</code>" endpoints, and "<code>"cargo loco doctor"</code>"." },
                                    plus: html! { "Health endpoints a load balancer can use, and a command that checks your setup." },
                                    minus: html! { "No admin UI: you build or add one." },
                                },
                                Choice {
                                    chose: html! { "No admin framework (an admin demo app exists). Logging and a built-in health endpoint are not documented (its examples write a "<code>"/api/health"</code>" route by hand); OpenAPI generation arrived in 0.12.1." },
                                    plus: html! { "It can generate an OpenAPI description of your API." },
                                    minus: html! { "No admin, logging guidance or built-in health endpoint is documented." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-cli",
                            "CLI and generators",
                            html! { <strong>"In plain words: "</strong>"a command-line tool creates a project and writes repetitive code (models, controllers) so that you start from a working structure." },
                            [
                                Choice {
                                    chose: html! { <code>"cargo rullst new"</code>" (Blank, API and SaaS starters) and "<code>"make:*"</code>" generators for models, migrations, controllers, resources, workers, mail, billing, MFA, OAuth and more; "<code>"dev"</code>", "<code>"doctor"</code>" and assisted "<code>"upgrade"</code>"." },
                                    plus: html! { "Generated code is ordinary Rust you own, and "<code>"upgrade"</code>" helps you move between versions." },
                                    minus: html! { "Many commands to learn, and the generated code is yours to maintain." },
                                },
                                Choice {
                                    chose: html! { <code>"loco new"</code>" (server-rendered and client-rendered SaaS, REST API, lightweight and advanced templates) and "<code>"cargo loco generate"</code>" for models, migrations, scaffolds, controllers, tasks, workers, mailers and deployment files." },
                                    plus: html! { "Rails-style generators for most pieces, deployment files included." },
                                    minus: html! { "Generators run only in debug builds, and "<code>"loco new"</code>" has no template flag: it asks for one unless "<code>"--db"</code>", "<code>"--bg"</code>" and "<code>"--assets"</code>" are all given." },
                                },
                                Choice {
                                    chose: html! { <code>"topcoat new"</code>", "<code>"dev"</code>", "<code>"fmt"</code>", "<code>"ui"</code>" (copies components into the project), "<code>"asset bundle"</code>" and "<code>"l10n check"</code>". Model or controller generators are not documented." },
                                    plus: html! { "Components copied into your project are code you own, and translation files can be checked." },
                                    minus: html! { "You write models and controllers by hand." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-deployment",
                            "Deployment guidance",
                            html! { <strong>"In plain words: "</strong>"putting the application on a machine people can reach: a build, a server, HTTPS and a way to restart it." },
                            [
                                Choice {
                                    chose: html! { "Guides for DigitalOcean, Hetzner, Oracle Cloud and Google Cloud, and a generated Dockerfile, Compose and Caddy setup. CI tests the generated VM deployment of the Blank and SaaS starters, on SQLite, with Docker and Podman on an x86_64 runner only. The maintainer's production applications, Academy, capoeira and vene.win, run on one Oracle Ampere A1 Arm machine behind a shared Caddy with Let's Encrypt certificates; no CI test runs on Arm. The provider guides were written from the providers' documentation, not followed end to end on real accounts." },
                                    plus: html! { "A tested deployment path, and a real production setup on an Arm machine that CI does not cover." },
                                    minus: html! { "Arm machines, real domains and Let's Encrypt, provider accounts, reboots and PostgreSQL or MySQL deployments are not tested in CI, and the provider guides have not been tried on real accounts." },
                                },
                                Choice {
                                    chose: html! { "A release binary, plus generated Docker, nginx or AWS Lambda setups, and "<code>"loco doctor"</code>" before going live." },
                                    plus: html! { "Generated deployment files, and a command that checks the setup." },
                                    minus: html! { "Its CI builds the generated Docker image every night for a new SQLite application; none of its workflows runs that image or deploys the nginx or Lambda setups." },
                                },
                                Choice {
                                    chose: html! { "Deployment docs are on the roadmap. A Cloudflare Workers example, and custom hosting through "<code>"serve()"</code>" or "<code>"Router::handle"</code>"." },
                                    plus: html! { "An example runs it on Cloudflare Workers, and you can host it in your own way." },
                                    minus: html! { "No deployment guide yet." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-stability",
                            "Stability, versioning and support window",
                            html! { <strong>"In plain words: "</strong>"how much an upgrade may break your code, and for how long a version keeps receiving fixes." },
                            [
                                Choice {
                                    chose: html! { "13.0 is in prerelease; 13.0.0-alpha.6 froze its features and public API, and 13.0.0-rc.1, the first release candidate (published on 26 November 2026), adds fixes, documentation and the changes the maintainer approved one by one after the freeze: two of them break the API, and others change what a working alpha.6 deployment does, such as refusing empty settings in staging and production. After 13.0.0, the maintainer intends to keep 13.x the active line for 12 months, then give it security fixes for 1 to 3 more months after 14.0.0 (an intention, not a guarantee). Semantic Versioning; MSRV Rust 1.96." },
                                    plus: html! { "A stated support intention and Semantic Versioning, with a migration checklist from alpha.6 to rc.1." },
                                    minus: html! { "Not final yet, and the window is an intention, not a guarantee." },
                                },
                                Choice {
                                    chose: html! { "1.0.0 (July 2026) was its first stable release. Upgrade notes list breaking changes; a semver statement and a support window are not documented. MSRV Rust 1.94." },
                                    plus: html! { "A stable 1.x line with upgrade notes, and an older compiler requirement than the others." },
                                    minus: html! { "No support window is documented." },
                                },
                                Choice {
                                    chose: html! { "Its README says: early-stage and experimental, expect breaking changes. Its changelog says it follows Semantic Versioning and marks breaking changes. No support window documented. MSRV Rust 1.99." },
                                    plus: html! { "The changelog marks what breaks." },
                                    minus: html! { "Expect breaking changes, and it needs a very recent Rust compiler." },
                                },
                            ])}
                        {blocks::topic(Lang::En,
                            "topic-maturity",
                            "Maturity and community",
                            html! { <strong>"In plain words: "</strong>"a bigger community means more answers, examples and people fixing bugs; a smaller one means you may be on your own." },
                            [
                                Choice {
                                    chose: html! { "First published in May 2026. MIT license. 21 GitHub stars and 7 contributors on 27 November 2026; two of the contributors are AI coding agents (one of them a bot account) and one is Dependabot; Rullst has one maintainer." },
                                    plus: html! { "An MIT license and a maturity tier documented for every package." },
                                    minus: html! { "The smallest community of the three, one maintainer who is a single point of failure, and no external audit." },
                                },
                                Choice {
                                    chose: html! { "First release in November 2023. Apache-2.0 license. 9,675 GitHub stars and 157 contributors on 27 November 2026." },
                                    plus: html! { "The largest community of the three and the longest release history." },
                                    minus: html! { "Its documentation website lagged its newest release when checked (1.2.0 pages while 1.3.1 was out), so some features are documented only in the repository." },
                                },
                                Choice {
                                    chose: html! { "Repository created in April 2026, first functional release in July 2026, in the tokio-rs GitHub organization. MIT license. 7,331 GitHub stars and 41 contributors on 27 November 2026." },
                                    plus: html! { "Maintained inside the tokio-rs organization, with a growing contributor base." },
                                    minus: html! { "A young 0.x project." },
                                },
                            ])}
                    </section>
                    <section id="ui-frameworks" aria-labelledby="ui-frameworks-heading">
                        <h2 id="ui-frameworks-heading">"Why not Leptos or Dioxus?"</h2>
                        <p class="compare-checked" id="checked-ui">"Checked on "<time datetime="2026-11-27">"27 November 2026"</time>" against Leptos "<code>"leptos 0.8.23"</code>" and Dioxus "<code>"dioxus 0.7.10"</code>", their latest stable releases that day."</p>
                        <p>"Leptos and Dioxus are full-stack in a different sense: they are UI frameworks. The same Rust components can render HTML on the server and then run in the browser as WebAssembly, which hydrates that HTML, and server functions let the browser call Rust code that runs only on the server. Dioxus also targets desktop and mobile."</p>
                        <p>"As UI frameworks, neither ships an ORM, migrations or authentication, and neither documents mail, background jobs or an admin panel. The Leptos book says Leptos itself provides neither the HTTP server nor the database; the Dioxus documentation says Dioxus does not provide a built-in way of managing authentication, and its database tutorial uses "<code>"rusqlite"</code>" and names SQLx and SeaORM among the alternatives. Applications assemble these parts from other crates: the Leptos authentication example and the Dioxus authentication guide both use "<code>"axum_session_auth"</code>" with SQLx."</p>
                        <p>"They are not in the Performance section because it measures equivalent applications serving HTTP requests. The server side of a Leptos application runs on Actix Web or Axum through Leptos's integrations, and Dioxus Fullstack is built on Axum, which is already the benchmark's baseline; an equivalent application would also need its ORM and authentication assembled by hand. The approaches are complementary: a Leptos or Dioxus interface can call a Rullst HTTP API."</p>
                    </section>
                    <section id="benchmarks" aria-labelledby="benchmarks-heading">
                        <h2 id="benchmarks-heading">"Performance"</h2>
                        <p>"These numbers are not written by hand: each daily build of this site inserts the latest results of the "<a href="/Benchmarks/">"Rullst/Benchmarks"</a>" suite for the three frameworks, Actix Web and Gin, plus a minimal Rullst configuration. Rullst's maintainer also maintains that benchmark and wrote every implementation in it. Its results page says whether these results pass the benchmark's own publication gate."</p>
                        <div class="benchmark-howto">
                            <h3>"How to read these numbers"</h3>
                            <ul>
                                <li><strong>"1.00× is Axum."</strong>" Every score divides a framework's result by the result of Axum, a small Rust framework used as the reference, measured on the same machine in alternating rounds. 0.50× is half of Axum's result and 2.00× twice. Higher is better in every score: for memory and start-up time the ratio is turned around, so a higher score means less RAM or a faster start."</li>
                                <li><strong>"Ties and ±."</strong>" The machines are shared cloud machines, so results vary from run to run. \"±6%\" next to a score is the margin within which two scores cannot be told apart. Scores that close share a range of positions, such as \"3–7\": those implementations are in positions 3 to 7 in no particular order."</li>
                                <li><strong>"The categories answer different questions."</strong>" Hello world: what a framework costs per request when the page is trivial. Database: how it reads, writes and renders rows. Application: what it does with a real request (input, validation, a write, a form with CSRF protection, a signed-in user). Memory: how much RAM it needs under load. Startup: how fast it starts. Stability (pass or fail): whether it keeps working under heavier load."</li>
                                <li><strong>"The database usually comes first."</strong>" In a real application the database is usually the limit before the framework is, so a large gap in Hello world shrinks or disappears once requests read and write data; Database and Application are closer to what an application does."</li>
                                <li><strong>"The overall score is a summary."</strong>" It combines the five category scores, with Hello world, Database and Application counted twice and Memory and Startup once, for quick reading. The categories are the real result: other weights change the order, as the full results show."</li>
                                <li><strong>"Rullst appears twice."</strong>" \"Rullst\" is the application that "<code>"cargo rullst new"</code>" generates, with "<code>"Server"</code>"'s production stack on: the WAF, CSRF protection, secure cookies and secure headers (its access log, also on by default, is off in the benchmark, as per-request logging is for every implementation); it is the Rullst with a position. \"Rullst (minimal)\" is the same router, ORM and "<code>"html!"</code>" templates served by Axum's own server, without "<code>"Server"</code>", like the Axum app; it is a diagnostic, with scores and without a position. The gap between the two rows is roughly what that stack costs, "<code>"Server"</code>" itself included."</li>
                                <li><strong>"Database pools differ."</strong>" Since 13.0.0-rc.1, Rullst's pool pings a PostgreSQL connection before handing it out only when it has been idle for at least 500 ms. The Axum reference, Actix Web and Loco ping on every acquisition (SQLx's default, which SeaORM 2.0.4 under Loco keeps). Each Database score includes its pool setting: in the 27 November 2026 run, Axum without the ping scored 1.24× in Database ("<a href="/Benchmarks/diagnostics/#pool">"database-layer defaults"</a>")."</li>
                            </ul>
                        </div>
                        <div class="benchmark-results" data-benchmark-results>
                            {results}
                        </div>
                    </section>
                    <section id="sources" aria-labelledby="sources-heading">
                        <h2 id="sources-heading">"Sources"</h2>
                        <ul>
                            <li>"Rullst: "<a href="https://github.com/Rullst/Rullst#integrations">"README integrations"</a>", "<a href="/Rullst/book/maturity.html">"maturity tiers"</a>", "<a href="/Rullst/book/compatibility-policy.html#supported-release-window">"supported release window"</a>", "<a href="/Rullst/book/database-support.html">"database support"</a>", "<a href="https://crates.io/crates/rullst">"crates.io"</a>", "<a href="/Rullst/book/security-layers.html">"security layers"</a>" and "<a href="/Rullst/book/scope-and-limits.html#external-accounts-and-providers">"scope and limits"</a>"."</li>
                            <li>"Loco: "<a href="https://loco.rs/docs/">"documentation"</a>", "<a href="https://github.com/loco-rs/loco/tree/v1.3.1">"repository at v1.3.1"</a>", "<a href="https://crates.io/crates/loco-rs">"crates.io"</a>", the "<a href="https://loco.rs/docs/reference/middleware/">"middleware reference"</a>", "<a href="https://loco.rs/docs/how-to/jwt-auth/">"JWT authentication"</a>" and the "<a href="https://github.com/loco-rs/loco/blob/v1.3.1/Cargo.toml">"1.3.1 feature flags"</a>"."</li>
                            <li>"Topcoat: "<a href="https://github.com/tokio-rs/topcoat/tree/v0.12.1">"repository at v0.12.1"</a>" (README, "<code>"docs/"</code>", changelog), "<a href="https://docs.rs/topcoat/0.12.1">"docs.rs"</a>", "<a href="https://crates.io/crates/topcoat">"crates.io"</a>", the "<a href="https://github.com/tokio-rs/topcoat/blob/v0.12.1/docs/session.md">"session guide"</a>" and "<a href="https://github.com/tokio-rs/topcoat/blob/v0.12.1/docs/mail.md">"mail guide"</a>"."</li>
                            <li>"Leptos: "<a href="https://book.leptos.dev/">"the Leptos book"</a>" ("<a href="https://book.leptos.dev/ssr/22_life_cycle.html">"the life of a page load"</a>", "<a href="https://book.leptos.dev/server/25_server_functions.html">"server functions"</a>", "<a href="https://book.leptos.dev/server/26_extractors.html">"extractors"</a>"), "<a href="https://github.com/leptos-rs/leptos/tree/v0.8.23/projects/session_auth_axum">"authentication example at v0.8.23"</a>", "<a href="https://crates.io/crates/leptos">"crates.io"</a>"."</li>
                            <li>"Dioxus: "<a href="https://dioxuslabs.com/learn/0.7/">"documentation for 0.7"</a>" ("<a href="https://dioxuslabs.com/learn/0.7/guides/platforms/web/">"web"</a>", "<a href="https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/">"router and state"</a>", "<a href="https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/">"authentication"</a>", "<a href="https://dioxuslabs.com/learn/0.7/tutorial/databases/">"databases"</a>"), "<a href="https://crates.io/crates/dioxus">"crates.io"</a>"."</li>
                        </ul>
                    </section>
                </main>
                <footer class="shell page-footer">
                    {made_with::badge(Lang::En, cx.weights.page("/compare/").map(|weight| crate::footprint::render(weight, Lang::En)))}
                    <div class="footer-bottom"><a href="/">"Rullst home"</a><a href="mailto:officialrullst@gmail.com">"Contact the team"</a></div>
                </footer>
            </body>
        </html>
    }
}
