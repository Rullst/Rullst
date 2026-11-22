//! The English landing page (`/`): Included, Integrations, Examples, Academy, Community, the release notice and the footer.

use rullst::html;
use rullst::html::Markup;

use super::{Demo, art, demo_card, included_card};
use crate::footprint;
use crate::pages::made_with;
use crate::pages::{Context, Lang};

/// 03 / Included in Rullst.
pub fn included() -> Markup {
    html! {
        <section id="included" class="shell section included-section" aria-labelledby="included-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"03 / Included in Rullst"</p>
                <h2 id="included-heading">"Less to assemble."<br /><span class="accent-orange">"More you can inspect."</span></h2>
                <p>"Parts that ship with the framework, each documented with what it was tested against and what it does not do. Twenty packages share one version; the specialized ones are optional Extension packages."</p>
            </div>
            <p class="landing-checked" id="checked-included">"What a card says about Loco and Topcoat was checked on "<time datetime="2026-11-21">"21 November 2026"</time>" in Loco "<code>"loco-rs 1.3.1"</code>" and Topcoat "<code>"topcoat 0.12.1"</code>", their latest releases then."</p>
            <div class="included-grid">
                {included_card("CLI · RESOURCES", html! { <code>"cargo rullst footprint"</code> }, html! { "Measures your application's binary size, idle memory, start-up time, CPU time per request and page weight, says how it measured each one, and can fail CI when a budget grows. Its carbon figure is an SCI estimate from inputs you give, not a measurement." }, Some("Loco and Topcoat document no such command."), ("/Rullst/book/footprint.html", "Footprint guide"))}
                {included_card("CLI · UPGRADES", html! { <code>"cargo rullst upgrade"</code> }, html! { <code>"--dry-run"</code>" lists each source change a newer Rullst needs, at "<code>"file:line"</code>", with a rule code and its migration-guide row; applying backs up the files, updates the versions and runs "<code>"cargo fix"</code>" and "<code>"cargo check"</code>". It does not migrate data." }, Some("Loco documents an upgrade guide and no upgrade command; Topcoat documents neither."), ("/Rullst/book/tutorials/36-assisted-framework-upgrades.html", "Assisted upgrades"))}
                {included_card("CLI · TESTS", html! { "Tests from the first commit" }, html! { "Every project "<code>"cargo rullst new"</code>" creates has a smoke test of each page (status, security headers, HTML hygiene) and, unless you pass "<code>"--no-ci"</code>", a CI workflow with Clippy, tests, "<code>"cargo audit"</code>" and a secret scan. "<code>"cargo rullst add browser-tests"</code>" opens every page in Chromium, Firefox and WebKit with axe-core." }, None, ("/Rullst/book/testing-your-app.html", "Testing your application"))}
                {included_card("DEVELOPMENT · STUDIO", html! { "A developer control room" }, html! { "Studio runs as a local debug server on "<code>"127.0.0.1:5555"</code>": a database inspector with an ER diagram, queues, feature flags, request traces and the security events of the running process." }, Some("Loco and Topcoat document no development dashboard."), ("/Rullst/book/3-rullst-studio.html", "Studio guide"))}
                {included_card("ADMIN · NEXUS", html! { "A generated admin" }, html! { "Nexus builds server-rendered list, search, create, edit and batch views for the ORM models you register, with field policies and RBAC; release builds need your application's sign-in or Basic Auth over TLS." }, Some("Loco and Topcoat document no generated admin (Topcoat has a hand-written admin demo)."), ("/Rullst/book/4-rullst-nexus.html", "Nexus guide"))}
                {included_card("BILLING · CAPITAL", html! { "Payments with fail-closed checks" }, html! { "Stripe and InfinitePay adapters, signed webhooks with replay protection, quotas, entitlements and invoices in minor units. The generated billing code refuses empty payment keys in production." }, Some("Loco and Topcoat document no payments."), ("/Rullst/book/crates/capital.html", "Billing guide"))}
                {included_card("MAIL", html! { "Transactional mail" }, html! { "Resend, AWS SES, SendPulse or SMTP, sent in the background, with recipient suppression and opt-in attachment inspection. In 13.0.0-rc.1, a missing provider key in staging or production is an error, not a send that only looked successful." }, None, ("/Rullst/book/crates/mail.html", "Mail guide"))}
                {included_card("AI", html! { "One guarded AI client" }, html! { "Gemini, OpenAI, Claude and OpenAI-compatible endpoints (DeepSeek and Ollama presets), with a prompt-injection filter, PII masking and offline fixtures for tests." }, None, ("/Rullst/book/ai-provider-capabilities.html", "Provider boundaries"))}
                {included_card("IOT · SIGNED OTA", html! { "Firmware updates you can verify" }, html! { <code>"no_std"</code>" telemetry helpers, MQTT 5 and CoAP encoders and Ed25519-signed OTA manifest verification with a rollback-counter contract, run on a BBC micro:bit V2. Transports, flashing and bootloaders stay the application's." }, Some("Loco and Topcoat document no IoT or OTA support."), ("/Rullst/book/crates/iot.html", "IoT guide"))}
                {included_card("LABS", html! { "Graded exercises, your runner" }, html! { "Contracts for exercise jobs that a separately deployed runner claims: leases, cancellation, retention and signed receipts. Labs itself never runs learner code." }, None, ("/Rullst/book/labs-runner-contract.html", "Runner contract"))}
                {included_card("PRIVACY", html! { "Age policies and consent" }, html! { "Proportional age policies, signed threshold attestations and versioned consent and withdrawal, on SQLite or PostgreSQL; "<code>"make:age-gate"</code>" and "<code>"make:privacy"</code>" wire them into an application. No live age provider is included." }, None, ("https://github.com/Rullst/Rullst/tree/main/rullst-privacy", "rullst-privacy"))}
                {included_card("SUPERVISION", html! { "Transparent exam supervision" }, html! { "Exam and parental contracts with an explicit collection acknowledgement, typed browser observations and camera-presence and audio-activity adapters. No media model, automatic grading or device-wide control is included." }, None, ("/Rullst/book/supervision.html", "Supervision guide"))}
            </div>
            <p class="section-note">"Core packages are in every application; Extension packages are optional and carry the same 13.x stability promise for their documented scope ("<a href="/Rullst/book/maturity.html">"maturity tiers"</a>"). "<a href="/Rullst/book/scope-and-limits.html">"Scope and limits"</a>" lists what Rullst does not do and what its tests cannot establish."</p>
        </section>
    }
}

/// 04 / Integrations, by evidence (the entries of README's Integrations table).
pub fn integrations() -> Markup {
    html! {
        <section id="integrations" class="shell section integrations-section" aria-labelledby="integrations-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"04 / Integrations, by evidence"</p>
                <h2 id="integrations-heading">"How far each integration"<br /><span class="accent-green">"has been tested."</span></h2>
                <p>"Every built-in integration, grouped by the strongest evidence behind it. Names are plain text: listing a service implies no partnership or endorsement."</p>
            </div>
            <div class="integration-levels">
                <article class="integration-level" data-integration-level="real-accounts">
                    <span class="integration-index">"01 / REAL ACCOUNTS"</span>" "
                    <h3>"Used with real accounts"</h3>
                    <ul>
                        <li><b>"Stripe"</b>" (real payments)"</li>
                        <li><b>"InfinitePay"</b>" (a real Pix payment)"</li>
                        <li><b>"Bunny Stream"</b>", "<b>"Resend"</b>", "<b>"SMTP"</b>" mail, "<b>"Google Gemini"</b>", "<b>"Google"</b>" sign-in and "<b>"GitHub"</b>" sign-in (used by the maintainer's production applications, with no automated test against a live account)"</li>
                        <li><b>"PostgreSQL"</b>" and "<b>"SQLite"</b>" (in production in the maintainer's applications, besides CI)"</li>
                        <li>"The OpenAI-compatible AI adapter (validated with "<b>"Groq"</b>", an OpenAI-compatible provider, not with OpenAI)"</li>
                    </ul>" "
                    <a href="/Rullst/book/maturity.html#what-supported-promises">"What \"supported\" promises ↗"</a>
                </article>
                <article class="integration-level" data-integration-level="ci-containers">
                    <span class="integration-index">"02 / LOCAL CONTAINERS IN CI"</span>" "
                    <h3>"Tested in CI against servers in local containers"</h3>
                    <ul>
                        <li><b>"MySQL"</b>" and "<b>"MariaDB"</b></li>
                        <li><b>"Redis"</b>" (cache, queues, rate limits, ORM data structures and Redis Streams messaging)"</li>
                        <li><b>"SurrealDB"</b></li>
                        <li><b>"Turso/libSQL"</b>" (the "<code>"libsql-server"</code>" image, which is "<code>"sqld"</code>")"</li>
                        <li><b>"Meilisearch"</b>" search"</li>
                        <li><b>"pgvector"</b></li>
                        <li><b>"S3"</b>"-compatible storage (a "<b>"Garage"</b>" server)"</li>
                    </ul>" "
                    <a href="/Rullst/book/database-support.html#where-the-live-tests-run">"Where the live tests run ↗"</a>
                </article>
                <article class="integration-level" data-integration-level="offline">
                    <span class="integration-index">"03 / OFFLINE ONLY"</span>" "
                    <h3>"Offline only: mock mode, local fixtures or a conformance kit"</h3>
                    <ul>
                        <li><b>"AWS SES"</b>" and "<b>"SendPulse"</b>" mail"</li>
                        <li><b>"OpenAI"</b>" and "<b>"Anthropic Claude"</b>", and the "<b>"DeepSeek"</b>" and "<b>"Ollama"</b>" presets of the OpenAI-compatible adapter"</li>
                        <li><b>"Microsoft"</b>", "<b>"Apple"</b>", "<b>"Auth0"</b>", "<b>"AWS Cognito"</b>", "<b>"Facebook"</b>", "<b>"X (Twitter)"</b>", "<b>"Discord"</b>", "<b>"LinkedIn"</b>" and generic "<b>"OpenID Connect"</b>" sign-in"</li>
                        <li>"The "<b>"S3/R2"</b>" adapter of "<code>"rullst-media"</code></li>
                    </ul>" "
                    <a href="/Rullst/book/provider-conformance.html">"Provider conformance kits ↗"</a>
                </article>
            </div>
            <p class="integration-recipes" data-integration-recipes><strong>"Recipes, not integrations."</strong>" MongoDB, DuckDB and Qdrant are application recipes: your code uses their official crates directly. Their built-in adapters were removed in 13.0.0-alpha.5. Other services are application code too: write an adapter on the provider trait (payments, mail, video) or follow a recipe. "<a href="/Rullst/book/recipes/index.html">"Read the recipes ↗"</a></p>
            <p class="integration-production">"The maintainer's production applications Academy, capoeira and vene.win run on real domains on one Oracle Ampere A1 Arm machine behind a shared Caddy with Let's Encrypt certificates. CI tests the generated deployment on x86_64 only; Arm machines are not tested in CI. "<a href="/Rullst/book/scope-and-limits.html#external-accounts-and-providers">"Deployment limits "<span aria-hidden="true">"↗"</span></a></p>
        </section>
    }
}

/// 05 / The deployed examples.
pub fn examples() -> Markup {
    html! {
        <section id="examples" class="shell section examples-section" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"05 / Less imagining. More exploring."</p>
                <h2>"Not just a framework."<br /><span class="gradient-text">"A suite to bring your ideas to life."</span></h2>
                <p>"Explore four independently deployed applications on Azure Container Apps. Follow the source, discover a starting point, then make it yours."</p>
            </div>
            <p class="demo-startup-note"><strong>"Demo startup notice."</strong>" These applications may scale to zero when idle. The next request triggers a cold start while Azure Container Apps starts a container instance and initializes the application. This can temporarily increase the initial response time. Please allow a moment for the site to load."</p>
            <div class="demo-grid">
                {demo_card(&Demo { class: "demo-showcase", caption: "ROUTES / DATA / IDEAS", kicker: "01 / EXPLORE THE BUILDING BLOCKS", title: "Rullst Showcase", text: "A blog and application showcase. Explore server-rendered pages and selected integration demonstrations.", href: "https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/", button: "Explore Showcase", art: art::demo_showcase() })}
                {demo_card(&Demo { class: "demo-lms", caption: "LESSONS / QUIZZES / PROGRESS", kicker: "02 / MAKE ROOM FOR LEARNING", title: "LMS Academy", text: "Discover a learning-platform example, with a public course catalog and account entry points.", href: "https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/", button: "Explore the LMS", art: art::demo_lms() })}
                {demo_card(&Demo { class: "demo-portfolio", caption: "PROJECTS / SKILLS / YOUR STORY", kicker: "03 / PUT YOUR WORK OUT THERE", title: "Developer Portfolio", text: "A portfolio starting point for presenting projects, skills and experience in a focused web application.", href: "https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/", button: "Explore Portfolio", art: art::demo_portfolio() })}
                {demo_card(&Demo { class: "demo-saas", caption: "BLUEPRINT / PRODUCT / POSSIBILITIES", kicker: "04 / START YOUR NEXT PRODUCT", title: "SaaS Blueprint", text: "A SaaS application blueprint. Explore a starting point for building your own software-as-a-service product.", href: "https://saas.rullst.win/", button: "Explore SaaS", art: art::demo_saas() })}
            </div>
            <div class="demo-footnote">
                <p>"Public demos, not production certifications. Each demo is upgraded on its own schedule and can run an earlier Rullst release than 13.0.0-alpha.6. The SaaS checkout is live and charges real money; the Showcase payment fixtures are not live checkout. Apart from a purchase you intend to make, use test data, not sensitive information. Availability is managed separately."</p>
                <a href="https://github.com/Rullst/examples">"Source &amp; deployment recipes ↗"</a>
            </div>
            {made_with::card(Lang::En)}
        </section>
    }
}

/// Rullst Academy.
pub fn academy() -> Markup {
    html! {
        <section id="academy" class="shell academy-section" aria-labelledby="academy-heading" data-reveal>
            <div class="academy-spotlight">
                <div class="academy-content">
                    <p class="academy-label"><span class="academy-badge">"Rullst Academy"</span>" YOUR FREE TECHNOLOGY SCHOOL"</p>
                    <h2 id="academy-heading">"A world to discover."<br /><span>"One step at a time."</span></h2>
                    <p class="academy-copy">"Turn curiosity into something you can build. Learn Rust, create with Rullst and explore GitHub through short lessons, hands-on challenges and your own projects."</p>
                    <ul class="academy-subjects" aria-label="Explore subjects in the Academy">
                        <li>"Rust &amp; Rullst"</li>
                        <li>"GitHub"</li>
                        <li>"AI &amp; cybersecurity"</li>
                        <li>"Robotics &amp; math"</li>
                    </ul>
                    <div class="academy-actions"><a class="button button-primary" href="https://academy.rullst.win/">"Explore Rullst Academy "<span aria-hidden="true">"↗"</span></a>" "<a class="academy-path-link" href="https://academy.rullst.win/en/paths">"Find your learning path "<span aria-hidden="true">"↗"</span></a></div>
                    <p class="academy-note">"Português · English · Español "<span aria-hidden="true">"/"</span>" Try the first lesson of each path without an account."</p>
                </div>
                <div class="academy-companion">
                    <div class="academy-art" aria-hidden="true">
                        <div class="academy-orbit academy-orbit-outer"></div>
                        <div class="academy-orbit academy-orbit-inner"></div>" "
                        <span class="academy-spark academy-spark-one">"✦"</span>
                        <span class="academy-spark academy-spark-two">"+"</span>
                        <span class="academy-spark academy-spark-three">"✳"</span>" "
                        <span class="academy-art-label">"CURIOSITY, MEET YOUR COMPANION."</span>" "
                        <div class="academy-crab">
                            // Rullst Academy's own mascot, adapted from its public homepage.
                            {art::academy_crab()}
                        </div>" "
                        <span class="academy-code-token">"&lt;hello, explorer /&gt;"</span>" "
                        <span class="academy-discovery"><span>"✓"</span>" Small steps. New possibilities."</span>
                    </div>
                    <div class="academy-companion-caption"><span>"Your robot crab. Ready to explore."</span><button class="academy-wave-button" type="button" data-academy-wave aria-pressed="false" hidden>"Say hello "<span aria-hidden="true">"↗"</span></button></div>
                </div>
            </div>
        </section>
    }
}

/// Community.
pub fn community() -> Markup {
    html! {
        <section id="community" class="shell section" data-reveal>
            <div class="community">
                <div>
                    <p class="eyebrow">"Open source. Open invitation."</p>
                    <h2>"Your next project."<br />"Our next "<span class="accent-green">"breakthrough."</span></h2>
                    <p>"Build something. Question an assumption. Bring a reproduction. Improve a tutorial. You don't need a huge pull request to make a meaningful difference."</p>
                    <div class="hero-actions"><a class="button button-primary" href="https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md">"Make your first contribution ↗"</a><a class="button button-secondary" href="https://discord.com/invite/2ntKFtsSjw">"Meet the community ↗"</a></div>
                </div>
                <div class="community-mark" aria-hidden="true"><span>"{"</span><span>"you"<br />"+ rust"<br />"+ us"</span><span>"}"</span></div>
            </div>
        </section>
    }
}

/// The release notice.
pub fn release() -> Markup {
    html! {
        <section class="shell release-card" aria-label="Before you deploy" data-reveal>
            <h2>"Explore boldly."<br />"Deploy responsibly."</h2>
            <div>
                <p><strong>"v13"</strong>" is in prerelease: 13.0.0-alpha.6 is the 13.0 feature freeze, followed by 13.0.0-rc.1 (fixes and documentation, and the changes the maintainer approved one by one after the freeze, two of them breaking) and 13.0.0. Cargo never selects a prerelease on its own, so name the exact version from "<a href="https://crates.io/crates/rullst">"crates.io"</a>" ("<code>"=13.0.0-alpha.6"</code>") and use its matching immutable tag rather than the moving "<a href="https://github.com/Rullst/Rullst/tree/main">"main"</a>" branch. The v12 line reached end of life on 16 October 2026 with 12.3.0, and the "<a href="https://github.com/Rullst/Rullst/tree/v12">"v12 branch"</a>" is frozen; move applications with the "<a href="/Rullst/book/migration-v13.html">"v12 → v13 adoption guide"</a>". The legacy "<a href="https://github.com/Rullst/Rullst/tree/v5">"v5 branch"</a>" is no longer maintained either. "<a href="/Rullst/book/scope-and-limits.html">"Scope and limits"</a>" lists what Rullst does not do by design and what its own tests cannot establish. Prerelease and stable describe the framework API lifecycle, not automatic approval of an application's authorization, provider configuration, operations or deployment."</p>
                <a href="https://github.com/Rullst/Rullst/blob/main/RELEASE_GUIDE.md">"Check the release guide ↗"</a>
            </div>
        </section>
    }
}

/// The footer, with the privacy summary and the dedication.
pub fn footer(cx: &Context) -> Markup {
    let weight = cx
        .weights
        .of(Lang::En)
        .map(|weight| footprint::render(weight, Lang::En));
    html! {
        <footer class="footer">
            <div class="shell">
                <div class="footer-main">
                    <div>
                        <a class="brand" href="https://rullst.win/">"rullst"<span class="brand-period">"."</span></a>
                        <p>"Because With Rullst, We Rule!"</p>
                        <span class="footer-subtitle">"An open-source framework suite."<br />"Built in the open, with room for you."</span>
                    </div>
                    <div class="footer-group">
                        <h2>"Build"</h2>
                        <a href="https://academy.rullst.win/">"Rullst Academy"</a>
                        <a href="/Rullst/book/">"Documentation"</a>
                        <a href="/Rullst/book/philosophy.html">"Our story"</a>
                        <a href="/Rullst/benches/">"Benchmarks"</a>
                        <a href="https://github.com/Rullst/Rullst">"Source code"</a>
                        <a href="https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md">"Contribute"</a>
                    </div>
                    <div class="footer-group">
                        <h2>"Connect"</h2>
                        <a href="mailto:officialrullst@gmail.com">"Contact the team"</a>
                        <a href="https://github.com/Rullst/Rullst/security/policy">"Report a vulnerability"</a>
                        <a href="./privacy.html">"Privacy notice"</a>
                        <a href="/Rullst/book/compatibility-policy.html#source-branches">"Release status"</a>
                    </div>
                </div>
                <nav class="social-links" aria-label="Rullst social and community links"><a href="https://rullst.github.io">"Website ↗"</a>" "<a href="https://github.com/Rullst">"GitHub ↗"</a>" "<a href="https://rullst.blogspot.com">"Blogspot ↗"</a>" "<a href="https://www.bilibili.tv/en/space/1436672033">"BiliBili ↗"</a>" "<a href="https://bsky.app/profile/rullst.bsky.social">"Bluesky ↗"</a>" "<a href="https://daily.dev/squads/rullst">"Daily.dev ↗"</a>" "<a href="https://dev.to/venelouis">"Dev.to ↗"</a>" "<a href="https://discord.com/invite/2ntKFtsSjw">"Discord ↗"</a>" "<a href="https://rullst.hashnode.dev/">"Hashnode ↗"</a>" "<a href="https://www.instagram.com/officialrullst/">"Instagram ↗"</a>" "<a href="https://www.linkedin.com/company/rullst">"LinkedIn ↗"</a>" "<a href="https://www.reddit.com/r/rullst">"Reddit ↗"</a>" "<a href="https://substack.com/@rullst">"Substack ↗"</a>" "<a href="https://t.me/rullst">"Telegram ↗"</a>" "<a href="https://www.tiktok.com/@venelouis">"TikTok ↗"</a>" "<a href="https://www.youtube.com/@officialrullst">"YouTube ↗"</a>" "<a href="https://x.com/venelouis">"X ↗"</a></nav>
                <section id="privacy" class="privacy-notice" aria-labelledby="privacy-heading">
                    <h2 id="privacy-heading">"A small footprint. A clear privacy notice."</h2>
                    <p>"This landing page has no analytics, advertising pixels or embedded social feeds. Its code does not set cookies or use browser storage. Copy buttons write only the displayed command to your clipboard, only when clicked."</p>
                    <details>
                        <summary>"Hosting, linked pages &amp; your privacy rights"</summary>
                        <div>
                            <p>"The site is hosted on GitHub Pages. Requests necessarily reach the host, which may process IP addresses and technical request information under the "<a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement">"GitHub Privacy Statement"</a>". We do not control GitHub's hosting logs or their retention."</p>
                            <p>"Social links are ordinary links, not embeds. Following them opens a service with its own privacy practices. Rullst Academy, documentation, benchmarks and the Showcase, LMS, Portfolio and SaaS demos have separate data flows. The documentation can store display preferences locally and include external resources; benchmark pages currently load Chart.js from jsDelivr. The no-tracking description above applies to this landing page, not to every linked service."</p>
                            <p><strong>"Responsible for this website:"</strong>" Venelouis, Brazil. For questions or requests to access, correct or delete personal data, contact "<a href="mailto:officialrullst@gmail.com">"officialrullst@gmail.com"</a>". If you email that address, the team and its email provider receive your address and message to handle your inquiry. Email is handled through Gmail; please don't include credentials or unnecessary personal data. Public issues and social posts are not private support channels. Your rights depend on the law that applies to your situation."</p>
                            <p><a href="./privacy.html">"Read the full privacy notice"</a>" for purposes, legal bases, retention, international processing and regional rights, including LGPD and GDPR where applicable. This notice describes the website—not data processing in applications built with Rullst—and does not claim universal legal compliance. Last updated: November 14, 2026."</p>
                        </div>
                    </details>
                </section>
                {made_with::line(Lang::En, weight)}
                <div class="footer-bottom"><span>"Built with care. Shared in the open."</span><a href="#main">"Back to top ↑"</a></div>
            </div>
            <p class="glory">"All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ)."</p>
        </footer>
    }
}
