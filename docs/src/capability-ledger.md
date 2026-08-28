# Capability status and vision decisions

Rullst's ambitious ideas are not deleted when an implementation is incomplete.
They are kept here with an explicit status and an engineering recommendation.
User-facing guides describe what can be used today; this ledger preserves the
larger vision without presenting roadmap work as a production capability.

Status is evaluated against the current version 12 source tree, not against an
old changelog entry or marketing description:

- **Implemented**: a real implementation and focused tests exist; deployment
  responsibilities may still apply.
- **Partial**: useful foundations exist, but the advertised end-to-end contract
  is not complete.
- **Experimental**: available only behind an explicit experimental/simulator
  boundary and carries no production guarantee.
- **Not implemented**: absent or intentionally returns a typed `Unsupported`
  error instead of simulating success.
- **Do not promise**: the absolute claim is not a technically honest product
  contract; a narrower engineering goal may still be worthwhile.

The recommendation column is intentionally opinionated. "Worth implementing"
does not mean "put it in Core": several excellent ideas belong in optional,
independently tested crates. Ideas that additionally need continuous operations,
homologation, or hardware can follow the
[Maybe SaaS incubation strategy](maybesaas.md).

## Architecture and framework contract

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| Runtime-only Core with optional ORM | **Implemented in current hardening (keep validating)** | Bare `rullst-core` now defaults to the HTTP/runtime surface without SQLx/ORM; `orm` and `queue-sqlite` are independent opt-ins. Studio and Nexus request their database features explicitly, while the application umbrella keeps `orm` and `queue-sqlite` in its default set for ergonomic compatibility. |
| One canonical security stack | **Partial (composition risk mitigated; high priority)** | Core and the extended crate now reuse one `CspNonce`, so nested header layers do not invalidate renderer output. Ownership is still split: the Server mounts Core CSRF/WAF/headers/PII while `rullst-security` supplies explicit RASP/DLP/abuse/telemetry layers. Introduce a Core stack contract implemented by the dedicated crate, then deprecate duplicates after parity tests; reversing the dependency directly would create a cycle. |
| Umbrella features for every advertised crate | **Implemented in current hardening (keep tested)** | The umbrella now exposes optional `security` and `iot` features, restores the `oauth`/Connect re-export, forwards SMTP to `rullst-mail/mail-smtp`, and has a focused all-features facade test. The extended suite remains nested under `security::runtime` because its surface intentionally overlaps the Core baseline; `CspNonce` itself is already one shared type. |
| Static dispatch everywhere | **Partial (do not force absolutely)** | Preserve generic/static paths for common cases, but runtime provider registries legitimately need dynamic dispatch. Document each intentional `dyn Trait` boundary instead of pretending it does not exist. |
| A universal environment/configuration policy | **Implemented (keep)** | The validated environment model and fail-closed production startup use exact `RULLST_ENV` → legacy `APP_ENV` → file precedence. New `.env`, Kubernetes, Foundry and billing scaffolds emit/read the canonical name first; retain the alias for existing applications and keep negative configuration tests. |
| Every source file below 500 lines | **Partial (worth continuous refactoring)** | Keep 500 lines as a design target, not a release fiction. Split production modules by responsibility; large test fixtures can use a looser limit. |
| Uniform `#[non_exhaustive]`, fallible builders, and `impl Into<String>` | **Partial (worth completing incrementally)** | Finish this during SemVer-reviewed API work. Mechanical mass changes without compatibility review are not worth the risk. |
| “Zero lock-in” or complete automatic ejection | **Do not promise (migration tooling is worthwhile)** | Standard Axum/SQLx escape hatches and an inspected eject snapshot reduce coupling, but no full-stack framework can guarantee zero migration cost for every optional subsystem. |
| “Zero panic/crash-free runtime” | **Do not promise (the scoped policy is worthwhile)** | Enforce typed errors and reject `panic!`/`unwrap`/`expect` in declared production paths. Dependencies, OOM, aborts, and host failures make an absolute runtime guarantee impossible. |
| “Zero latency/zero allocation everywhere” | **Do not promise (benchmark instead)** | Bounded allocation and latency goals are excellent, but only reproducible per-operation benchmarks should carry numbers. |
| “100% memory-safe/no `unsafe` anywhere” | **Do not promise (an unsafe allowlist is worthwhile)** | Rust dependencies and the development FFI/hot-swap boundary contain reviewed `unsafe`. Keep the allowlist tiny, documented with `SAFETY`, and CI-enforced instead of hiding it. |
| “100% Pure-Rustls for every feature” | **Do not promise (a transport inventory is worthwhile)** | Prefer Rustls-backed first-party clients, then inspect the complete feature-specific lockfile. Transitive and optional stacks make a universal brand claim brittle. |
| Static competitor matrix declaring other frameworks lack features | **Do not maintain without dated sources** | Comparative research can be useful, but ecosystems change quickly and absence claims are easy to get wrong. Prefer a Rullst capability/boundary matrix or a dated, sourced benchmark repository. |
| Framework-wide “production-ready” label | **Do not promise as one boolean** | Publish stability per crate/capability. Routing can be stable while live fiscal or hardware integration remains unavailable. |

## Capital, billing, and fiscal vision

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| Deterministic offline DPS/NFS-e preview | **Implemented (mock only)** | Keep it clearly typed as unsigned and unauthorized. It is useful for UI, schema, and workflow development. |
| Live NFS-e Nacional issuance with PKCS#12, C14N/XMLDSig, mTLS, XSDs, official rejection parsing, and SEFIN homologation | **Not implemented (extraordinary and worth implementing conditionally)** | This is genuinely valuable for the Brazilian ecosystem, but it needs a dedicated fiscal workstream, official test environment, independent XMLDSig validation, certificate lifecycle design, and legal/protocol maintenance. Do not rush it into a generic payment adapter. |
| Alipay RSA2 signing and verification | **Not implemented (worth implementing only with real demand/partner access)** | Use audited RSA primitives, official canonical parameter rules, replay tests, and provider sandbox contract tests. HMAC fixtures must never masquerade as RSA2. |
| Uniform live support across all advertised gateways | **Partial (worth a capability matrix)** | Keep the adapters, but publish method-by-method support and provider contract tests. An adapter name must not imply checkout, subscriptions, payouts, portal, tax, and webhook support all exist. |
| Durable cross-instance webhook idempotency/replay store | **Partial (worth implementing as a pluggable backend)** | In-memory primitives are useful locally. Production multi-instance billing needs a database/Redis uniqueness contract supplied or selected explicitly. |
| Static fee, settlement, and tax promises in framework docs | **Do not promise** | Provider terms and regional availability change. Link to official current terms and document only what the adapter itself implements. |
| “Zero-cost invoicing” | **Do not promise** | Removing an intermediary fee does not remove certificate, accounting, infrastructure, support, or compliance cost. Preserve cost transparency without advertising zero total cost. |

## IoT, edge, and cryptography vision

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| `no_std` telemetry/frame helpers | **Implemented (keep focused)** | This is a credible small foundation. Preserve the lightweight, transport-neutral core. |
| Ed25519-signed OTA manifest, firmware hash, and monotonic verification gate | **Implemented (foundation only)** | Keep negative vectors and anti-rollback tests. The gate is not a firmware installer. |
| Persistent anti-rollback counter, download, flash, bootloader slot selection, rollback, and commit | **Not implemented (worth implementing after choosing target hardware)** | This is essential before calling OTA end-to-end. It belongs in target-specific adapters with power-loss/fault-injection tests, not a pretend generic `Ok(())`. |
| MQTT 5, CoAP, and Sparkplug B transport | **Not implemented (worth a separate transport crate)** | Excellent for the vision, but network runtimes and brokers should not bloat the `no_std` frame crate. Require broker interoperability and parser fuzzing. |
| Hardware HSM/secure-element backends | **Not implemented; simulators are experimental (worth adapter traits, later)** | Implement only against named hardware/PKCS#11 interfaces with device tests. Never create home-grown “HSM-like” hashing and call it secure. |
| NIST ML-KEM/post-quantum encryption | **Not implemented; simulators are experimental (worth later, not home-grown)** | Adopt an audited implementation only when a concrete protocol and threat model justify it. A generic “quantum-safe” badge is not worthwhile. |
| CAN/J1939, LoRaWAN, GPIO/I2C hardware integration | **Not implemented or helper-only (worth separate hardware packages)** | Preserve the idea, but require target boards, interoperability fixtures, and maintainers for each protocol. |
| Embassy async executor integration | **Not implemented (worth implementing after the transport split)** | Valuable for embedded ergonomics once the `no_std` ownership and timer/network abstractions are stable. |
| Actual QEMU/hardware-in-the-loop CI | **Not implemented (worth implementing with real runtime code)** | Merely compiling a target is not QEMU testing. Add it when there is boot/flash behavior to execute and assert. |
| Aerospace/autonomous-vehicle/defense framework claims | **Not implemented (do not place in the web-framework Core)** | The idea is ambitious, but safety-critical systems need independent standards, certification, hardware, and governance. Consider a separate future project only after the IoT foundation is mature. |

## Connect, real-time, queues, storage, and data

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| OAuth2/OIDC/social-login providers | **Implemented (keep as Connect's current identity)** | Continue issuer, redirect, JWKS rotation, offline-fixture, and negative-token contract testing. |
| RabbitMQ, Kafka, and Redis Streams inside `rullst-connect` | **Not implemented (worth implementing, but not in the OAuth crate)** | Create `rullst-messaging` or rename/split Connect before adding brokers. Mixing identity providers and enterprise messaging creates an incoherent crate boundary. |
| WebSocket pub/sub and SSE | **Implemented in Core, not Connect (do not duplicate)** | Re-export through a coherent facade if desired; one runtime implementation is better than competing copies. |
| Bounded Memory/SQLite/Redis queues with recoverable leases | **Implemented (keep hardening)** | Continue backend contract tests, dead-letter observability, graceful shutdown, and multi-worker fault tests. |
| NATS JetStream, SQS/SNS, and GCP Pub/Sub | **Not implemented (worth demand-driven optional adapters)** | Define one queue conformance suite first; add providers only when each can pass the same delivery/lease/idempotency semantics. |
| Authenticated tenant-local storage namespace | **Implemented / bounded in Core** | `TenantStorage` is constructed from a validated `TenantContext`, prefixes keys with an immutable canonical tenant root and reuses traversal/symlink protections. An exact local test stores the same logical key under two authenticated tenants without interference. Academy attachment/media integration, remote bucket policy and cross-backend conformance remain open. |
| Authenticated tenant cache namespace | **Implemented / bounded in Core** | `TenantCache` is constructed from a validated `TenantContext`, validates logical keys, applies an immutable namespace for memory or Redis-backed Core caches and deliberately exposes no global flush. Exact same-key non-interference is tested locally. Academy cache integration, Redis cluster/failover and application-wide conformance remain open. |
| Authenticated tenant realtime namespace | **Implemented / bounded in Core** | `TenantRealtime` and `TenantPresence` bind shared in-process broadcast/presence managers to validated `TenantContext`, validate logical channel/event/identity names, cap payloads at 64 KiB and prove that the same room name in two tenants does not interfere. Room membership authorization, distributed transport/liveness and Academy notification integration remain application/roadmap work. |
| S3 and Cloudflare R2 storage | **Not implemented (worth optional remote-storage crates)** | Useful and commercially relevant. Use official/signed clients, path/key constraints, multipart and retry semantics, and deterministic mocks; do not put fake success in the local facade. |
| Upload admission and quarantine contract | **Implemented / bounded, storage-agnostic** | Core validates bounded in-memory bytes against tenant/name/size/type policy, recognized signatures, MIME/extensions and active-text denial, produces a randomized tenant quarantine key plus SHA-256, and releases only after a clean scanner verdict. The bundled scanner is deterministic and mock-only. Multipart streaming, sandboxed deep parsing/transcoding, S3/R2 movement, archives and a production malware adapter remain open. |
| Image resize pipeline | **Not implemented (worth an optional media crate, not Core)** | Media decoding expands attack surface and binary size. Isolate it with strict size/pixel limits and fuzzing. |
| Transparent SQLite/Turso/database replication | **Not implemented (do not implement generically in Core)** | Integrate vendor-specific replication clients or sidecars. A generic timer that prints “syncing” cannot provide consistency semantics. |
| Immutable/zero-copy distributed ledger engine | **Not implemented (interesting, but lower priority)** | First define the exact consistency, persistence, recovery, and audit use case. The current HMAC audit chain is not a distributed immutable ledger. |

## Security, authentication, Studio, and Nexus

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| Versioned AES-256-GCM field encryption | **Implemented (keep)** | Keep envelope versioning, AAD, random nonces, key rotation, and round-trip/negative tests. Key custody remains external. |
| Strict CSP nonce/header baseline | **Implemented (baseline)** | Worth keeping. Tune it per application and expose nonces to rendering; never promise a universal third-party A+ score. |
| “OWASP A+ guaranteed” | **Do not promise** | A scanner grade depends on the final page, proxy, cookies, TLS, and deployment. The worthwhile goal is a strict tested baseline, not a badge guarantee. |
| Bounded body-aware WAF/RASP and DLP | **Implemented (defense-in-depth)** | Keep content-type, streaming, compression, overflow, and header-consistency tests. Never position heuristics as a replacement for parsers, binds, validation, or authorization. |
| Distributed rate limiting | **Implemented foundation / external proof pending** | `rullst-security/redis-rate-limit` uses one atomic Redis script, namespace validation, hashed client keys, TTLs and typed fail-closed errors. Empty/`mock_*` configuration is explicitly process-local and `require_distributed()` rejects it. Real Redis cross-instance, eviction/failover and trusted-proxy deployment tests remain release work; the legacy no-argument selector stays `Unsupported`. |
| Durable tamper-evident audit storage | **Partial (worth implementing as a sink interface)** | Canonical HMAC sequencing exists. Durable append-only storage, key protection, retention, and independent verification remain application/integration work. |
| Versioned local security-event envelope | **Implemented and bounded (v1)** | `LiveSecurityEvent` has a frozen six-field v1 contract, a packaged JSON Schema, normalized identifiers/IPs/timestamps, a 2 KiB UTF-8 detail limit, and CEF field escaping. It remains a process-local telemetry envelope, not durable SIEM delivery. |
| Full normative WebAuthn server | **Partial (worth completing before declaring stable)** | The implementation now checks many ceremony invariants, but an audited library or a full conformance suite is preferable to indefinite custom protocol ownership. |
| TOTP recovery codes | **Implemented cryptographic foundation / workflow pending** | Security generates subject-bound 80-bit codes, returns plaintext only at enrollment, serializes salted HMAC verifiers, compares in constant time and removes a consumed verifier. The application must encrypt the TOTP secret, persist verifier consumption atomically, rate-limit attempts and own enrollment/recovery UX and audit. |
| First-class application JWT service in `rullst-auth` | **Implemented foundation / shared store pending** | The opt-in `jwt` feature centralizes issuer/audience, required versioned claims, bounded TTL/scopes, strong HS256 keys, `kid` rotation and a static revocation-store contract. Production verification rejects the bundled process-local store. A maintained shared durable adapter, device inventory and refresh-token workflow remain application/roadmap work. |
| Nexus open-by-default admin | **Removed; hardened Nexus is implemented** | Keep fail-closed construction, TLS boundary, role/field policy, ownership integration, and rate limits. Production readiness still depends on the host application identity model. |
| Autonomous AI admin that can mutate production data | **Not implemented as a safe contract (do not enable by default)** | A read-only, explainable assistant is worthwhile. Mutations need explicit human approval, scoped capabilities, audit records, dry-run previews, and rollback. |
| Studio with real telemetry | **Implemented with unavailable states** | Linux and Windows expose delta-based process CPU alongside RSS, Tokio and span probes; KPI cards poll the local JSON source. Keep Studio explicit and local by default, and do not fabricate numbers for unsupported or missing sources. |
| Studio automatically stripped from every release with zero overhead | **Do not promise** | Feature selection and route mounting determine inclusion. Explicit compile features are clearer than relying on a universal debug/release assumption. |
| Threat Radar with external reputation and verified audit feeds | **Partial (worth pluggable connectors)** | Local counters exist. External intelligence, durable audit verification, and SIEM delivery should appear only when a source is connected and healthy. |
| “100% OWASP coverage”, “tamper-proof”, or “zero-leak DLP” | **Do not promise** | These absolutes are not provable framework properties. Preserve the controls and publish exact threat-model/test scope. |
| Automatic SOC 2/ISO/FedRAMP certification | **Do not implement as a PASS generator** | The evidence exporter is worthwhile; certification evaluates the whole organization and deployment. Emit `PASS`, `FAIL`, `SKIPPED`, or `NOT_EVALUATED` from real checks only. |

## AI and mail

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| DeepSeek provider | **Implemented** | Keep it in the same provider contract and offline mock suite as the other cloud adapters. |
| Mandatory prompt-injection/PII guardrail pipeline | **Implemented in the high-level client and built-in provider transports** | The versioned deterministic offline corpus fixes implemented injection/jailbreak/PII regressions across built-in transports. Custom low-level providers remain an explicit extension boundary; heuristics and this corpus cannot prove a prompt safe or replace adaptive/live-model evals. |
| Offline mocks for chat, vision, embeddings, and mail transports | **Implemented** | Keep mocks deterministic and selected only by explicit empty/`mock_*` credentials; live endpoints must never silently fall back. |
| Native JSON Schema structured output on every LLM | **Partial (worth capability-typed support)** | Separate parseable JSON from provider-enforced schema. Return `UnsupportedCapability` when native enforcement is unavailable. |
| Machine-readable AI provider capabilities | **Implemented / bounded** | `AiProvider::capabilities()` and `AiClient::capabilities()` report transport support for text, chat, embeddings, vision, JSON/schema, streaming, tools, timeout, retry, and explicit cancellation. The public matrix states model-dependent and unsupported boundaries. |
| Guarded local AI tool dispatch | **Implemented / bounded** | `ToolRegistry::execute` requires an exact allowlist, principal authorization, closed bounded JSON, a call budget and audit sink. Destructive/financial approvals are one-use and payload-bound. Provider-native tool calling, approver authentication, domain authorization and a durable production sink remain application/roadmap boundaries. |
| Strict AI egress fetcher | **Implemented / opt-in bounded transport** | `EgressPolicy::strict()` denies all hosts until an exact allowlist is configured. `EgressFetcher` validates HTTPS URL/port and every DNS answer, pins all accepted addresses in a proxy-free client, verifies the connected peer, revalidates manual redirects and bounds deadline/declared and streamed bytes. It does not wrap arbitrary application/provider clients automatically; tenant-aware destination authorization, response schema/content handling and a deterministic successful live-origin contract remain integration work. |
| “Any local LLM over any HTTP API” | **Do not promise** | Supporting explicit Ollama/OpenAI-compatible protocols is worthwhile. Arbitrary HTTP APIs have incompatible auth, streaming, schema, and error semantics. |
| Autonomous AI self-healing/DevOps changes | **Partial recommendation tooling (do not auto-apply by default)** | Diagnostics and patches can be valuable, but infrastructure/code mutation needs review, capability scopes, preview, audit, and rollback. |
| Tenant-aware secure mail pipeline and expiring tracking tokens | **Implemented** | Continue provider contract tests and ensure tenant identity comes from authenticated application state. |
| Mailgun, Brevo, MailerSend, Plunk, and Scaleway transports | **Not implemented (worth demand-driven adapters)** | Add only with maintainers and the shared offline/live transport conformance suite; provider count alone is not product quality. |
| “Air-gapped/zero-leak AI” | **Do not promise automatically** | Local endpoints can avoid cloud LLM calls, but host networking, logging, model runtime, and telemetry determine the real data boundary. |

## CLI, generated applications, and release engineering

| Capability or former claim | Current status | Recommendation and reason |
|---|---|---|
| Stable blueprint IDs and corrected Nix/Buildah/Island/Resource flags | **Implemented in current hardening** | Keep snapshot tests because generated CLI identifiers are public compatibility surface. |
| LMS/Academy starter domain foundation | **Implemented / bounded scaffold** | Generates curriculum, enrollment/progress, immutable course publication/pins/rollback, availability/prerequisites, quizzes, text assignments/rubrics/grading/correction, completion/certificates, roles, leaderboard, automation/outbox/worker/scheduler and notifications. Its typed activity-client boundary defaults to zero-bundle SSR/HTMX; opt-in Canvas/Wasm manifests require same-origin paths, canonical digest and a bounded artifact while server results remain authoritative. It now also generates schools, active/default memberships, one school scope per course, cohorts and temporal entitlements; authentication treats `X-School-ID` only as a selector against persisted membership and binds the result to `UserContext`. Learning, publication, assignment grading, score correction and leaderboard reads, completion/certificate mutation, roles and scheduled publication check the active school. Outbox records, derived automation rules/executions/achievements and notification data/preferences preserve `school_id`. The leaderboard has a bounded, payload-validated `TenantCache` projection invalidated by authoritative score/quiz/correction commits; its source of truth remains the database. Unsuppressed in-app notification commits are additionally projected best-effort to an authenticated tenant/user room through in-process `TenantRealtime`; the database remains authoritative. The materialized SQLite journey proves these vertical contracts plus arbitrary/ambiguous selection denial and cross-school non-interference for enrollment, lesson, leaderboard/cache, same-user notifications/preferences, foreign automation rule, privacy consent, publication, rollback, grade, score, certificate, scheduler and role revocation. This is not complete multi-tenancy: attachments/media, other caches, Core storage integration, search, metrics, exports, Nexus, distributed cache/realtime/failover and PostgreSQL/MySQL isolation remain open, as do portable signed credentials, full relational snapshots, step-up, visual authoring, Mail/push, browser E2E and the separately operated Academy. |
| Selectable LMS foundation profile | **Implemented / bounded; broader matrix partial** | `--lms-modules auth,learning` emits fewer than 30 files with auth, catalog, enrollment, protected lesson playback and monotonic/idempotent progress; it records the selection and a materialized project passes offline Cargo tests without quiz/gamification/automation/notification files. The complete starter remains default. Other combinations and foundation hot reload fail explicitly until their dependencies are genuinely detached; realtime and billing are not claimed as selectable modules yet. |
| Academy privacy lifecycle foundation | **Implemented / bounded scaffold** | Generates minimized age-band policy without birth date, school-scoped versioned retention, purpose-bound guardian consent/revocation and idempotent export/delete request records. SQLite evidence denies a foreign-school guardian and fails minors closed without active consent. It does not fulfill requests, anonymize cross-table state, run retention jobs, verify a legal guardian, guarantee PII-free application logs or establish legal compliance. |
| mdBook documentation hub | **Implemented in current hardening** | Keep mdBook build in CI pages deployment; retired legacy internal SSG in favor of industry standard mdBook. |
| Parameterized browser RPC through `server_function` | **Partial (worth a versioned protocol)** | Native expansion now preserves the complete Rust signature and body, and invalid macro forms have compile diagnostics. The Wasm helper still has no argument payload or matching server-side `/api/rpc` registration, so end-to-end parameter transport must not be claimed until serialization, routing, auth, errors, and browser tests exist. |
| Every generator/blueprint combination compiled in a tempdir | **Partial (highest remaining CLI priority)** | A structural matrix validates 270 combinations across six blueprints, materializes every blueprint, parses extracted templates, and inventories every public command. Eight representative source-tree projects pass real offline Cargo checks/tests across all six blueprints plus the detached LMS foundation, three ORM modes, five frontends, API, database, hot reload and release; a separate packaged-distribution gate installs the extracted CLI and compiles all six default blueprints without monorepo paths. CI still needs to prove this on the final RC SHA. Expand `fmt/smoke` only where it adds a distinct contract and do not treat provider/deploy commands as offline tests. |
| Generated Auth/JWT/Billing production defaults | **Implemented for the audited regressions; still application code** | Async Argon2, issuer/audience/strong-secret checks, authenticated billing identity, CORS allowlists, and signed webhooks are foundations; generated apps still need deployment review. |
| Fully compliant automatic OpenAPI | **Partial (do not claim completeness)** | A syntax-derived draft is useful. Full fidelity needs a typed schema/route contract and validation, not regex confidence. |
| TypeScript SDK that eliminates contract breaks | **Partial (worth contract tests)** | Generation reduces duplication; add serialization golden tests and API compatibility tests instead of promising elimination of drift. |
| Total framework ejection | **Partial (keep as migration aid)** | Generate an inspectable entry point, list remaining Rullst dependencies, and run `cargo check`. Do not promise automatic removal of every subsystem. |
| Opt-in Git pre-commit/commit-message installer | **Partial (useful DX, CI remains authoritative)** | `hook:install` writes format/Clippy/IDOR and Conventional Commit hooks. Add safe handling for existing hooks, idempotency/permission tests, and clear failure on a missing Git worktree before presenting it as a hardened installer. |
| One-click, zero-downtime deployment | **Partial (guided deploy is worthwhile)** | Manifest/SSH helpers are useful, but availability, secrets, migrations, rollback, DNS, and cloud credentials remain operator concerns. |
| IDOR scanner that proves authorization | **Heuristic (keep as a warning tool)** | AST patterns can find omissions, not prove ownership semantics. Pair findings with route-level negative tests. |
| Compliance report that prints unconditional PASS | **Removed; evidence-oriented report implemented** | This is the correct direction. Keep raw evidence, tool versions, skipped states, and commit digests. |
| Release packaging in dependency order with preflight gates | **Implemented in workflow; unreleased** | Keep package-all-before-publish and topological publishing. The tag job now bundles Cargo metadata/lockfile, governed Cargo Audit output, CycloneDX, bounded compliance evidence, policies, context and checksums, then attests the bundle and packages. Do not call version 12 released until a matching green tag, crates and notes exist. |
| SLSA Level 3 certification | **Not established (do not claim)** | Build provenance is worth keeping. Pursue a named SLSA level only after every requirement is independently evaluated for the actual release platform. |
| RustSec/OSV exception governance | **Implemented** | Keep only unavoidable exceptions, each with owner, compensating control, and expiry; patched findings must block regressions again. |
| Full Kani/Miri/mutation proof of the framework | **Not implemented (do not promise)** | Scoped harnesses are valuable. Promote stable, deterministic subsets to blocking gates and label exploratory jobs honestly. |
| “100% test coverage” | **Do not promise (measured coverage is worthwhile)** | Report the exact commit, features, targets, excluded/generated code, and coverage tool output. Line coverage is not behavioral completeness. |

## Suggested sequencing

1. **Finish version 12 hardening:** complete the generated-project matrix,
   workspace tests/Clippy/format, and release evidence.
2. **Architecture cycle:** preserve the new Core/ORM and umbrella feature gates,
   consolidate Security, and define `rullst-messaging`/remote-storage boundaries.
3. **Demand-backed integrations:** durable rate limits/idempotency, S3/R2, and
   selected message brokers with shared conformance suites.
4. **High-assurance programs:** WebAuthn conformance and, if Brazil is a core
   market, a dedicated officially homologated NFS-e implementation.
5. **Hardware program:** only then expand OTA into real flash/boot lifecycle,
   MQTT/Embassy, named HSM devices, and audited PQC protocols.

This ordering keeps the extraordinary vision while making the stable surface
smaller, testable, and trustworthy.
