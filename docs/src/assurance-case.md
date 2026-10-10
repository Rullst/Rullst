# Security Assurance Case

This page argues why Rullst's security requirements are met, and where they
are not. It ties together the threat models, the trust boundaries, the design
principles and the countermeasures for common weaknesses, and points to the
code and tests behind each argument. Source paths are relative to the
[repository root](https://github.com/Rullst/Rullst/tree/main); tests are named
as `file::test_function`.

It covers the framework crates, the `cargo rullst` CLI and the release
pipeline. It is not a certification, a penetration test or a guarantee for any
application built with Rullst. Applications own their data model, identity
policy, deployment and providers, and must extend the threat model for them.

## Top-level claim

Used as documented, Rullst's framework-owned controls counter the abuse cases
named in its threat models, fail closed when they are missing or
misconfigured, and ship in releases that users can verify. Every narrower
claim below cites its mechanism and at least one negative test. The
[limits](#limits) section lists what is not claimed.

## Threat model

The [threat models](threat-models.md) (model version TM-13.0; v12 keeps
TM-12.10) use a lightweight STRIDE review. Each model separates untrusted
network input, authenticated application identity, process-local state, shared
durable state and third-party or hardware trust, and names abuse cases with
stable IDs such as `AUTH-06` or `STUDIO-01`. There are 19 models:

| Area | Models |
| --- | --- |
| Runtime and network identity | `TM-CORE-1` (readiness and drain), `TM-CORE-2` (client identity behind proxies) |
| Identity | `TM-AUTH-1` (sessions, passwords, OAuth/OIDC, passkeys), `TM-CONNECT-1` (OAuth token generations) |
| Administration and tooling | `TM-NEXUS-1` (admin CMS), `TM-STUDIO-1` (local control room) |
| Data | `TM-TENANT-1` (multi-tenant access), `TM-ORM-1` (document recovery) |
| Request defenses | `TM-SEC-1` (payload contracts), `TM-SEC-2` (anomaly and proof-of-work admission), `TM-SEC-3` (security-event journal) |
| Integrations | `TM-PAY-1` (webhooks and billing), `TM-MAIL-1` (outbound mail), `TM-MESSAGING-1` (messages and wire frames), `TM-AI-1` (prompts, RAG, tools) |
| Devices and education | `TM-IOT-1` (OTA and devices), `TM-ACADEMY-1`, `TM-LABS-1` |
| Supply chain | `TM-DEPLOY-1` (CLI, artifacts, release and deployment) |

The machine-readable minimum in
[`.github/threat-model-release-minimum.json`](https://github.com/Rullst/Rullst/blob/main/.github/threat-model-release-minimum.json)
binds 48 abuse-case IDs to 64 evidence rows and 63 exact tests across thirteen
crates. `check-threat-model-release-minimum.sh` rejects missing markers,
missing tests and filters that match zero tests, then runs each test with
`--exact`. It runs in four CI shards and again in the release workflow.
Changes to authentication, authorization, tenant resolution, webhooks, AI
tools, OTA or the release flow must reference an abuse-case ID (see
[review and change control](threat-models.md#review-and-change-control)).

Related evidence: the [v12 security claims ledger](v12-security-claims.md)
lists each narrow claim with its tests and known limits, and the
[external audit kit](external-audit-kit.md) gives third-party reviewers the
scope, a sample application and tooling.

## Trust boundaries

| Boundary | Untrusted side | Enforced by | Model |
| --- | --- | --- | --- |
| Browser or client ↔ application | Every request, header, cookie, WebSocket upgrade and body | The [canonical production order](security-architecture.md#canonical-production-preset) (`rullst-core/src/production.rs`): trusted-proxy policy, body limit, secure headers, CORS allowlist, WAF/RASP, CSRF, then application-owned session, authentication, tenant and authorization slots | `TM-AUTH-1`, `TM-SEC-1` |
| Reverse proxy ↔ application | `Forwarded` and `X-Forwarded-For` | `TrustedProxyLayer` (`rullst-core/src/security/trusted_proxy/`): the direct socket peer is the identity until exact proxy hops are configured | `TM-CORE-2` |
| Identity ↔ tenant and object | Route IDs and tenant claims | `RbacGuard::authorize_owner_or_role` and `UserContext` (`rullst-security/src/rbac/guard.rs`), `TenantContext` (`rullst-core/src/security/tenant_guard.rs`), ORM `tenant_column` scoping | `TM-TENANT-1` |
| Application ↔ database | Values and identifiers that reach SQL | Bind parameters; identifier validation and operator allowlists in the query builder | `TM-ORM-1`, `TM-TENANT-1` |
| Application ↔ external providers | Webhooks, OAuth/OIDC responses, AI output, provider URLs | Signature checks with constant-time comparison, replay stores, HTTPS-only provider URLs, deny-by-default egress | `TM-PAY-1`, `TM-MAIL-1`, `TM-CONNECT-1`, `TM-AI-1` |
| Content ↔ model; model output ↔ tools | Retrieved documents and model replies | Prompt-injection filter, PII masking and guarded tool dispatch ([guarded local AI tools](ai-tool-security.md)) | `TM-AI-1` |
| Developer tools ↔ network | Remote access to Studio and debug endpoints | Debug-only, Development-only, loopback-only mounting; same-origin checks for state changes | `TM-STUDIO-1`, `TM-NEXUS-1` |
| Firmware signer ↔ device | OTA manifests | Ed25519 manifest signatures and a caller-provided rollback counter (`rullst-iot/src/ota.rs`) | `TM-IOT-1` |
| Contributor and CI ↔ repository; tag ↔ artifact ↔ registry | Pull requests, dependencies, build runners | Protected branches with 46 required checks, pinned Actions, release admission of the exact SHA, attestations and reproducible archives ([verifying a release](../../SECURITY.md#verifying-a-release)) | `TM-DEPLOY-1` |
| CLI ↔ filesystem and processes | Project names, paths, templates | Identifier and path validation in generators; fixed programs with argument arrays | `TM-DEPLOY-1` |

## Secure design principles

The argument follows Saltzer and Schroeder's principles.

| Principle | How it is applied | Evidence |
| --- | --- | --- |
| Least privilege | Workflow tokens default to read-only access (`contents: read`). In the release workflow only `attest` may write attestations, only `publish` gets a registry identity, and only `github_release` may write repository contents. API tokens carry an exact scope list without wildcards or implied parent scopes. | [`release.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/release.yml), [scoped API tokens](api-tokens.md) |
| Fail-safe defaults | Optional subsystems are off by default (`default = []` in Core, Security and Auth). Production webhook state rejects `mock_*` verifiers. Documented placeholder `APP_KEY` values and weak keys are refused. The CORS scaffold fails on a missing or `*` origin. Image fetches for AI use a deny-by-default egress policy. | `rullst-capital/src/webhook.rs` (`WebhookMiddlewareState::production`), `rullst-auth/src/auth/app_key.rs`, [CORS advisory](cors-scaffold-security-advisory.md), `rullst-ai/src/ai/egress.rs` |
| Complete mediation | `Server` applies the security baseline (headers, CORS, WAF, CSRF) to the whole router in staging and production, not per route. Every parameterized route must carry a `// rullst-access:` classification, and `cargo rullst audit --idor` fails when one is missing. | `rullst-core/src/server/stack.rs`, `rullst-core/src/security/baseline.rs`, [access contract](security-architecture.md#parameterized-route-access-contract) |
| Economy of mechanism | Twenty focused crates with feature-gated optional parts; one escaping path for `html!`; one canonical middleware order; source files target 500 lines. Maturity tiers keep experimental code out of the core promise. | [maturity tiers](maturity.md), `.github/check-crate-architecture.sh` |
| Separation of privilege | Releases pass through separate jobs: admit the exact SHA, verify, attest without executing source, reproduce, then publish from a protected `crates-io` environment that requires a reviewer. Branch rules have no bypass actors. | `release.yml`, [governance](../../GOVERNANCE.md) |
| Least common mechanism | Tenant data is scoped per request through `TenantContext` and ORM tenant columns, with fail-closed checks, instead of one shared unscoped query path. Studio and the debug consoles are separate mechanisms, mounted only in debug Development builds and reachable only from loopback. | [spec, tenant scope contract](spec.md), [threat models](threat-models.md) |
| Psychological acceptability | Secure behavior is the default path: `cargo rullst new` scaffolds the baseline, `cargo rullst doctor` reports a security baseline, `cargo rullst audit` checks routes, and `html!` escapes without extra calls. | [CLI reference](cli_reference.md), [which security layer to use](security-layers.md) |
| Open design | The source, threat models, claims ledger and audit kit are public. Security relies on keys and verified controls, not on secret designs. | [threat models](threat-models.md), [claims ledger](v12-security-claims.md), [external audit kit](external-audit-kit.md) |

## Countering common implementation weaknesses

The table maps OWASP Top 10 and CWE Top 25 style weaknesses to the mechanism
and to tests that fail if the mechanism regresses.

| Weakness | Mechanism | Tests | Limits |
| --- | --- | --- | --- |
| SQL injection (CWE-89) | The ORM binds values as parameters. Column names pass `validate_identifier` (`rullst-orm/src/schema/validation.rs`); operators come from an allowlist. Nexus and Studio use `sanitize_identifier`. | `rullst-orm/src/schema/tests.rs::test_validate_identifier`, `rullst-orm/tests/builder_validation_test.rs::joins_and_vector_helpers_reject_dynamic_sql_fragments_that_are_not_safe`, `rullst-studio/src/data_browser/tests.rs::test_sanitize_identifier`; Kani proofs for `sanitize_identifier` | `where_raw`, `or_where_raw` and `select_raw` take caller-written SQL. |
| Path traversal (CWE-22) and OS command injection (CWE-78) | `validate_relative_path` (`rullst-core/src/storage.rs`) and cloud `validate_key` reject `..`, absolute paths and control characters; static files accept only plain paths. The CLI runs fixed programs with argument arrays and validates names as Rust identifiers. | `rullst-core/src/storage.rs::relative_path_validation_rejects_escape_attempts`, `rullst-core/src/storage.rs::local_storage_rejects_symlink_escape`, `rullst-core/src/storage/cloud/tests.rs::every_cloud_operation_rejects_unsafe_or_ambiguous_keys`, `rullst-core/src/server/server_middleware.rs::only_plain_static_paths_are_probed` | A tenant prefix does not authorize access to every object in that tenant. |
| Cross-site scripting (CWE-79) | `html!` (`rullst-macros/src/html_parser.rs`) escapes every interpolation, neutralizes `javascript:` and `vbscript:` URLs and rejects dynamic event-handler attributes at compile time. Nexus escapes stored values. A per-response CSP nonce is set by the header layer. | `rullst-core/src/html_macro_tests.rs::dynamic_url_attributes_neutralize_script_schemes`, compile-fail fixture `rullst-macros/tests/ui/dynamic_event_handler.rs`, `rullst-nexus/tests/renderer_escaping.rs::stored_cells_remain_text_including_entity_prefixes`, `rullst-security/src/headers.rs::nonce_is_available_to_renderer_and_matches_header` | `RawHtml`, custom escaping, JavaScript contexts and dynamic `style` values stay the caller's responsibility. |
| Cross-site request forgery (CWE-352) and cross-site WebSocket hijacking | Double-submit cookie middleware (`rullst-core/src/security/csrf.rs`) with constant-time comparison and `Secure` cookies in production; an Origin allowlist for WebSocket upgrades (`rullst-security/src/cswsh.rs`). | `rullst-core/src/security/csrf_tests.rs::empty_or_ambiguous_csrf_proofs_do_not_reach_the_handler`, `rullst-core/src/security/csrf_tests.rs::production_like_environment_sets_secure_cookie`, `rullst-security/src/cswsh.rs::middleware_rejects_a_deceptive_localhost_origin` | The unsigned double-submit scheme depends on cookie isolation between sites; no blueprint mounts the CSWSH layer by default. |
| Broken authentication and session management (CWE-287, CWE-916, CWE-307) | Argon2id password hashing off the async runtime, with a dummy verification against user enumeration (`rullst-auth/src/auth/password.rs`). Cookie sessions are AES-256-GCM encrypted and `HttpOnly`. `LoginGuard` adds progressive delay and temporary bans. Passkeys check presence and monotonic counters. Webhook signatures use constant-time comparison. | `rullst-auth/tests/integration_test.rs::test_argon2_password_hashing_and_verification_async`, `rullst-security/src/login_guard/capacity_tests.rs::identity_flood_cannot_disable_the_jail_for_a_later_identity`, `rullst-auth/src/auth/passkey/invariant_tests.rs::assertions_require_presence_verification_and_monotonic_counters`, `rullst-capital/tests/stripe_event_envelope.rs::invalid_signatures_are_rejected_and_mock_events_cannot_pass_real_mode_gate` | Cookie-session inventory and refresh remain open (`AUTH-06`). The login jail is process-local. |
| Broken access control and IDOR (CWE-639, CWE-862, CWE-863) | `RbacGuard::authorize_owner_or_role` and its tenant variant, `#[require_role]`, ORM tenant scoping, and the route classification enforced by `cargo rullst audit --idor` on pull requests and releases. | `rullst-security/tests/security_tests.rs::parameterized_route_denies_cross_owner_access_before_the_handler_succeeds`, `rullst-security/src/rbac/guard.rs::tenant_authorization_never_allows_role_bypass`, `rullst/tests/rbac_macro.rs::role_attribute_preserves_arguments_and_denies_before_the_handler`, `rullst-orm/tests/builder_safety_audit_test.rs::tenant_scope_cannot_be_bypassed_by_or_filters`, `cargo-rullst/src/generators/audit_idor.rs::owner_routes_require_the_owner_or_role_guard` | The IDOR scanner is a heuristic; it cannot prove domain authorization correct. |
| Server-side request forgery (CWE-918) | `EgressPolicy` (`rullst-ai/src/ai/egress.rs`) and outgoing webhooks resolve DNS once, pin the addresses and refuse private, special and mixed results. AI provider URLs must use HTTPS, or a literal loopback address for local models. | `rullst-ai/src/ai/egress.rs::strict_policy_rejects_ssrf_destination_forms`, `rullst-ai/src/ai/egress_fetch.rs::fetcher_rejects_private_or_mixed_dns_before_transport`, `rullst-messaging/src/webhooks/transport.rs::private_special_transition_and_mixed_dns_addresses_are_denied` | OIDC discovery requires HTTPS but does not block private addresses. These checks do not replace a network firewall. |
| Unsafe deserialization and resource exhaustion (CWE-502, CWE-400, CWE-770) | Input is parsed into typed serde structures, never into executable objects. `ValidatedJson` and `ValidatedForm` add validation (`rullst-core/src/validation.rs`). The JSON schema guard caps payloads at 2 MiB and nesting at depth 32 and rejects duplicate keys. 42 fuzz targets cover parsers and protocol inputs. | `rullst-security/src/schema_guard/tests.rs::middleware_rejects_deep_and_oversized_json`, `rullst-core/src/validation_tests.rs::extraction_failures_keep_their_413_and_415_status`; targets in [`.github/fuzz-targets.json`](https://github.com/Rullst/Rullst/blob/main/.github/fuzz-targets.json) | Upload routes need an explicit body limit; fuzz campaigns run on demand, not per pull request. |
| Memory-safety errors (CWE-787, CWE-416, CWE-125) | Rust's ownership rules. `unsafe-policy.yml` compiles the workspace with `-D unsafe-code` and allows `unsafe` only in a fixed list of five OS-integration files (Windows ACLs, Unix permissions, the development dynamic-library loader and process telemetry). Four crates use `#![forbid(unsafe_code)]`. | Nightly AddressSanitizer and ThreadSanitizer (`sanitizers.yml`), Miri (`miri.yml`), 22 Kani harnesses (`kani.yml`) | Miri, Kani and fuzzing are started by hand. The allowlisted files hold about 50 `unsafe` blocks, mostly Windows FFI that Linux sanitizers do not exercise. |
| Secrets exposure (CWE-798, CWE-312, CWE-532) | Keys come from the environment or configuration, never from source. `VaultSecret` zeroizes on drop and redacts `Debug`. `FieldEncryptor` and ORM field encryption use AES-256-GCM with key IDs for rotation. The DLP layer masks a bounded set of key formats in responses. TruffleHog scans the repository. | `rullst-security/src/vault/tests.rs::test_vault_secret_redaction`, `rullst-security/src/vault/tests.rs::supports_key_rotation_with_envelope_key_ids`, `rullst-auth/tests/app_key_resolution.rs::documented_placeholder_app_keys_are_rejected`, `rullst-auth/tests/scaffold_app_key.rs::public_env_example_key_cannot_encrypt_or_decrypt_sessions`, `rullst-core/src/validation_tests.rs::display_and_debug_never_contain_the_rejected_values` | DLP recognizes only listed formats (not JWTs or generic API tokens). Applications that used public example keys must rotate them. |
| Weak cryptography and certificate validation (CWE-327, CWE-295) | AES-256-GCM, HMAC-SHA256, Ed25519, ECDSA P-256 and Argon2id. TLS uses rustls only (TLS 1.2 and 1.3); no first-party code disables certificate verification, and remote PostgreSQL requires `sslmode=verify-full`. | `rullst-auth/tests/recovery_contract.rs::postgres_urls_use_the_hardened_connection_policy_before_connecting`; CodeQL and `cargo audit` after every push to `main` | HMAC-SHA1 is used only for TOTP, as RFC 6238 and authenticator apps require. JWTs support HS256 only. |

Security headers add defense in depth: HSTS, a nonce-based CSP,
`X-Frame-Options: DENY`, `nosniff`, COOP, COEP, CORP and a Permissions-Policy
(`rullst-core/src/security/headers.rs`, tested by
`rullst-core/src/security/headers_tests.rs::unmarked_responses_get_every_default`).

## How the argument is kept true

- **Per change:** formatting, Clippy with `-D warnings`, the test suite, the
  threat-model release minimum, `cargo audit`, `cargo deny`, the unsafe policy,
  the zero-panic Clippy gates and the crate-archive reproducibility check run
  on pull requests. CodeQL
  analyzes Rust and JavaScript/TypeScript after every push to `main`, on pull
  requests to the maintained lines and weekly.
- **Coverage:** line coverage must stay at or above 90% (`coverage.yml`, after
  every push to `main`); `main` measured 92.56% on 9 October 2026.
- **Scheduled and on demand:** nightly sanitizers, and manual fuzzing, Miri,
  Kani and mutation campaigns. See the
  [workflow guide](workflows.md).
- **Releases:** the exact tagged SHA must pass admission; artifacts are
  attested with Sigstore and archives are reproduced before publication. See
  [verifying a release](../../SECURITY.md#verifying-a-release) and
  [reproducible crate archives](reproducible-builds.md).

## Limits

- **Application-owned controls.** Session validation, authentication, tenant
  membership and object authorization have slots in the canonical order, but
  the application must mount them. The framework cannot infer those policies.
- **Open abuse cases.** The threat models mark items such as `AUTH-06`
  (cookie-session inventory and refresh) as open. A passing release minimum
  does not close every case.
- **Heuristic layers.** WAF, RASP, DLP and the IDOR scanner reduce risk but can
  be bypassed or miss findings. They do not replace typed parsing,
  parameterized SQL or authorization.
- **Single-instance state.** Login jails, replay caches, rate limits and audit
  buffers are process-local unless a shared backend is configured.
- **Defaults depend on environment.** The Core security baseline applies in
  staging and production. Development builds and custom routers must not be
  exposed publicly.
- **Analysis cadence.** Fuzzing, Miri and Kani run on demand, so a regression
  between campaigns is caught by tests and sanitizers, not by those tools.
- **Review.** One active maintainer reviews changes. AI-assisted review and
  automated checks are not independent human review, and no external audit has
  been completed.
- **Native binaries.** CLI executables are attested but not claimed to be
  bit-for-bit reproducible.
