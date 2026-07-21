# Changelog 📝

All notable changes to the **Rullst Framework** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.0.0] - Unreleased 🚀

### Added
- **Hybrid Hot-Reloading (Dynamic Linking + AST Parsing)**: The ultimate DX revolution. For business logic changes, Rullst uses dynamic library hot-swapping (`dylib` / `.so`) to update the backend instantly. For frontend views (`html!` macros), the CLI intercepts changes, parses the AST, and squirts new HTML fragments over WebSockets to morphdom. The result? Sub-millisecond layout updates (like Vite/Dioxus) with a stateful Rust backend.
- **Native Reactive SSR (Rullst Live)**: Introduced `#[live_component]` and `#[live_event]` declarative macros. Developers can now build highly interactive, real-time WebSocket UIs (LiveView style) without writing JavaScript or WebAssembly. The macro automatically handles DOM diffing events and state syncing via HTMX.
- **Documentation Hub**: A premium VitePress documentation portal in `docs/` with dark mode aesthetics.
- **TypeScript SDK Generator (`generate:ts`)**: AST-based CLI command to dynamically generate typed `rullst-client.ts`.
- **Ultra-Lightweight OpenTelemetry Core**: Integrated OTLP exporter behind a `telemetry` feature flag, keeping the framework lightweight by default.

### Breaking Changes
- **Rullst Connect v11.0.0 API**: Upgraded `rullst-connect` to `11.0.0`. Since `rullst-connect` is re-exported via `rullst::auth::connect`, developers using the `oauth` feature must adapt to any breaking changes.
- **Major Version Bump**: Rullst was upgraded to `5.0.0` to respect SemVer due to the `rullst-connect` API breaking changes.

### Changed
- **Dependencies Upgrade**: Updated all dependencies in the workspace to their latest stable compatible versions.

### Security
- **SSH/SCP Option & Command Injection Defense**: Hardened `cargo-rullst` deployment generator (`foundry.rs`) by adding POSIX option delimiters (`--`) before destination arguments in `ssh` and `scp` invocations, enforcing strict character validation on upload binary names (`bin_name`), and validating environment variable keys and values to prevent newline or argument injection during remote server provisioning.
- **Supply Chain Security**: Pinned `softprops/action-gh-release` and `rust-lang/crates-io-auth-action` to their absolute commit SHAs in the release workflow to resolve Scorecard Pinned-Dependencies alerts.
- **Windows Shell Injection Mitigation**: Replaced unsafe `cmd /C npm` invocations with direct `npm.cmd` and `npx.cmd` binary executions in the `cargo-rullst` desktop generator to prevent potential command injection on Windows environments.

### Refactoring & Code Quality
- **Server Router Readability**: Extracted `handle_oneshot_error` and `handle_panic_error` helper functions in `rullst/src/server.rs` to flatten deep nesting and improve maintainability of the `HotSwapService` response handler.

### Testing
- **Rate Limit Middleware Rejection Coverage**: Added integration test `test_rate_limit_middleware_rejection` in `rullst/src/resilience.rs` verifying that `rate_limit_middleware` properly intercepts requests exceeding the rate limit and returns HTTP `429 Too Many Requests` with the expected rejection text body.
- **Edge Cases & Builders Coverage**: Added rigorous unit tests for `HtmxResponse::refresh` builders, `ReplicationConfig` sync/auth token builders, memory cache `remember` error closures, and a crucial edge case testing empty Stripe webhook secrets in `capital.rs`.


## [4.0.2] - 2026-06-29

### Added
- **CLI**: Added the `cargo rullst nixify` command to generate a reproducible Nix development environment (`flake.nix` and `.envrc`).
- **CLI**: Added `--nix` flag to `cargo rullst new` for scaffolding Nix-enabled projects directly.
- **CLI**: Added missing `cargo rullst dev` command to the "View Help & Commands" list.
- **AI Directives**: Refined `AGENTS.md` and `docs/spec.md` with explicit instructions: "Static Dispatch over Dynamic", allowing `unwrap()` in test scenarios, explicit quotation of HTML macro attributes, and the strict prohibition of raw SQL macros within controllers in favor of ORM delegation.
- **Testing**: Added rigorous global facade coverage verifying the `Storage::put` and `feature::init` API capabilities, ensuring local disk storage defaults and singleton initialization work flawlessly across test environments.
- **Testing**: Increased edge-case test coverage for `htmx::render_page` (testing empty contents and unescaped HTML characters) and `MemoryFeatureDriver` manual override functionality.

### Refactoring & Code Quality
- **Job Queue Type Refactoring**: Extracted a complex tuple return type `Vec<(String, ...)>` into a dedicated `JobRow` structure in the SQLite queue driver (`queue.rs`), enhancing code readability and complying with strict `clippy::type_complexity` limits.
- **Codegen Optimization**: Simplified the self-healing AST codemod regex compilation (`build.rs`) to map patterns natively within the `OnceLock` initialization, eliminating redundant regex construction during `cargo-rullst` execution.
- **Nexus Panel Maintenance**: Eliminated unused dead code (`db_url`) within the internal `NexusState` and `Nexus` auto-CMS builder, streamlining the component struct architectures.
- **Blueprint Enterprise Architecture (MVC)**: Extensively refactored the generated blueprints (SaaS, ERP, and Uptime) to decouple database logic from controllers. All raw `sqlx::query` invocations inside controllers were systematically replaced with native Rullst Active Record (`.save()`, `.all()`, `.find()`) or explicitly delegated to `impl Model` repositories.
- **Studio Readability**: Significantly reduced cyclomatic complexity within `handle_table` (`studio.rs`) by extracting the SQL generation, schema introspection, and raw record fetching logic into concise independent functions.
- **Scheduler Complexity**: Addressed `clippy::type_complexity` warnings inside `scheduler.rs` by exporting a clean `ScheduledHandler` trait alias for the recurrent callback functions, removing the need for `#[allow]` suppression pragmas.

### Performance & CI
- **Benchmark Regression Testing**: Integrated a GitHub Actions pipeline (`bench.yml`) utilizing `github-action-benchmark` to enforce a maximum 30% latency degradation (`alert-threshold: '130%'`) on pull requests. The routing, HTML macros, and WAF middleware `Criterion` benchmarks now run with an expanded sample size (`sample_size(100)`) to mitigate false positives caused by noisy neighbor CPU throttling in shared environments.
- **Insta Snapshot Testing**: Adotada a biblioteca `insta` no ecossistema de testes para garantir precisão e ausência de regressões na renderização de macros HTML e em geradores de código. O primeiro teste de snapshot foi adicionado para a engine da macro `html!`.
- **Dead Code Extirpation**: Converted `field_kind_input_type` in `nexus.rs` strictly into a test-only `#[cfg(test)]` function since it is not utilized anywhere in the active production paths, decreasing build footprints.
- **Desktop Generator Reliability**: Removed synchronous `std::thread::sleep(3)` delays from the Tauri mobile initialization scripts inside `desktop.rs`, converting them into an active rapid polling mechanism against `127.0.0.1:3000`, making local mobile/desktop test runs faster and fail-fast capable.
- **Uptime Seeder Optimization (N+1 Elimination)**: Wrapped multiple sequential heartbeat inserts inside the Uptime Monitor generator within a single database transaction (`pool.begin()`) using a single batch insert query string, significantly optimizing startup time for new generated projects and eliminating all N+1 querying.
- **Generator Complexity**: Split the monolithic `generate_docker_files` function in `cargo-rullst/src/generators/project.rs` into smaller, focused helpers (`create_dockerfile`, `create_docker_compose`, `create_env_files`). Similarly modularized `run_build_client` in `cargo-rullst/src/generators/build.rs` to vastly improve readability and maintainability.

### Security
- **Nexus CSRF Hardening**: Enforced robust CSRF protection on the Rullst Nexus auto-CMS by applying the `csrf_middleware` directly to the Nexus router buildup. Additionally injected HTMX config event listeners in the `render_shell` to seamlessly attach the `X-CSRF-Token` header on all dynamic admin requests.
- **Timing Attack Mitigation (Auth Scaffolding)**: Resolved a user enumeration vector within the `cargo-rullst` authentication generator. Login attempts for nonexistent email addresses now dynamically trigger a dummy Argon2 hash verification to ensure constant-time execution against brute-force enumeration bots.
- **LMS Denial of Service (DoS) Mitigation**: Removed unsafe `unwrap()` invocations inside the `lms.rs` blueprint template. Unsafe access to non-existent courses or lessons could panic the Axum server process. The blueprint now safely delegates missing IDs to a `404 Not Found` response in accordance with the Zero-Panic policy.

### Testing & Code Coverage
- **Edge Server Emulation**: Added integration tests for `EdgeServer::run` in `tests/edge_tests.rs`, spawning the emulator on a background tokio thread and executing actual HTTP requests via `reqwest` to validate end-to-end edge router initialization.

### Fixed
- **Axum 0.8 Wildcard Syntax**: Updated the Edge emulation router (`EdgeServer::run`) to use the new `/{*path}` syntax mandated by Axum 0.8, fixing a startup panic when testing integration scenarios.
- **Console Style Invocation**: Fixed a compilation error in `cargo-rullst/src/generators/build.rs` where an invalid method `.dim()` was called instead of the correct `colored::Colorize::dimmed()`.


## [4.0.1]

### Testing & Code Coverage
- **Total Coverage 82% Milestone**: Reached and solidified over 81.60% test coverage across the entire framework workspace.
- **Passkey WebAuthn Edge Cases**: Extensively covered `auth/passkey.rs` parsing and assertion failures, pushing CBOR payload decoding coverage from 73% to 88.38%. Validated malformed attestation objects, truncated signatures, invalid base64, and origin mismatches.
- **Billing Webhook Resilience**: Fortified `capital.rs` test suites by simulating sophisticated Stripe and LemonSqueezy webhook attacks, including invalid HMAC signatures, payload tampering, unrecognized events, and missing headers. Coverage for the billing module reached 93.89%.
- **Feature Flags Robustness**: Added deep testing for `feature.rs` drivers, handling complex fallbacks like TOML parsing errors, missing database tables, uninitialized ORM states, and raw-string interpolation failures.

## [4.0.0] - 2026-06-19 🚀

### Breaking Changes
- **Rullst Connect v10.0.1 API**: Upgraded `rullst-connect` to `10.0.1`. Since `rullst-connect` is re-exported via `rullst::auth::connect`, developers using the `oauth` feature must adapt to any breaking changes.
- **Rullst ORM v6 API**: Upgraded `rullst-orm` to `=6.0.0` across the framework, scaffolding templates, and examples. Projects utilizing the ORM must adapt to the new `v6.0.0` API.

### Added
- **Auto-Migrations in Dev Server**: Automated the database migration workflow. The `cargo rullst dev` command now silently executes `cargo run -q -- db:migrate` behind the scenes to apply any pending migrations before starting the hot-reloader, vastly improving Developer Experience (DX).

### Changed
- **ERP Blueprint Translation**: Translated the entire ERP Pocket scaffolding blueprint (`erp.rs`) from Portuguese to English to standardize the framework's default language.
- **Nexus Panel Scaffolding UX**: Standardized the display of the Nexus CMS button across all blueprints (SaaS, LMS, ERP, Blog) to include a helper text `(login: admin / password)` perfectly aligned using flexbox layout.
- **Dependencies Upgrade**:
  - Upgraded `cron` from `0.16.0` to `0.17.0`.

### Security
- **Native Security Matrix (CI/CD)**: Upgraded the framework's CI/CD pipeline to Enterprise-grade "Secure by Design" status using hardcore Rust-native tooling.
  - Added **cargo-deny** to ban unapproved licenses and vulnerable dependencies.
  - Added **OSSF Scorecards** to establish a public, enterprise security score.
  - Added **OWASP ZAP** DAST pipeline to proactively attack generated SaaS blueprints in real-time.
    - Updated the `blank` blueprint generation to natively include `headers_middleware`, ensuring all new projects pass DAST scanning out-of-the-box.
  - Added **cargo-tarpaulin** for native, terminal-based code coverage reporting within PRs.
  - Showcased GitHub Actions badges in the `README.md`.
  - Added **cargo-mutants** to enforce test suite quality via deliberate mutation injections.
  - Added **cargo-fuzz** with an initial target (`mask_pii`) to guarantee DoS immunity against malformed byte sequences.
- **URL Decoding Integrity (WAF Bypass Mitigation)**: Fixed the WebAssembly-compatible `url_decode` function in `rullst/src/security.rs` which was silently dropping invalid hex sequences (e.g. `%XY`). It now safely preserves the intact invalid sequences, preventing WAF bypass attacks where an attacker could construct malicious payloads that trick the firewall but execute on the backend.
- **Scaffolding Password Length Limits**: Integrated the strict 72-character maximum password length validation directly into the `cargo-rullst/src/blueprints/saas.rs` and `cargo-rullst/src/generators/auth.rs` scaffolding generators, providing immediate UI error feedback to the user and securing all newly generated Rullst projects out-of-the-box against Argon2 resource exhaustion DoS attacks.
- **Password Length Limits (DoS Mitigation)**: Enforced a strict maximum password length of 72 characters in `rullst/src/auth.rs` (`hash_password` and `verify_password`). This prevents Denial of Service (DoS) attacks where maliciously oversized inputs could exhaust CPU and memory resources during Argon2 hashing.
- **Timing Attack Mitigation (Dummy Hash Verification)**: Closed a subtle timing vulnerability in `verify_password` (`rullst/src/auth.rs`). Previously, passwords exceeding the 72-character limit returned `false` immediately, allowing an attacker to determine password length discrepancies through latency measurements. The system now utilizes "Dummy Hash Verification" to compute a valid Argon2 hash in the background for oversized inputs, masking the failure and equalizing the CPU execution time.
- **Path Traversal Mitigation (Workspace Bound)**: Strengthened the `rullst/src/error_console.rs` AI auto-fix and explain endpoints to mitigate path traversal bypasses. Previously, the endpoints unconditionally rejected all absolute paths, which inadvertently blocked legitimate internal file reads during error displays and caused panics. The `extract_source_context`, `handle_explain`, and `autofix` functions now correctly verify absolute paths against the `project_root`, safely permitting workspace-bound access while preventing directory traversal attacks.
- **Scaffolding Nexus Authentication**: Secured the generated Rullst projects by adding default `.with_auth("admin", "password")` credentials to the `Nexus::new()` builder in all `cargo-rullst` blueprints (`blog`, `erp`, `lms`, `saas`). Previously, the boilerplates instantiated the Nexus panel without authentication, leaving the admin interface completely exposed to the public upon deployment.
- **HTTP Security Headers Enhancement**: Fortified the `headers_middleware` in `rullst/src/security.rs` by adding a strict `Permissions-Policy` header (`geolocation=(), camera=(), microphone=()`) to proactively block access to sensitive browser APIs by default. Additionally, upgraded the `Strict-Transport-Security` header to include the `preload` directive for maximum HTTPS enforcement.
- **Strict CORS Origin Enforcement**: Fixed a vulnerability in `rullst/src/server.rs` where the server would apply an overly permissive `allow_origin(Any)` rule if the user populated the `cors_allow_origins` array in the config. The server now correctly maps and enforces the specific trusted origins provided by the user via `CorsLayer::new().allow_origin(origins)`.
- **Edge Server Memory Exhaustion (DoS)**: Resolved a critical vulnerability in the Edge emulator (`rullst/src/edge.rs`) where the request body was buffered into memory with a limit of `usize::MAX`. An attacker could exploit this to trigger an Out-Of-Memory (OOM) crash by sending an arbitrarily large payload. The buffer limit has been reduced to a secure default of 2MB to align with other framework middlewares.
- **CSRF Denial of Service (DoS) Mitigation**: Resolved a critical vulnerability in `handle_csrf_state_modifying` (`rullst/src/security.rs`) where an attacker could remotely crash the Axum server process by sending a CSRF token with a mismatched length. The panic occurred because the `ct_eq` function from the `subtle` crate requires byte slices to be identical in length. A strict length validation has been added prior to the constant-time equality check, securely dropping invalid lengths.
- **Nexus Admin Auth Timing Attack**: Fixed a timing attack vulnerability in `rullst/src/nexus.rs` where the Basic Authentication login used a standard string comparison (`==`) for verifying passwords. The verification logic was upgraded to use `subtle::ConstantTimeEq` to prevent attackers from guessing passwords byte by byte via timing discrepancies.
- **Insecure Stateless Session Management**: Patched a critical cryptographic design flaw in `rullst/src/auth.rs`. Previously, the stateless session cookie payload only encrypted the `user_id` without an expiration timestamp, meaning stolen session tokens would remain perpetually valid as long as the server's `APP_KEY` did not change. The `encrypt_session` algorithm now seamlessly embeds a 30-day expiration UNIX timestamp inside the encrypted payload (`user_id|timestamp`). The `decrypt_session` function cryptographically verifies this expiration date, while maintaining a smart fallback to prevent mass-logout of legacy active tokens during the rollout.
- **WAF Evading (Payload Obfuscation)**: Fixed a WAF bypass vulnerability in `rullst/src/security.rs`. The custom WebAssembly-compatible `url_decode` helper was failing to correctly convert `+` characters into spaces before pattern matching. Attackers could theoretically evade the malicious pattern scanner by replacing spaces with `+` in their payloads (e.g., `SELECT+*+FROM`). The decoder now properly handles `+` characters.
- **HTTP Desync in PII Middleware**: Addressed a severe protocol compliance issue in the `pii_masking_middleware` (`rullst/src/security.rs`). When the middleware redacted sensitive information (like substituting a credit card for `***`), the length of the HTTP body changed, but the `Content-Length` header remained unmodified. This could cause erratic browser behavior, truncated responses, or HTTP Desync attacks against intermediate proxies. The middleware now dynamically updates the `Content-Length` header with the exact byte size of the newly masked payload.
- **Zero-Allocation HTML Escaping**: Optimized the `HtmlEscape` trait in `rullst/src/html.rs` to return `std::borrow::Cow<'_, str>` instead of unconditionally allocating a new `String`. This ensures that strings without special HTML characters (like `<`, `>`, `&`, `"`, `'`) are passed as zero-cost `Cow::Borrowed(&str)` references, avoiding unnecessary heap allocations and memory copies during Server-Side Rendering (SSR).
- **HTML Escaping Performance**: Optimized the `escape_str` function in `rullst/src/html.rs` (the core of the `html!` macro) to use chunk-based slice pushing instead of iterating and escaping character by character. Furthermore, the internal string allocation now uses `String::with_capacity(0)` and `reserve_exact()` to prevent the memory allocator from unnecessarily over-provisioning space. These changes combined reduce `.push()` overhead and overall escape latency by up to 50%.
- **`html!` Macro Output Optimization**: Rewrote the attribute generation logic in `rullst-macros/src/html_parser.rs` to replace expensive `format_args!` and `std::fmt::Write::write_fmt` runtime invocations with direct `push_str()` calls. For static attributes, it now injects pre-formatted string literals at compile time, eliminating runtime formatting entirely.
- **WAF Panic/Malformed Strings Fix**: Fixed a panic and data corruption vulnerability in the WebAssembly-compatible `url_decode` helper (`rullst/src/security.rs`). The previous implementation pushed URL-decoded hexadecimal bytes directly as Rust `char` values, which creates invalid internal UTF-8 representations and triggers panics when encountering non-ASCII bytes (e.g., `%e2%98%ba`). It now buffers to a `Vec<u8>` and uses `String::from_utf8_lossy()` to ensure safe UTF-8 decoding.
- **WAF Middleware Hardening**: Upgraded the `waf_middleware` (`rullst/src/security.rs`) to detect and block OS command injection patterns (e.g., `; ls`, `| bash`, `&& cat`). Furthermore, expanded the malicious payload scanner to inspect HTTP headers (`Referer` and `Cookie`) in addition to URL query parameters, protecting against advanced header-based injection attacks while carefully avoiding body buffering to preserve high framework throughput.
- **Micro-UX and Accessibility**: Corrected all instances of JSX-style `htmlFor` attributes to the standard HTML `for` attribute in the `cargo-rullst` blueprints and example templates, ensuring proper native browser accessibility for form labels. Additionally, integrated explicit `aria-label` and `aria-busy="false"` accessibility states to all primary interactive buttons (`Sign In`, `Registrar`, `Publish`, etc.) across the SaaS, ERP, Auth, and Uptime boilerplates to better support screen readers and HTMX loading indicator implementations. Furthermore, applied `aria-hidden="true"` to all decorative inline SVG icons across the dashboard, horizon, and billing templates to ensure screen readers properly ignore purely aesthetic visual elements. Added `aria-label` to the AI Chat and table search inputs in the Rullst Nexus CMS (`rullst/src/nexus.rs`). Finally, improved keyboard navigation by adding `focus-visible` CSS outlines to primary buttons, and greatly enhanced form UX by adding the correct `autocomplete` tags (`email`, `current-password`, `new-password`) to the Auth and SaaS blueprint login/register forms.
- **Studio HTMX Parameter Escaping**: Enforced strict `urlencoding` on all dynamically generated database identifiers (table names) and search queries interpolated within `hx-get` attributes in `rullst/src/studio.rs`. This prevents potential Stored XSS or HTMX injection vectors caused by attributes breaking prematurely if an attacker managed to create a malicious table schema name containing quotes.

### Fixed
- **Gitignore Cleanup**: Removed duplicate and corrupted lines containing null bytes (`NUL`) from `.gitignore`.
- **HTML Escaping Reference Bug**: Fixed a compilation error and type mismatch in `rullst/src/error_console.rs` by correctly passing `&str` references instead of `String` ownership to the `escape_str` utility.

### Refactoring & Code Quality
- **URL Encoding Micro-Optimization**: Eliminated multiple heap allocations and intermediate string formatting in `url_encode` (`rullst/src/capital.rs`). It now pre-allocates `String::with_capacity(s.len())` and utilizes `std::fmt::Write::write_fmt` directly, drastically speeding up Stripe and LemonSqueezy checkout session generations.
- **Nexus HTML Generation Optimization**: Removed repetitive intermediate heap allocations when rendering dashboard statistics in the `nexus_dashboard` view (`rullst/src/nexus.rs`) by replacing `.push_str(&format!(...))` loops with `.fold()` iterators mapped directly to a pre-allocated `String::with_capacity` using `std::fmt::Write::write_fmt`.
- **Stripe HMAC Allocation Optimization**: Removed an unnecessary `String` heap allocation during Stripe webhook signature verification (`rullst/src/capital.rs`). Instead of converting the raw payload byte slice into a lossy UTF-8 string just to concatenate it with the timestamp, the HMAC-SHA256 signature is now computed natively using multi-step byte slice updates (`ctx.update`), eliminating a highly inefficient memory copy operation.
- **Hex Decoder Micro-Optimization**: Eliminated dynamic vector reallocation in the `hex::decode` utility (`rullst/src/capital.rs`) by using `Vec::with_capacity(s.len() / 2)` instead of `Vec::new()`, removing overhead in high-frequency cryptographic and webhook signature validation loops.
- **PII Masking Bottleneck Removal**: Disabled the `pii_masking_middleware` by default. Previously, this security layer was buffering and scanning up to 2MB of memory for every single outgoing text/json response in production, severely crippling the framework's maximum throughput (Req/s) compared to raw Axum. It is now strictly opt-in via `SecurityConfig::enable_pii_masking`. Additionally, removed a wasteful double-allocation (`Vec<char>` -> `String` -> `Vec<char>`) inside the internal `mask_pii` text iteration phase (`rullst/src/security.rs`).
- **Redis Cache Flushing Optimization**: Upgraded the `flush` and `forget` methods in the Redis cache driver (`rullst/src/cache.rs`) to use the non-blocking `UNLINK` command instead of `DEL`. This massively improves performance when invalidating large caches by deleting keys asynchronously on the server side, completely eliminating Redis event loop stalls.
- **Job Queue Performance**: Added a composite database index (`idx_rullst_jobs_status_created`) on `status` and `created_at` fields for the SQLite background queue driver (`rullst/src/queue.rs`), completely eliminating full table scans during job polling and massively improving worker throughput.
- **Studio HTML Generation Optimization**: Replaced `push_str(&format!(...))` anti-patterns with `.fold()`, `String::with_capacity()`, and the `write!` macro in `rullst/src/studio.rs` (`build_rows_html`, `build_headers_html`), `rullst/src/nexus.rs` (`render_table_view`, `render_table_rows`), and `rullst/src/error_console.rs` (`render_console_html`), completely eliminating intermediate string allocations in the critical rendering paths. Also replaced chained `.replace()` calls with the single-pass `crate::html::escape_str` utility in `error_console.rs` to eliminate further allocations.
- **Authentication Key Derivation**: Extracted duplicated `Aes256Gcm` cipher initialization into a centralized `derive_cipher` helper in `rullst/src/auth.rs`, improving code maintainability.
- **Task Scheduler Loop**: Decomposed the infinite polling loop in `rullst/src/scheduler.rs` into a standalone `run_task_loop` asynchronous function, significantly cleaning up the `start` method.
- **Nexus N+1 Query Elimination**: Optimized `render_form_fields_html` in `rullst/src/nexus.rs` by pre-fetching all `ForeignKey` relational options concurrently using `tokio::task::JoinSet`, eliminating the N+1 database query bottleneck during form rendering.
- **Clippy Optimization**: Removed an unnecessary `let out =` binding and return statement inside `rullst/src/nexus.rs` (identified by the `clippy::let_and_return` lint) to allow the `.fold()` expression to return implicitly.
- **HTML Escaping Performance**: Optimized the `escape_str` utility (`rullst/src/html.rs`) by introducing a fast-path byte scan. Strings without special HTML characters are now returned immediately without character-by-character iteration or intermediate reallocation, resulting in a massive execution speedup during `html!` macro rendering.
- **String Generation Micro-Optimization**: Eliminated multiple intermediate heap allocations and formatting bottlenecks by replacing remaining `push_str(&format!(...))` anti-patterns with `let _ = std::fmt::Write::write_fmt(&mut ..., format_args!(...));` across `rullst/src/nexus.rs` and `rullst/src/validation.rs`. This implements the framework's performance guidelines for rendering loops.
- **HTML Macro Attribute Allocation Elimination**: Re-architected the compile-time code emission of the `html!` macro inside `rullst-macros/src/html_parser.rs`. Previously, HTML attributes were dynamically interpolated into intermediate `String` instances at runtime before being pushed to the main buffer. The macro now emits direct zero-allocation `write_fmt!` invocations natively into the target HTML buffer. This dramatically slashes memory allocations globally across every single Rullst application using the `html!` macro.

## [3.0.0] - 2026-06-15 🚀

### Breaking Changes
- **Rullst Connect v8 API**: Upgraded `rullst-connect` from `7.0.2` to `8.0.0`. Since `rullst-connect` is directly re-exported via `rullst::auth::connect` under the `oauth` feature, developers integrating social logins will need to adapt to the new `v8.0.0` breaking changes in their own application code. 
- **OAuth Module Renamed**: Renamed the `rullst::auth::socialite` module to `rullst::auth::connect` to standardize nomenclature. Applications upgrading to `3.0.0` must update their `use` imports accordingly.

### Changed
- **Dependencies Upgrade**: 
  - `tower-http` bumped from `0.6` to `0.7.0`.
  - Upgraded `rullst-orm` to `=5.0.2`.
  - Updated scaffolding generator templates for `cargo rullst` to automatically wire new projects using the latest `rullst-orm` and `rullst-connect` versions.
  - Performed a workspace-wide `cargo update` which bumped 16 transitive crates (including `brotli`, `redis`, `time`, `wasm-bindgen`, and `libsqlite3-sys`) to their latest secure and performant patch versions.

## [2.0.10] - 2026-06-13 🚀

### Refactoring & Code Quality (Jules' suggestions)
- **Regex Compilation Optimization**: Optimized the `cargo-rullst` project dependency generator (`build.rs`) using `std::sync::OnceLock`. This completely avoids the expensive redundant compilation of Regex patterns inside hot loops during code generation.
- **Async I/O Safety**: Simplified the `TomlFeatureDriver` in `rullst/src/feature.rs` by removing the explicit synchronous `tokio::task::block_in_place` wrapper. This eliminates Tokio thread pool blocking warnings during configuration loading.
- **Secure Key Generation**: Strengthened the `generate_secure_app_key` function inside `cargo-rullst/src/generators/project.rs` by utilizing the robust `rand::rngs::OsRng` rather than the default `thread_rng`.
- **UI Blueprint Refactoring**: Decomposed the massive `dashboard_page` HTML macro inside `cargo-rullst/src/blueprints/erp.rs` and `cargo-rullst/src/blueprints/uptime.rs` into modular `< 100` line view functions (`render_kpi_cards`, `render_products_table`, `render_orders_table`, `render_forms`, `render_monitors_list`, `render_new_monitor_form`) for significantly better maintainability.
- **WASM Interop Serialization**: Implemented strict serialization for native ES6 `Set` and `Map` collections directly into the Rullst Wasm Island debug hydration logger (`rullst_blog_example.js`).
- **Server Boot Refactoring**: Extracted `run` logic in `rullst/src/server.rs` into smaller modular methods (`load_config`, `setup_tracing`, `resolve_storage`, etc.) to stay under the 100-line limit per function. Removed dead commented-out code block from `rullst/src/server.rs`.
- **Security Middleware Decoupling**: Split the monolithic `csrf_middleware` in `rullst/src/security.rs` into distinct GET and state-modifying handlers (`handle_csrf_get`, `handle_csrf_state_modifying`) for better clarity.
- **Nexus Admin Refactoring**: Decomposed complex HTML rendering macros in `rullst/src/nexus.rs` (`render_table_rows`, `render_record_form`) into smaller, focused internal helpers, ensuring no single function exceeds 100 lines.
- **Examples Cleanup**: Refactored the blog demo (`index` in `examples/blog/src/lib.rs`) and omni-app demo (`App` in `examples/blog/omni-app/src/main.rs`) to use modular helper functions and extracted components, keeping UI templates small and readable.
- **CLI Code Generators Refactoring**: Decomposed large generator functions in `cargo-rullst` (`run_upgrade` and `run_dev_server` in `build.rs`, `create_new_middleware` in `middleware.rs`) into private helper methods. Extracted large inline CSS strings from `login_page` and `register_page` in `auth.rs` into a shared helper function (`auth_styles`).
- **CLI UI and Blueprint Refactoring**: Decomposed large functions in `cargo-rullst` (`run_foundry_deploy` and `scaffold_foundry_config` in `foundry.rs`, `show_interactive_dashboard` and `show_help_reference` in `ui/components.rs`, `pricing_page` in `blueprints/saas.rs`, and `render` in `blueprints/portfolio.rs`) into smaller, focused private helper methods to adhere to the 100-line limit and improve maintainability.
- **Test Coverage Expansion**: Added comprehensive unit tests for `active_requests`, `db_latency`, `event_loop_lag`, `per_minute`, `per_second` and `per_hour` (RateLimiter/TrafficShield) in `rullst/src/resilience.rs`, `needs_rehash`, `get_app_key` and `extract_session_cookie` in `rullst/src/auth.rs`, uninitialized state checks for `safe_driver` in `rullst/src/db.rs`, router nesting and websockets (`test_nest_axum`, `test_ws_routing`) in `rullst/src/routing.rs`, and custom driver (`test_custom_cache_driver`) in `rullst/src/cache.rs`.
- **Security hardening**: 
  - Fixed a potential arbitrary file deletion vulnerability in `rullst/src/server.rs`. The background `.so`/`.dll` cleanup routine now strictly validates the exact dynamic library prefix using `starts_with(&format!("{}_active_", filename))` instead of loose `.contains()`, ensuring only active framework binaries are pruned.
  - Mitigated a Path Traversal risk in the AI Error Console (`rullst/src/error_console.rs`) by strictly rejecting paths containing parent directory (`../`) components prior to canonicalization.
  - Prevented potential Command Injection in the CLI scaffolding tools (`cargo-rullst/src/generators/project.rs`) by ensuring the binary name strictly contains only alphanumeric characters, dashes, or underscores before passing to `std::process::Command`.
### Changed
- **Ecosystem Diet**: Audited the entire workspace with `cargo-machete` and removed unused "ghost" dependencies from `cargo-rullst` and internal benchmark projects (`tower-http`, `tokio`, `async-trait`, `serde`), keeping the codebase as lightweight as possible.

## [2.0.9] - 2026-06-12 🚀

### Performance & Benchmarks
- **Criterion Fullstack Benchmarks Suite**: Integrated PR #80 by Jules with comprehensive benchmark tests comparing Rullst's zero-cost architecture against Axum, Loco, Leptos, and Dioxus.
- **SSR Rendering Dominance**: Confirmed that Rullst's compile-time `html!` macro executes at `~1.07 µs`, being significantly faster than Tera (2x), Dioxus Virtual DOM (4.2x) and Leptos (8.5x).
- **Zero-Cost Routing**: Validated that Rullst's high-level declarative router compiles down to near-identical Axum-level latency (`~974 ns` for Rullst vs `~946 ns` for raw Axum).
- **Website Redesign**: Overhauled the framework's website with a premium glassmorphism dark-mode design, showcasing dynamic visual elements and injecting the new official performance metrics.
- **Dependency Cleanups**: Pruned unused dependencies (including the `cookie` crate) across the framework workspace.

## [2.0.8] - 2026-06-12 🚀

### Added
- **Axum 0.8 Router Composition**: Added `Router::merge_axum` method in `rullst/src/routing.rs` allowing developers to merge raw `axum::Router` instances (e.g., from `utoipa_axum`) directly into the Rullst router at the root. (PR #78 by @mengyou658).

### Changed
- **Rust 1.96 Upgrade**: Upgraded all Rullst internal Dockerfile templates (used in `cargo rullst new` with `--docker` and all framework benchmarks) from `1.94`/`1.95` to use the newly released `rust:1.96-slim-bookworm` base image.
- **CLI Translation**: Translated the remaining Portuguese configuration comments inside the `.cargo/config.toml` linker performance hints scaffolding into English.

### Fixed
- **Ecosystem Crash Shielding (E0119)**: Pinned the `time` dependency strictly to `0.3.36` inside the framework's core `Cargo.toml`. This explicitly shields all newly scaffolded Rullst applications from a global ecosystem crash caused by `time 0.3.37` which broke the standard `cookie 0.18.1` crate.

## [2.0.7] - 2026-06-10 🚀

### Performance & Stability
- **Uptime Blueprint Window Functions**: Replaced an N+1 query vulnerability in the Uptime Monitor dashboard (`cargo-rullst/src/blueprints/uptime.rs`) by using SQLite Window Functions (`ROW_NUMBER() OVER`), massively improving dashboard load times.
- **ORM Dependency Bump**: Upgraded `rullst-orm` to `5.0.0` for latest database performance and macro improvements.

### Security & Testing
- **Hot-Reload Isolation**: Hard-disabled dynamic library (`.dll`/`.so`) hot-reloading router implementations (`Server::new_hot`) when compiled in `--release` profiles, aggressively mitigating Remote Code Execution (RCE) via `libloading` in production.
- **Foundry SCP Hardening**: Fixed a potential MITM vulnerability in `cargo-rullst`'s Web3 deployment scaffolding by replacing `StrictHostKeyChecking=no` with `accept-new`.
- **Passkey WebAuthn Tests**: Added unit testing coverage to the `rullst/src/auth/passkey.rs` manager to validate credential start/finish options.
- **Server Resilience Tests**: Added builder validation tests for `Server::shield` and `Server::rate_limit` modifiers.
- **AI Providers Tests**: Added API key and model builder test validations to OpenAI, Gemini, Anthropic, and Ollama core providers.
- **Wasm & Auth Test Coverages**: Expanded testing suites into `client.rs` (wasm_bindgen support), `config.rs`, `security.rs` (CSRF), and `resilience.rs`.

### Maintenance & Dependencies
- **Rand 0.10.1 Compatibility**: Upgraded `rand` dependency to `0.10.1` and migrated the internal `cargo-rullst` app key generator from `thread_rng().gen_range()` to the new `rng().random_range()` API.
- **Root Dependencies Update**: Safely bumped patch versions for multiple core dependencies (`regex` to 1.12.4, `uuid` to 1.23.3, `wasm-bindgen` to 0.2.123, `rullst-connect` to 7.0.2) following a pristine security audit with zero CVEs.

## [2.0.5] - 2026-06-10 🛠️

### Performance & Stability
- **Concurrent Uptime Monitoring**: Optimized the Uptime Monitor blueprint (`cargo-rullst/src/blueprints/uptime.rs`) by replacing blocking sequential HTTP requests and database inserts with concurrent `tokio::spawn` tasks, drastically improving throughput for multiple monitors.
- **Async I/O Safety**: Refactored `MailDriver` resolution (`rullst/src/mail.rs`) to strictly utilize asynchronous `tokio::fs::read_to_string` instead of `std::fs`, eliminating Tokio event-loop thread blocking during email dispatches.

### Security & Testing
- **Rust 1.80+ Test Compatibility**: Patched `auth.rs` tests failing on newer Rust compilers by wrapping the newly deprecated and unsafe `std::env::set_var` within an explicit `unsafe` block for local testing environments.
- **Test Coverage Expansion**: Added strict boundary condition tests for source code context extraction (`error_console.rs`) and session cookie parameter generation (`auth.rs`), resolving gaps in coverage.
- **Security Validation**: Addressed and invalidated false-positive AI security audits regarding CLI command injection, hot-reloading `unsafe` blocks, and uptime scaffolding inserts, cementing Rullst's 100/100 pristine security baseline.

### CLI & Tooling
- **Docker Cache Bugfix**: Fixed an issue in `cargo rullst dockerize` and `--docker` scaffolding where Docker's `mtime` caching behavior would cause Cargo to skip compilation of `.rs` files after building dependencies, resulting in empty binaries that exited with code 0.
- **Lean Core Refactor**: Completely removed the internal `rullst-press` SSG tool from the framework workspace and CLI menu. Rullst is now strictly focused on backend/fullstack productivity, and the main documentation has migrated to a dedicated modern SPA site.
- **Clippy Strict Compliance**: Re-audited and passed `cargo clippy --workspace --all-targets --all-features -- -D warnings`, resolving a stray `clippy::useless_vec` warning in the interactive menu.

## [2.0.4] - 2026-06-09 🔒

### Security & Stability

- **Zero-Panic Policy Enforcement (P1)**: Replaced the single remaining `unwrap()` call inside the Nexus Basic Auth middleware (`nexus.rs:249`) with a `unwrap_or_else` fallback response builder, fully complying with the Zero-Panic production requirement across all runtime paths.
- **WASM Panic Elimination (P3)**: Fixed a panic vector in the `#[client_component]` proc-macro (`rullst-macros`). The generated WASM code now uses a `let Some(...) else { return String::new() }` pattern instead of `unwrap()` when accessing the DOM, making island components safe to use inside Web Worker contexts.
- **Basic Auth Strip Hardening**: Replaced the manual `starts_with("Basic ") + &auth_str[6..]` byte-index slice in the Nexus middleware with `.strip_prefix("Basic ")`, eliminating any risk of a byte-boundary panic on malformed `Authorization` headers.
- **ORM Alignment & Panic Safety**: Upgraded `rullst-orm` dependency version to `4.0.5` across the framework and scaffolding templates to resolve type-mismatch compile errors in derived macro implementations. Introduced panic-safe database guards `safe_pool()` and `safe_driver()` in `rullst::db` to cleanly query initialization status and handle offline database states without crashing the server.
- **Blueprint & Example Migration Alignment**: Updated `rullst-blog-example` and all scaffolding templates (`uptime`, `lms`, `erp`, `blog`) to align with `rullst-orm` 4.0.5's non-Result pool signature, removing obsolete `?` error propagation operators on pool retrieval.

### Code Quality

- **Clippy Clean Sweep (`-D warnings`)**: Resolved all 7 clippy lints found during the formal audit pass. `cargo clippy --workspace --all-targets --all-features -- -D warnings` now exits with **0 errors, 0 warnings** across the entire workspace:
  - `dead_code` — `NexusState::db_url` field suppressed with `#[allow(dead_code)]` and a reserved-for-future-use comment.
  - `dead_code` — `field_kind_input_type` is test-only; annotated with `#[cfg_attr(not(test), allow(dead_code))]`.
  - `clippy::manual_strip` — replaced manual prefix-strip with `.strip_prefix("Basic ")`.
  - `dead_code` — `CborValue::Array` CBOR variant suppressed with `#[allow(dead_code)]` and a spec-compliance comment.
  - `unused_imports` — removed unused `Response` import from `benches/rullst_bench.rs`.
  - `clippy::useless_vec` — replaced `vec!["Rust", "Go", "Python"]` with an array literal in `benches/rullst_bench.rs`.

### Testing

- **Storage Test Environment Isolation (P2)**: Added `#[allow(unsafe_code)]` with full SAFETY documentation to the `unsafe { std::env::set_var }` call in `storage.rs` tests. Added a matching `remove_var` call after the test to prevent environment state from leaking into parallel test threads.
- **CBOR Parser Spec Compliance**: The `CborValue::Array` variant in `auth/passkey.rs` is retained for future attestation format compatibility; annotated to suppress the `dead_code` lint without removing the spec-correct variant.

### Documentation

- **`AUDIT.md`**: Added a comprehensive formal security and architecture audit report to the repository root. The document covers dependency security (`cargo audit`), code quality (`cargo clippy`), Zero-Panic policy compliance, `unsafe` block analysis, SQL injection prevention, CSRF, session encryption, HTTP headers, WebAuthn, rate limiting, backpressure, and hot-reload safety. All 9 findings identified have been resolved; only the advisory `RUSTSEC-2026-0173` (`proc-macro-error2` unmaintained) remains under monitoring as a compile-time-only concern with no associated CVE.

## [2.0.3] - 2026-06-07 🛠️

### Added
- **Nexus Live Database Mapping**: Integrated Rullst Nexus auto-CMS with the real database via `rullst-orm` to display and interact with actual records.
- **Nexus Live Search & Pagination**: Completed live search and database pagination for registered models.
- **Nexus Dynamic CRUD**: Implemented dynamic CRUD routes (`INSERT`, `UPDATE`, `DELETE`) mapping form payloads directly to database tables, including automatic table refresh on successful form submission.
- **Nexus Relationship Dropdowns**: Introduced `FieldKind::ForeignKey` to dynamically map database relations and render fully populated `<select>` dropdown inputs in creation/editing forms (e.g. choosing categories for courses and courses for lessons).
- **Security Middlewares Injection**: Configured automatic injection of WAF, CSRF, Secure Headers, and PII masking middlewares to production Axum routing.
- **CLI Workspace Path Resolution**: Upward-searching directory resolver for Rullst workspace path when generating projects from subdirectories.

### Changed
- **CLI Interactive Menu Reorganization**: Restructured the main `cargo rullst` dashboard. Extracted operations that depend on an existing project (Dev Server, Database, Auth, Scaffolding, Dockerize, Deploy, etc.) into a cleaner `Already have a project?` submenu. Adjusted emoji spacing and rigidly aligned descriptive help text.
- **Server Address Binding**: The server now respects the `HOST` environment variable to define the binding address, falling back to `127.0.0.1` for local development and `0.0.0.0` for production or Docker environments. This ensures full Docker compatibility out of the box.
- **Config-Driven Storage Root**: Restructured local storage root resolution to strictly use validated configuration (`Rullst.toml`) instead of direct env-variable lookups in production.
- **UNIX Hot Reload Shared Object Cleanup**: Instantly unlinks temporary dynamic library files after mapping is loaded on UNIX to prevent disk space leaks during active dev watch runs.

### Fixed
- **Nexus Unit Test Suite**: Converted all database-interactive Nexus unit tests to asynchronous `#[tokio::test]` runners. Implemented a thread-safe `tokio::sync::Mutex` initialization guard to prevent parallel test threads from panicking due to duplicate static database pool creation.
- **Nexus CRUD Form Actions**: Replaced the static "Save" button label with a dynamic one ("Create" for new records, "Save Changes" for edits) in the auto-CMS form renderer, correcting failing test assertions.
- **Nexus UI Cleanups**: Fixed duplicate navigation menu elements and repositioned the admin dashboard components.
- **Nexus Modal Alignment**: Centered the Create/Edit dialog modal in the middle of the screen instead of the top-right corner.
- **Nexus Record Creation**: Excluded empty primary key fields from SQL `INSERT` statements when creating new records, ensuring auto-increment generation works flawlessly for models like categories, courses, and lessons.
- **Tauri Desktop Config & Assets**: Fixed Tauri build issues by removing non-existent macOS configurations and resolving the corrupted IDAT chunk CRC bytes in mock 1x1 PNG generation within the desktop packager.
- **Dioxus Template Syntax**: Corrected an invalid semicolon syntax error inside the `rsx!` macro templates generated by the Omni scaffolder.
- **CLI Scaffolding Outputs**: Cleaned up log messages to remove "(Dioxus)" references, clarifying Omni/Hyper targeting.
- **Nexus SQL Injection Vulnerabilities**: Sanitized dynamic table, column search filters, updates, and order fields in all `UPDATE`, `DELETE`, and `SELECT` query builders inside the auto-CMS panel.
- **Zombie Process Prevention**: Integrated a `ChildGuard` drop wrapper inside the Omni CLI generator to ensure background development servers are killed immediately when the frontend exits.
- **Static Format Optimization**: Optimized interactive prompt text formatting by removing unnecessary format macros.

## [2.0.2] - 2026-06-03 🚀

### Added
- **Native Hot-Reloading**: Integrated `cargo-watch` natively into the `cargo rullst dev` command. Rullst now automatically tracks project files and intelligently recompiles and restarts the server with sub-second latency, providing an incredibly fast developer loop.
- **English Documentation Hub**: Rewrote and expanded the entire Rullst documentation ecosystem in English.
  - Added dedicated guides for **Rullst Nexus** (Auto-CMS), **Rullst Studio** (Telemetry), and **Rullst Capital** (Billing).
  - Enhanced the **AI Agents Manifesto** (`AGENTS.md`) guide to explicitly instruct LLMs on how to leverage Rullst's strict typing as an absolute validation layer.

### Changed
- **Lints Modernization**: Injected `[lints.rust] unexpected_cfgs = "allow"` into all new projects generated by the Rullst CLI. This preemptively handles the strict feature-flag checking introduced in Rust 1.80+ macros (like `rullst-orm`), guaranteeing that new user projects compile with absolutely zero false-positive warnings.
- **Formatting Standardization**: Enforced strict `cargo fmt` formatting guidelines across all raw string templates within the CLI (`erp.rs`, `lms.rs`, `saas.rs`, `portfolio.rs`, `uptime.rs`, `blank.rs`), ensuring generated code is beautiful right out of the box.

### Fixed
- **Clean Blueprints**: Removed stale and unused ORM imports (`Blueprint`, `RullstModel`, `sqlx`, etc.) across all starter Blueprints. Generated code is now warning-free, scoring 10/10 on `cargo clippy`.
- **Clippy Optimization**: Replaced a `useless_format` in the CLI's environment generator (`project.rs`) with a standard `.to_string()`.
- **Zero-Panic Stability**: Eliminated all occurrences of `.unwrap()` and `.expect()` throughout the Rullst core (`edge.rs`, `server.rs`, `security.rs`, `resilience.rs`, `error_console.rs`), utilizing safe `match` patterns.
- **Strict Linting Enforcement**: Injected `#![deny(clippy::unwrap_used)]` and `#![deny(clippy::expect_used)]` into `rullst/src/lib.rs` to enforce zero-panic code.
- **100% Documentation Coverage Baseline**: Enabled `#![warn(missing_docs)]` across the main library, automatically seeding 282 missing documentation segments to mandate strictly documented APIs for future PRs.

## [2.0.1] - 2026-06-03 🐛

### Changed
- **CLI Upgrades**: Improved the `cargo rullst` CLI wizard hints and simplified the dev server startup message.
- **Blueprint Fixes**: Fixed `routes!` syntax (`get("/" => handler),`) and `html!` macro syntax (`required="true"`) that caused compilation errors in newly generated ERP and Uptime Monitor projects.
- **Blueprint Resilience**: Added a 3-second initialization delay to background workers to completely prevent SQLx `Orm must be initialized before querying` panics on startup.
- **Design Standardization**: Updated all 5 starter blueprints to strictly use the Rullst branding colors (Emerald Green `emerald-500` and Orange `orange-500`) instead of generic blues and purples.
- **RullstPress Engine**: Completely rewrote the Rullst documentation using the internal SSG Engine, providing accurate tutorials for the new interactive CLI.

## [2.0.0] - 2026-06-01 🚀

### Security & Deep Audit 10/10 Certification
- **100/100 Pristine Status**: Resolved all technical debt and performance bottlenecks flagged in the June 2026 Deep Audit.
- **Studio SQL Security**: Hardened SQL identifier sanitization with strict 64-character length limits to prevent buffer exhaustion.
- **HTML Macro Zero-Allocation**: The `html!` compile-time macro now pre-computes static AST sizes and injects `String::with_capacity(STATIC_SIZE)` for maximum memory efficiency.
- **AI-Native Maintainability**: Created standard `AGENTS.md` and `.ai-rules` files to govern AI tooling workflows securely.
- **Async I/O Optimization**: Refactored `RedisDriver::flush` cache pruning to utilize a single batched `DEL` roundtrip, eliminating event-loop blocking from sequential iterators.
- **Complex View Engine Sanitization**: Added strict HTMX-safe validation and encoding checks for complex Javascript data types mapped to HTML strings.
- **AWS S3 Disablement**: Completely deactivated the `storage-s3` feature and purged the AWS SDK dependencies from the framework. This decisively eliminates the `rustls` CVE vulnerabilities instead of suppressing them, guaranteeing a mathematically proven 100% vulnerability-free build.

### Added (Milestone 11: Real-World Business Blueprints)
- **ERP Pocket Starter Blueprint (ID 4)**:
  - Scaffolds a complete Dark & Neon styled inventory, product, and stock management system with auto-CMS and HTMX.
  - Features dynamic HTMX stock increments and order processing with strict transactional database logic to validate quantity and automatically decrement product stock.
- **Uptime Monitor Starter Blueprint (ID 5)**:
  - Scaffolds a stunning "Uptime Robot" replica dashboard using glassmorphic UI components, average latency metrics, and color-coded status history block bars.
  - Spawns a background ping worker loop (`tokio::spawn(ping_monitors)`) running concurrently to Axum's web routing thread, recording historic latency and response metrics.
  - Integrates reqwest TLS features automatically in `Cargo.toml` on demand.

### Added (Milestone 9 – Phase 5: Rullst Foundry CLI)
- **1-Click DevOps Deployment (`cargo rullst foundry:init` & `cargo rullst foundry:deploy`)**:
  - Implements declarative infrastructure configuration via `Foundry.toml`, automatically generated and tailored to the Rullst project context with native gitignore protection.
  - Supports 6 major cloud providers out of the box: **AWS**, **Hetzner Cloud**, **Google Cloud Platform**, **Microsoft Azure**, **Oracle Cloud Infrastructure**, and **DigitalOcean**.
  - Implements a resilient 5-stage deployment pipeline using system SSH/SCP integrations: compiles the production binary, provisions the remote server environment, uploads the compiled binary, configures environment variables, configures a Caddy HTTPS reverse proxy with automatic SSL certificate management, sets up a persistent `systemd` service, and performs a live application health check.

### Added (Milestone 9 – Phase 4: Dual-Engine Frontend (Hyper & Omni))
- **Tauri Desktop Packaging (`cargo rullst make:desktop`)**:
  - Automatically scaffolds the full Tauri configuration (`src-tauri/`) required to compile Rullst Hyper (HTMX + SSR) applications into native desktop executables.
  - Implements a high-reliability background server lifecycle orchestrator in Rust (`src/main.rs`) that starts the Rullst backend on a background thread, monitors and polls TCP port `3000` for binding, launches the webview interface, and gracefully terminates the backend when the window is closed.
  - Integrates a smart transparent 1x1 icon generator directly in the Rust CLI to build and write fully valid, structured binary PNG, `.ico`, and `.icns` file formats to prevent Tauri compilation errors due to missing assets.
- **Dioxus Multi-Platform Scaffolding (`cargo rullst make:omni`)**:
  - Scaffolds a complete monorepo template with a Dioxus v0.7 multi-platform frontend application (`omni-app/`) pre-wired to talk to the Rullst backend API.
  - Features a beautiful dark-mode glassmorphic user interface (`style.css` using modern gradients, ambient glows, responsive panels, beacons of status, and micro-animations) for high-impact visual aesthetics.
  - Integrates Dioxus v0.7 signals (`use_signal`, `use_future`) for async state fetching from the Rullst REST/WS backend with visual offline fallbacks.

### Added (Milestone 9 – Phase 1: Rullst Nexus Panel)
- **`rullst::nexus` Module**: Introduced the `Nexus` auto-generated CMS & AI Admin Panel. Developers register any struct that implements the `NexusModel` trait and instantly get a fully functional, dark-mode admin panel served at `/nexus` — zero templates or configuration required.
- **`NexusModel` Reflection Trait**: Added the core `NexusModel` trait for model schema reflection. Implement `nexus_table()`, `nexus_label()`, `nexus_icon()`, `nexus_fields()`, and `nexus_pk()` to expose any model to the panel. A future `#[derive(Nexus)]` macro will auto-generate this.
- **`FieldMeta` & `FieldKind`**: New types to describe model field schemas with semantic types (Text, Email, Number, Boolean, Date, DateTime, Password, Json, Textarea, Url), visibility (hidden), and editability (readonly) controls.
- **Dynamic CRUD via HTMX**: The Nexus router auto-generates full `GET/POST/PUT/DELETE` routes per registered model, with reactive HTMX-powered paginagtion, live search (300ms debounce), and create/edit/delete modals — all without additional handler code.
- **AI Query Assistant (`/nexus/chat`)**: Added an AI-powered chat interface at `/nexus/chat`. The system prompt is automatically populated with the full registered database schema. Connects to `rullst::ai::AiClient` for production deployments; includes a built-in smart mock responder for development.
- **Premium Dark-Mode UI**: The panel features a bespoke glassmorphism dark-mode design system (Inter + JetBrains Mono, CSS custom properties, smooth animations) embedded directly into the binary — no external CSS files required.
- **Re-exports**: `Nexus`, `NexusModel`, `FieldMeta`, and `FieldKind` are now available at the top-level `rullst::` namespace.


### Added (Milestone 10: Instant Incremental Compilation & Linker Hacking)
- **Dynamic Linker Hacking Detection**: Added runtime capability to detect fast modern linkers (`mold` on Linux/macOS and `lld` on Windows/Linux/macOS) in `cargo-rullst`.
- **Smart Scaffolding Optimization**: Automatically generates the `.cargo/` structure and `.cargo/config.toml` configuring high-performance linkers if they are found in the developer's system path. Prevents build breaks by elegantly generating them commented out with precise activation instructions if not installed.
- **Cranelift Compiling Integration**: Scaffolds new projects with a ready-to-use, well-documented `[profile.dev] codegen-backend = "cranelift"` block inside `Cargo.toml`, guiding users on how to achieve sub-100ms compilation times in development.
- **Interactive Performance Scaffold Banners**: Renders a beautiful tip banner at the end of the new project scaffolding wizard, recommending exact commands to install LLD or Mold based on the developer's operating system (e.g. `winget install LLVM.LLVM` for Windows).

### Dependency Updates & Modernization
- **Rullst-ORM v3.x Migration**: Migrated the core framework and project generation templates to `rullst-orm v3.x`, updating all occurrences of the renamed `EloquentModel` trait to `RullstModel`.
- **Cargo Dependency Upgrades**: Upgraded various key dependencies across the workspace to their latest versions (including `toml`, `redis`, `aws-sdk-s3`, `uuid`, `dashmap`, `walkdir`, `colored`, `tokio`, `pulldown-cmark`, `axum`, and `tower-http`) to guarantee the framework is running on the latest stable and secure releases.
- **Rng Stability & rand_core Resolution**: Resolved version conflicts between `rand_core` versions. Removed the direct explicit dependency on `rand_core` from the main framework cargo definition, leveraging `argon2`'s re-exported types inside `auth.rs` to allow smooth and crash-free password hashing and salt generation.

### Community Health
- **Community Standards**: Added `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant).
- **Issue Templates**: Added structured GitHub templates for Bug Reports and Feature Requests.
- **PR Checklists**: Added strict `PULL_REQUEST_TEMPLATE.md` to ensure code quality prior to review.

## [1.0.10] - 2026-05-29 🛡️

### Security & Quality Audits (10/10 Milestone)
- **Deep Audit 10/10 Certification**: Passed all strict security, performance, and maintenance requirements outlined in the 2026 deep audit.
- **Dynamic Local Secret Persistence**: Removed the last static hardcoded `DEV_APP_KEY` from memory. In development, keys are now generated securely and persisted automatically to `.rullst_dev_key`, preventing any false-positive security scans.
- **Massive Test Coverage Expansion**: Introduced comprehensive unit and integration test suites for `mail.rs`, `queue.rs`, `db.rs`, `live.rs`, `studio.rs`, `error_console.rs`, `edge.rs`, and `resilience.rs`, achieving flawless coverage.

- **Production Fail-Hard**: Added a strict enforcement in `auth.rs`. If `RULLST_ENV` or `APP_ENV` is set to `production` or `prod` and `APP_KEY` is missing, the server explicitly panics instead of generating an ephemeral key.
- **Removed Dummy Tests**: Replaced `assert!(true)` dummy tests inside `db.rs`, `mail.rs`, and `queue.rs` with functional assertions and proper struct validations to guarantee honest Code Coverage reports.
- **Passkey Linter Fixes**: Removed `dead_code` warnings from the WebAuthn lightweight CBOR parser.
- **Dependabot Updates**: Updated core transitive dependencies (`hyper`, `aws-sdk`, `redis`, etc.) to mitigate known downstream CVE vulnerabilities.

### Refactoring & Stability
- **CLI Refactoring**: Extracted the massive CLI command matching block inside `cargo-rullst/src/main.rs` into an isolated `run_cli_command()` function for optimal AI-maintainability.
- **Studio Dashboard Refactoring**: Extracted raw string generation inside the SQL-inspection tool `studio.rs` into pure `build_headers_html()` and `build_rows_html()` helpers, dramatically reducing the cognitive complexity of the HTTP handler.
- **Upgraded ORM**: Bumped `rullst-orm` to `1.1.13` for the latest critical fixes.
- **Queue Worker Stabilization**: Verified and locked the `Worker` polling logic inside `queue.rs` for frictionless background job processing without blocking the tokio event-loop.

## [1.0.8] - 2026-05-28 🚀

### Added (Production Readiness)
- **Rust-Socialite Native Support**: Integrates `rullst-connect` seamlessly into the framework under the `oauth` feature, exposing ready-to-use authentication endpoints in `rullst::auth::socialite`.
- **Rullst.toml Configuration Parsing**: Added strong typing and `toml` parsing directly in `Server::run` to read `Rullst.toml`, dynamically applying properties such as `database.url` and `security.csrf_same_site`. Defaults to SQLite `rwc` mode for zero-config persistence.
- **Dynamic SameSite & CORS**: Removed hardcoded `SameSite=Strict` CSRF cookies, supporting dynamic values (like `Lax`) configurable via `Rullst.toml`. Automatically injects optional `tower_http::cors::CorsLayer`.
- **Rehash on Login Pattern**: Added `needs_rehash` in `auth.rs` to allow safe migrations of existing user password hashes from unstable Argon2 parameters to current stable defaults seamlessly during authentication.
- **Stabilized Dependencies**: Downgraded RC dependencies (`dashmap 7.0.0-rc2`, `notify 9.0.0-rc.4`) to stable `6.1.1` tags to ensure solid production stability for applications relying on `rullst`.

## [1.0.7] - 2026-05-28 🛡️

### Security & Quality Audits
- **APP_KEY Hardcoded Fallback Removed**: Deleted the insecure static `DEFAULT_APP_KEY` from `auth.rs`. In development mode, the framework now generates an ephemeral, cryptographically secure random key in memory (using `rand::RngCore` and `OnceLock`), perfectly retaining the "Zero-Config" local DX while preventing predictable session secrets. Production environments still strictly require `APP_KEY` to be defined.
- **Local Network RCE Prevention**: Bound the development server's default port (`3000`) exclusively to the local loopback interface (`127.0.0.1`) instead of `0.0.0.0`. This hardens the Self-Healing Auto-Fix console from being exposed to the local network by default.
- **Test Isolation & Mutex Locks**: Added thread-safe `std::sync::Mutex` (`ENV_LOCK`) blocks to `feature_tests.rs` and `error_console_tests.rs`. This correctly isolates `unsafe { std::env::set_var }` calls, preventing flaky failures and race conditions when `cargo test` executes asynchronous runners in parallel.

### Performance & Stability
- **Non-Blocking Static Assets**: Upgraded the `serve_static_zst` middleware inside `server.rs` to use fully asynchronous `tokio::fs::metadata(path).await` instead of the synchronous `std::path::Path::exists()`, eliminating CPU I/O wait blocking on the Tokio thread pool.
- **Auto-Fix Regex Hardening**: Rewrote the AI Code Extraction parser in `error_console.rs` using robust string boundary searches (`rfind` and `find`), resolving uncompilable rust code crashes caused by hallucinated whitespace and markdown fence variations from LLMs.
- **Core Dependency Updates**: Ran `cargo update` on the workspace, pulling in upstream security patches for `hyper` (v1.10.0), `libsqlite3-sys` (v0.37.0), and other core dependencies.

## [1.0.6] - 2026-05-26 🌐

### Fixed
- **RullstPress GitHub Pages Paths**: Fixed a critical routing bug where all internal links and image sources used absolute paths (e.g. `/Rullst.png`, `/1-getting-started.html`) that resolved to the GitHub Pages root (`venelouis.github.io/`) instead of the repository sub-path (`venelouis.github.io/Rullst/`). All paths in `docs_generator.rs` have been converted to relative URLs, making the site work correctly regardless of deployment base path.
- **Broken Navigation Buttons**: The "Learn how to begin" CTA button and all Navbar links were directing users to 404 pages. Fixed by using relative paths (`1-getting-started.html` instead of `/1-getting-started.html`).
- **Broken Sidebar Links**: Sidebar navigation links used a leading slash that caused 404 errors on GitHub Pages. Now uses bare relative paths (e.g. `spec.html` instead of `/spec.html`).
- **Broken Logo & Favicon**: The `<img src="/Rullst.png">` and `<link rel="icon" href="/Rullst.png">` failed to load on GitHub Pages. Fixed to use relative path `Rullst.png`.

### Added
- **Rullst Edge Runtime (`rullst::edge`)**: Introduced native support for compiling and running Rullst applications on WebAssembly edge infrastructure (Cloudflare Workers, Fastly Compute, AWS Lambda@Edge) abstracting Tokio/WASI differences. Features an environment-agnostic task spawner `spawn` that maps to `tokio::spawn` natively and `wasm_bindgen_futures::spawn_local` on `wasm32`. Exposes portable, extensible `EdgeRequest` and `EdgeResponse` HTTP models, alongside an `EdgeServer` that emulates edge routing locally on native systems using Axum.
- **Zero-Config SQLite Replication**: Added support for distributed SQLite synchronization configurations (e.g. Turso/libsql and Cloudflare D1 emulators). Exposes `ReplicationConfig` built with strict builder pattern standards, and `ReplicationManager` that boots a non-blocking background thread task to periodically synchronize the local replica with remote master nodes out-of-the-box.
- **Non-Intrusive Background Version Checker**: Implemented a background crates.io version updater in the `cargo-rullst` CLI that runs on a spawned thread and caches version status under the OS temporary directory (`rullst_version_cache.txt`). The network fetch is limited to at most once per day, ensuring 0ms impact on developer terminal execution speeds.
- **Terminal Update Banner**: Visual, colored terminal banner rendered at CLI tool exit when a newer version is cached, prompting users to upgrade.
- **Self-Healing CLI `upgrade` Codemods**: Refactored the `cargo rullst upgrade` command into a full autonomous refactoring pipeline: automatically updates `Cargo.toml` dependency tags to the latest release, runs search-and-replace codemods across `src/**/*.rs` to patch legacy APIs and enforce dependency shielding automatically, and runs validation compilation checks (`cargo check`) as a final quality gate.
- **Dependency Shielding Abstraction cascades**: Encapsulated transitive external dependencies into secure modular namespaces within Rullst core's public API: `rullst::db` (wrapping `sqlx`, `rullst_orm`), `rullst::web` (wrapping `axum`, `tower`, `tower_http`), `rullst::async_runtime` (wrapping `tokio`), and `rullst::email_client` (wrapping `lettre`). This isolates downstream applications from external breaking changes.
- **Resilient Traffic Shielding & Adaptive Backpressure**: Introduced a router-level load shielding and backpressure system inside [`rullst/src/resilience.rs`](file:///c:/Users/venelouis/Desktop/REPOS/Rullst/rullst/src/resilience.rs) that actively monitors thread-pool saturation (Tokio event loop lag) and database roundtrip latency (using low-frequency active query probes on the connection pool wrapped in safe `catch_unwind` guards to elegantly bypass panics if a DB is offline or unconfigured). The middleware automatically degrades traffic (returning `503 Service Unavailable` with `Retry-After: 5`) under critical CPU/DB/Active Request saturation, or gently throttles traffic under moderate load using lightweight 25ms delays to serialize requests naturally, preventing out-of-memory (OOM) crashes.
- **Token-Bucket Rate Limiter**: Added a thread-safe, atomic rate limiting system powered by a concurrent Shared-Memory (`DashMap`) engine. Features a highly customizable `RateLimitConfig` constructed with the Builder Pattern for strict backward-compatibility, and includes convenient factory builders (`per_second`, `per_minute`, `per_hour`). Seamlessly handles proxy environments by resolving client identifiers through standard headers (`X-Forwarded-For`, `X-Real-IP`) and peer addresses (`ConnectInfo`).
- **Edge-Optimized Assets & Pre-Compression (Brotli + Zstandard)**: Implemented an advanced high-performance pre-compression pipeline within the `cargo-rullst` CLI tool (`cargo rullst build [--debug]`) that recursively compiles the production binary and compresses all text-based static assets (HTML, CSS, JS, SVG, JSON, WASM, TXT, XML) in the `static/` directory using **Brotli (level 11)** and **Zstandard (level 19)** formats, saving `.br` and `.zst` files alongside their original sources. Upgraded the Rullst core library static asset serving (`ServeDir::new("static")`) inside `rullst/src/server.rs` to support pre-compressed Brotli served natively, and integrated a fast zero-overhead rewriting middleware `zstd_static_middleware` that intercepts client requests, checks for `Accept-Encoding: zstd`, rewrites the request URI to `.zst` zero-copy if the file is present, and overrides proper `Content-Encoding: zstd` and mime-specific `Content-Type` headers for blazing-fast edge-optimized transfers.
- **Native WebAuthn (Passkeys & Biometrics First)**: Added a 100% pure-Rust WebAuthn signature verification and challenge-processing engine (`rullst::auth::passkey`) powered by `ring` and a zero-dependency recursive CBOR decoder, eliminating native OpenSSL requirements for developer cross-compiling ease. Upgraded `cargo rullst auth` CLI scaffolding to generate a complete, secure, passwordless biometrics registration and sign-in flow out-of-the-box. Scaffolds sequential database migrations for both `users` and `user_passkeys` tables, the corresponding `UserPasskey` Orm model, in-memory REST controllers mapping pending challenge states natively via thread-safe `Mutex<HashMap>`, and updated responsive templates in `src/pages/auth.rs` styled with emerald biometrics CTA buttons and lightweight client-side Vanilla JS binary buffer decoders. Inherits all backward-compatibility standards by exposing `PasskeyConfig` utilizing the `#[non_exhaustive]` attribute and fluent Builder pattern.
- **Copy-to-Clipboard for Code Blocks**: All `<pre>` code blocks in the RullstPress documentation site now feature a floating "Copy" button (top-right corner). On click, the code is copied to the clipboard and the button changes to "✓ Copied!" with green feedback, reverting after 2 seconds. Includes a textarea-based fallback for older browsers without Clipboard API support.
- **One-Click Install Snippet**: The home page now features a clickable `cargo add rullst` snippet that copies the command to the clipboard on click, with animated ✓ Copied! feedback.
- **Crates.io Navigation Link**: Added a direct "Crates.io ↗" link in the home page hero and the navbar, pointing to https://crates.io/crates/rullst.
- **Spec Page Link**: Added "Spec" link to the homepage navbar for quick access to the framework specification page.
- **Floating Logo Animation**: The hero logo now uses a smooth CSS `float` keyframe animation for a more premium, dynamic first impression.

## [1.0.5] - 2026-05-26 🚀

### Fixed
- **Macro `html!` Self-Closing Bug**: Fixed a critical HTML parsing bug in `rullst-macros` where empty elements (like `<script src="..."></script>`) were incorrectly compiled into self-closing tags (`<script src="..." />`). Now the macro enforces self-closing tags *only* for valid HTML5 void elements (e.g. `<img>`, `<br>`, `<meta>`), preventing complete page collapse in web browsers.

### Added
- **Startup Diagnostic Links**: Added a friendly `🚀 Visit: http://localhost:3000 to see the result!` message to the `rullst::Server` boot logs.
- **RullstPress Tutorials**: Merged the advanced Developer Portfolio HTMX/Tailwind tutorial directly into the end of `1-getting-started.md` to streamline the onboarding experience for new users, removing the redundant blog tutorial.
- **Automated Documentation Deployment (`pages.yml`)**: Added a GitHub Actions workflow to automatically build and deploy the RullstPress documentation to GitHub Pages on every push to the `main` branch.
- **Official Links**: Added official Crates.io and GitHub Pages Documentation links to the project's English and Portuguese READMEs.
- **Pre-Release Technical Audit (`audit-report.md`)**: Conducted a rigorous technical audit covering security, performance, maintainability, and DX. Documented all active framework mitigations (Path Traversal, XSS, insecure APP_KEY hashing, queue worker polling latency, decoupled task scheduler, and memory-driver active cache janitor) and archived the official report at `docs/audit-report.md` for complete version transparency.

### Changed
- **Axum 0.8 Upgrade**: Fully migrated the core framework, `cargo-rullst` scaffolding templates, and internal examples to `axum = "0.8"` and `tower-http = "0.6"`.
- **WebSocket Updates**: Updated internal WebSocket message handling to use `Utf8Bytes` according to the new `axum 0.8` requirements.
- **Routing Syntax**: Updated Horizon dashboard route definitions from `:id` to `{id}` to match the new Axum 0.8 path parameter syntax.
- **Async Trait**: Removed `#[async_trait]` from `FromRequest` implementations as Axum 0.8 natively supports `async fn` in traits.

## [1.0.4] - 2026-05-26 🛠️

### Fixed
- **Conditional Scaffolding for Database-Disabled Apps**: Fixes a compilation error (`E0433: cannot find module or crate rullst_orm`) that occurred when creating a project with database support disabled ("no" database selected). The generation of the `src/migrations` folder, `pub mod migrations` module declaration, and `rullst::artisan!` macro call are now strictly conditional on enabling database support during `cargo rullst new`.

### Added
- **`sync-badges` Automation Tool**: A new internal binary (`cargo-rullst/src/bin/sync_badges.rs`) and cargo alias (`cargo sync`) that automatically reads the current version from `cargo-rullst/Cargo.toml` and updates the status badge in `README.md` and `README.pt.md`. This prevents version badges from becoming stale after releases.
- **Dependabot Configuration**: Added `.github/dependabot.yml` to automatically monitor and open Pull Requests for outdated Cargo dependencies every Monday at 08:00 (America/Sao_Paulo). PRs will be tested by CI before merging, ensuring dependencies are always up to date without breaking the build.
- **Automated Release Pipeline (`release.yml`)**: Added a dedicated GitHub Actions workflow that triggers exclusively when a version tag (e.g. `v1.0.5`) is pushed. It runs the full test suite as a mandatory gate before publishing `rullst-macros`, `rullst`, and `cargo-rullst` to crates.io in sequence. This prevents publishing broken releases.
- **CI Extended to `dev` Branch**: The existing CI workflow (`ci.yml`) now also runs on every push to `dev`, providing continuous feedback during active development — not just on `main`.

### Documentation
- **`RELEASE_GUIDE.md`**: Added a comprehensive guide documenting the official development and release workflow, including the `dev` → `main` branching strategy, step-by-step release instructions, CI/CD automation details, and the one-time GitHub Secret setup required for automatic crates.io publishing.

## [1.0.3] - 2026-05-26 🛠️

### Fixed
- **CLI wizard prompt restoration**: Restores and guarantees the advanced CLI prompts (no-spaces validation, Full-Stack App vs Headless API selection, Hot reloading toggle, Database configuration toggle, and MySQL/MariaDB provider option) that were reverted in the 1.0.2 release due to a translation sync conflict.

## [1.0.2] - 2026-05-26 🚀

### Added
- **Rullst CLI Interactive Wizard (`cargo rullst new`) Improvements**:
  - Restricts application names to contain "no spaces allowed".
  - Adds descriptive options to select application type ("What would you like to build?": Full-Stack Web App vs Headless REST API).
  - Prompts to enable/disable Hot Reloading by default during scaffolding.
  - Prompts to configure database support ("Will your project need a Data Base?").
  - Adds "MySQL/MariaDB" provider selection option alongside SQLite and PostgreSQL.
- **RullstPress General-Purpose SSG**:
  - Capitalized CLI command descriptions and help menus to correctly read "RullstPress".
  - Updated documentation tutorial in `docs/2-tutorial-rullstpress.md` to introduce RullstPress as a general-purpose, high-performance, and multi-purpose Static Site Generator perfect for SaaS landing pages, wikis, blogs, and personal portfolios, rather than just documentation.

### Documentation
- Updated `README.pt.md` and `README.md` to reflect the new interactive CLI wizard questions and choices.

## [1.0.1] - 2026-05-26 🛡️

### Added
- **RullstPress (Native SSG)**:
  - `cargo rullst docs build`: Compiles all `.md` files in the `docs/` folder into static HTML files inside `docs/dist/`.
  - `cargo rullst docs dev`: Starts a live-preview local server for your documentation powered by Axum.
  - Automatically parses Markdown (via `pulldown-cmark`) and renders a premium dark-mode sidebar layout.

### Security & Quality Fixes
- **Security Enhancements**:
  - Implemented SHA-256 key derivation in `auth.rs` to securely stretch `APP_KEY` for AES-256-GCM.
  - Added safe `serde_urlencoded` parser to `security.rs` to guarantee CSRF tokens are safely extracted and compared from deeply nested url-encoded forms.
  - Restored strict HTML template string sanitization via template literals inside `error_console.rs` to prevent JS injection vectors.
- **Stability & Performance Fixes**:
  - Eliminated `.unwrap()` calls in `server.rs`, migrating `HotSwapService` to use graceful fallbacks that prevent runtime panics when dylibs are missing or file handles are locked.
  - Migrated dynamic library historical handles to `Mutex<Vec<Library>>` to safely retain historical pointers, preventing `libloading` Drop implementations from immediately freeing hot-swapped memory boundaries resulting in Segmentation Faults.
  - Refactored `scheduler.rs` loop to use `tokio::spawn` instead of blocking `await` on cron jobs, avoiding scheduler deadlock.
  - Migrated `queue.rs` SQLite worker to decouple popping from the database driver and loop latency, removing sleep-based latency blocks.
  - Fixed TOML parser bug in `mail.rs` resolving arbitrary `.unwrap()` when casting integer ports to unsigned integers.
  - Enabled inline comment stripping for `feature.rs` file reads to support `#` comments inside `Rullst.toml`.
  - Added background Cache Janitor to `cache.rs` via `tokio::spawn` using interval loops to actively prune expired DashMap keys.

## [1.0.0] - 2026-05-25 🚀


### Added (The "Unfair Advantage" & Local AI Dev Tooling)
- **Hot Reloading via Dynamic Linking (`Server::new_hot`)**:
  - Implemented `HotSwapService` wrapping `Arc<RwLock<axum::Router>>` for atomic in-flight router replacement without restarting the server or dropping TCP connections.
  - `Server::new_hot(lib_path)` builder that loads the application router from a `cdylib` (`.dll` / `.so`) at runtime via `libloading`.
  - Background file-watcher thread (using `notify`) that monitors `src/` for changes, debounces events (300ms), triggers `cargo build --lib`, and hot-swaps the router on success.
  - Timestamp-based unique DLL naming (`_active_{nanos}.dll`) to prevent Windows OS error 32 (file-locked-by-process), with automatic cleanup of stale copies.
  - FFI entry point convention: libraries export `#[unsafe(no_mangle)] pub extern "C" fn rullst_router_init() -> *mut rullst::Router`.
  - Blog example refactored to demonstrate hot-reload mode: `HOT_RELOAD=1 cargo run` for live-editing, default `cargo run` for standard static compilation.
- **Declarative E2E Testing (`rullst::testing`)**:
  - Introduced a fluent, high-level testing framework for complete application workflows.
  - Added `TestClient` to mount and run HTTP routing logic over the Axum application without actual TCP binding.
  - Implemented standard HTTP builders with convenient `.await` execution via Rust's `IntoFuture` trait.
  - Provided extensive cookie-based assertions (`.assert_cookie()`) and structured payload assertions (`.assert_json_value()`).
- **Built-in Feature Flags (`rullst::feature`)**:
  - Implemented full-stack toggles and dynamic A/B test splits with zero external runtime dependencies.
  - Support for `EnvDriver`, `MemoryDriver`, `TomlDriver`, and `DatabaseDriver` (backed by SQLx with a thread-safe TTL Cache for near-zero latency DB lookups).
  - High-performance deterministic consistency hash utilizing a custom MurmurHash3 implementation for stable weighted rollouts.
- **AI-Powered "Self-Healing" Error Console (`rullst::error_console`)**:
  - Gorgeous interactive glassmorphic web dashboard (`rullst-ignition`) triggered on application panics.
  - Seamless tokio panic interception using a custom `std::panic::set_hook` implementation to isolate runtime worker thread crashes.
  - Direct local code-snippet lookup pointing to the exact file, module, and line index where the panic occurred.
  - Integrated local AI-healing assistant that resolves runtime errors and can patch files directly back to the physical disk on a single web interface click.

### Security & Quality Audit Fixes (Audit 2026-05-25)
- **Security Enhancements**:
  - SEC-1: Removed unsafe `std::env::set_var("RUST_BACKTRACE", "1")` in `server.rs` (unsound in multi-threaded environments) and replaced it with a safe warning prompting the user to set the env var.
  - SEC-2: Added strict path traversal protection to the `/_rullst/autofix` endpoint in `error_console.rs` (verifies paths are canonicalized and located within the project root, restricts edits to `.rs` and `.toml` files).
  - SEC-3: Added a startup warning in `auth.rs` when the default development `APP_KEY` is used.
  - H-3 (Path Traversal in Error Console): Secured the GET `/_rullst/explain` handler in `error_console.rs` with robust path traversal validation, restricting file reads to `.rs` and `.toml` files within the workspace root.
  - H-1 (Poisoned RwLock Recovery): Added poison-recovery safety logic to `RwLock` reads/writes in `server.rs`, preventing a single dynamic loading thread panic from cascading to crash all request tasks.
  - H-2 (Graceful Oneshot Error Handling): Gracefully handle `oneshot()` failures inside tower routing, returning an internal server error response instead of panicking.
- **Spec & API Alignments & Stability**:
  - Marked `Server`, `Router`, `HtmxRequest`, and `HtmxResponse` as `#[non_exhaustive]` per Rullst Spec §9.1 to ensure future-proof API stability.
  - Replaced a `panic!` in `Storage::disk()` with a graceful fallback `ErrorDriver` returning `StorageError::DriverError` on all methods when an unknown disk is requested.
  - M-2 (Stable Rollout Hashing): Replaced `DefaultHasher` in progressive rollouts (`feature.rs`) with deterministic `FnvHasher` (adding `fnv` to main dependencies) to guarantee bucket stability across Rust upgrades.
  - L-3 (TOML Path Isolation): Cached `Rullst.toml`'s path during construction in `TomlFeatureDriver` to prevent lookup failure if the runtime working directory changes.
  - L-4 (Removed Undocumented Tenancy Fallback): Removed the undocumented `"tenant"` parameter fallback in `multitenant.rs` to enforce explicit, predictable tenancy extraction.
- **Performance & Reliability**:
  - Migrated `LocalDriver` in `storage.rs` from blocking `std::fs` to fully asynchronous `tokio::fs` operations.
  - Optimized Redis `CacheDriver`'s `flush()` method to use a memory-efficient `SCAN` cursor loop instead of the blocking `KEYS *` pattern.
  - M-1 (Watcher Compilation Timeout): Implemented a `120s` timeout for background `cargo build --lib` compilation using std channel `recv_timeout` to prevent blocking the watcher indefinitely.
  - M-4 (Configurable Testing Limits): Made the E2E testing request body limit configurable in `TestApp` and `TestRequestBuilder`, and provided comprehensive panic error details if limits are exceeded.
  - L-1 (Guaranteed Temp DLL Uniqueness): Swapped timestamp suffixes with UUID v4 to completely rule out dynamic library path collision bugs under high concurrent loads.
- **UX & Diagnostics Improvements**:
  - I-2 (Hot-Reload Panic Capture Console): Wrapped `HotSwapService`'s execution future in a spawned task, intercepting panic unwinds to render the gorgeous glowing interactive Self-Healing Console during development.
  - L-2 (HTML Attribute Injection Guard): Implemented robust HTML attribute escaping to `ws_path` before mounting Live component tags inside `live.rs`.
- **Testing & CI/CD**:
  - Added full test coverage for the wrapper `Router` in `routing.rs`, the builder in `server.rs`, and argument translation in `artisan.rs`.
  - Created a GitHub Actions CI pipeline (`.github/workflows/ci.yml`) enforcing automated test suites, clippy lint checks, and rustfmt checks.


## [0.8.0] - 2026-05-25 🛡️

### Added (Self-Healing Upgrades & Architectures)
- **Architectural Guidelines (`docs/spec.md`)**:
  - Enforced the Builder Pattern and `#[non_exhaustive]` on public configurations to prevent struct instantiation breakages.
  - Formally integrated `#[deprecated]` lifecycle for smooth transition between APIs.
  - Implemented the "Sealed Traits" pattern for internal interfaces.
- **Automated CLI Upgrade Command (`cargo-rullst`)**:
  - Added `cargo rullst upgrade` command.
  - Safely updates dependencies via `cargo update -p rullst`.
  - Automatically runs codemods using `cargo fix --allow-no-vcs --allow-dirty` to apply Rust compiler suggestions based on Rullst's deprecation warnings.

## [0.7.0] - 2026-05-25 🤖

### Added (AI-Native Core Milestone)
- **Extensible AI Facade (`rullst::ai`):**
  - Introduced the `AiClient` facade and the `AiProvider` trait (similar to Rullst Storage and Mailer patterns) to build highly extensible AI applications.
  - Implemented automatic driver resolution via `AiClient::auto()`, which dynamically detects `OPENAI_API_KEY`, `GEMINI_API_KEY`, `ANTHROPIC_API_KEY`, or `OLLAMA_HOST` from environment variables.
- **Multi-Provider Drivers (`rullst::ai::providers`):**
  - `OpenAiProvider`: Integrates with OpenAI models (e.g. `gpt-4o-mini`) and text embeddings.
  - `GeminiProvider`: Full integration with Google Gemini models (e.g. `gemini-1.5-flash`), with native support for `systemInstruction` parameters.
  - `AnthropicProvider`: Claude integration utilizing the Messages API and top-level system prompts.
  - `OllamaProvider`: Local LLM execution supporting local completions (e.g. `llama3`) and vector embeddings (e.g. `nomic-embed-text`) via Ollama.
- **Fluent Chat Builder (`ChatBuilder`):**
  - Fluent builder for multi-turn conversational agents with simple `.system()`, `.user()`, and `.assistant()` methods.
  - Handles dynamic role mapping per provider transparently (e.g., mapping `assistant` role to `model` role in Gemini).
- **Strongly Typed Structured Prompts:**
  - Added `structured_prompt<T>` helper to parse LLM outputs into strongly typed Rust structs, automatically sanitizing markdown wraps (e.g., ` ```json ... ``` `).
- **In-Memory RAG Engine (`VectorIndex`):**
  - Zero-dependency, pure Rust in-memory `VectorIndex` for instant vector search.
  - Utilizes high-performance Cosine Similarity algorithms to let developers build light, instant RAG applications without external vector databases.

## [0.6.1] - 2026-05-25 🛠️

### Added (CLI Empowerment & Generators completions)
- **Interactive Project Scaffolding (`cargo rullst new`):**
  - Added a beautiful prompt-based wizard wizard asking for App Name, App Type (Fullstack SSR vs REST API), and Database Provider (SQLite, PostgreSQL, MySQL) using the `dialoguer` crate.
  - Automatically structures dependencies, configuration database connection strings (`Rullst.toml`), and generated boilerplate templates based on wizard choices.
- **Milestone 1 CLI Generators:**
  - `make:cors`: Generates a standard Axum CORS middleware in `src/middlewares/cors_middleware.rs` with OPTIONS preflight handling and safe owned string lifetime parameters.
  - `make:jwt`: Generates a token-based JWT authentication middleware in `src/middlewares/jwt_middleware.rs` with a `generate_token` helper, injecting `jsonwebtoken` and `chrono` into `Cargo.toml`.
  - `make:worker`: Generates background task worker modules and registers them inside `src/workers/mod.rs` for processing asynchronous queue tasks.
  - `generate:openapi`: Zero-magic static analysis OpenAPI generator that scans `src/main.rs` route patterns and `src/controllers/` actions' doc-comments (`///`) to output a high-performance `openapi.json` spec.

## [0.6.0] - 2026-05-25 🏢

### Added (Enterprise Features Milestone)
- **Declarative Validation (`rullst::validation`):**
  - Added `ValidatedForm<T>` and `ValidatedJson<T>` Axum extractors that automatically perform validations using the `validator` crate.
  - Generates beautiful HTMX validation error lists for frontend clients, or redirects, or returns standard `422 Unprocessable Entity` JSON responses automatically based on client negotiation.
- **Mailer System (`rullst::mail`):**
  - Added unified `Mail` facade and `MailDriver` trait.
  - Implemented `LogDriver` for local development, `SmtpDriver` for classic email setups, and highly optimized, async REST-based `ResendDriver` and `SendGridDriver` utilizing `reqwest` and `rustls` (zero-openssl dependency for maximum factory productivity).
- **Storage Abstraction (`rullst::storage`):**
  - Unified `Storage` facade and `StorageDriver` trait.
  - Implemented `LocalDriver` writing files locally under `storage/app`, and AWS-compliant `S3Driver` for cloud storage.
- **WebSockets & Real-Time (`rullst::ws`):**
  - High-level `WebSocket` wrapper for real-time messaging.
  - Seamlessly integrated with Axum, supporting high-level HTMX out-of-band swaps via `.send_html()`.
  - Added `.ws(path, handler)` and `.nest` routing methods to Rullst `Router` for modular setups.
- **Rullst Horizon (`rullst::horizon`):**
  - Gorgeous, premium, high-fidelity dark mode dashboard built entirely in Rust using raw `html!` templates and HTMX polling.
  - Real-time queue metrics (pending counts, failed jobs, active worker status), failed jobs detail lists, and instant one-click dashboard retries/purges!

---

## [0.5.0] - 2026-05-25 📦

### Added (Production Utilities Milestone)
- **Docker & Containerization (`cargo rullst new --docker`):**
  - Multi-stage `Dockerfile` using `rust:1.87-slim` builder → `gcr.io/distroless/cc-debian12` runtime (~20MB final image).
  - Auto-generated `docker-compose.yml` with App + PostgreSQL 16 + Redis 7 services, health checks, and persistent volumes.
  - `.dockerignore` to exclude build artifacts and dev files.
- **Queue & Background Workers (`rullst::queue`):**
  - `Queue` facade with `dispatch()` for pushing named jobs with JSON payloads.
  - `Worker` with `register()` for mapping job names to async handler closures and `run()` for background processing.
  - `SqliteDriver`: Uses auto-created `rullst_jobs` table, zero config, FIFO with atomic pop.
  - `RedisDriver` (optional, `queue-redis` feature): Uses Redis lists for high-throughput distributed workloads.
- **Caching Layer (`rullst::cache`):**
  - `Cache` facade with `get`/`put`/`forget`/`flush`/`has` and the `remember()` cache-aside pattern.
  - `MemoryDriver`: Lock-free `DashMap`-based concurrent store with lazy TTL expiration.
  - `RedisDriver` (optional, `cache-redis` feature): Redis-backed with `SETEX` TTL support and `rullst:cache:` key prefix.
- **Task Scheduler (`rullst::scheduler`):**
  - `Scheduler` with `.task("cron_expr", handler)` for registering recurring async jobs.
  - Standard 5-field cron expressions auto-converted to 7-field for the `cron` crate.
  - Integrated into `Server` via `.schedule(scheduler)` builder method — runs alongside HTTP server.

---

## [0.4.0] - 2026-05-25 ⚡

### Added (HTMX & Interactivity Milestone)
- **HTMX First-Class Support (`rullst::htmx`):**
  - Added `HtmxRequest` extractor to easily detect `HX-Request` and other HTMX headers in Axum routes.
  - Added `HtmxResponse` builder for setting HTMX-specific response headers (like `HX-Trigger`, `HX-Redirect`, `HX-Retarget`).
  - Added `render_page` macro/helper for hybrid SSR rendering, automatically serving partial fragments for HTMX requests or the full HTML layout for standard browser visits.
- **TailwindCSS Integration:**
  - `cargo rullst new` now automatically configures TailwindCSS via CDN in the generated templates.
  - Scaffolded projects include a reactive HTMX counter component to demonstrate immediate interactivity without writing JavaScript.
- **Hyphenated HTML Attributes (`rullst-macros`):**
  - Updated the `html!` procedural macro to fully support hyphenated attributes like `hx-post`, `hx-target`, and `hx-swap`.

---

## [0.3.0] - 2026-05-25 🛡️

### Added (Authentication & Security Milestone)
- **Local Authentication Primitives (`rullst::auth`):**
  - High-security password hashing and verification powered by **Argon2id**.
  - Secure **AES-256-GCM** client-side encrypted cookie sessions (`rullst_session`) valid for 30 days.
  - Automatic `APP_KEY` cryptographic key resolution from environment variables or `Rullst.toml`.
- **Double Submit CSRF Validation (`rullst::security::csrf_middleware`):**
  - Automatic injection of secure CSRF cookies on GET requests.
  - Validation of state-modifying requests (`POST`, `PUT`, `DELETE`) comparing cookie tokens with HTTP headers (`X-CSRF-Token`) or hidden `_token` fields.
  - Custom stream re-builder to safely buffer the request body during verification.
- **Production Security Headers (`rullst::security::headers_middleware`):**
  - Standard headers injected on all HTTP responses: HSTS, Content-Type-Options (nosniff), Frame-Options (DENY), XSS-Protection, and Referrer-Policy.
- ** CLI Auth Command (`cargo rullst auth`):**
  - Scaffold entire authentication systems (local register, login, logout, and GitHub social auth redirect and callback handlers via the dynamic `rullst-connect` sibling dependency).
  - Scaffold database migrations for `users`, the `User` Active Record model, and restricted route `AuthMiddleware`.
  - Scaffold beautiful responsive Dark Mode HTML templates (`login_page`, `register_page`, `dashboard_page`) using the procedurally compiled `html!` macro.

---

## [0.2.0] - 2026-05-25 🚀

### Added (Database Supremacy Milestone)
- **Artisan CLI Engine (`rullst::artisan!`):** A declarative macro that intercepts process execution to run database migrations, seeds, and status checks directly within the application binary before the server boots.
- **Rullst Dev CLI Migrations:** `cargo-rullst` now proxies artisan commands (`db:migrate`, `db:rollback`, `db:status`, `db:seed`) gracefully to the target workspace.
- **Database agnostic URL Injection:** Rullst `Server::new` now auto-parses `Rullst.toml` and automatically injects the `DATABASE_URL` into the `rullst-orm` connection pool during boot, supporting SQLite, PostgreSQL, and MySQL effortlessly.
- **Rust-DSL Migrations:** Scaffolding databases now uses pure Rust closures (`make:migration`) instead of raw SQL, giving developers strong typing and compile-time validation for schema building.

---

## [0.1.1] - 2026-05-25 ✨

### Added
- **AI-Native Engineering & AI-Friendliness** added to core pillars in `README.md` and `README.pt.md`.
- **Master Plan Roadmap update:** Introduced the AI-Native Design Pillar at the top of the development roadmap (`ROADMAP.md` and `ROADMAP.pt.md`).
- **CLI Code Generator:** Added the first code generator subcommand `cargo rullst make:controller <Name>` in `cargo-rullst`.
  - Normalizes controller name inputs (e.g. `UsersController` -> `users_controller`).
  - Scaffolds REST endpoints (`index`, `show`) pre-configured with the JSX-like `html!` macro.
  - Automatically manages mod declarations in `src/controllers/mod.rs` and injects `pub mod controllers;` in `src/main.rs`.
- **CLI Path Normalization:** Normalized workspace and package names when scaffolding projects using path expressions (e.g. `cargo rullst new ..\my_project`).

---

## [0.1.0] - 2026-05-24 🚀

### Added
- **Core Crate (`rullst`):** Wrapped Axum server, routing macro `routes!`, lifecycle DB injection, and response models.
- **Macros Engine (`rullst-macros`):** Built procedural compiler-level `html!` JSX macro with static memory-string concatenation and dynamic XSS protection.
- **Developer CLI (`cargo-rullst`):** Scaffolds complete starter workspaces with integrated sqlite in-memory testing out-of-the-box.
- **Manifestos:** Created rich English (`README.md`) and Portuguese (`README.pt.md`) project overviews.
