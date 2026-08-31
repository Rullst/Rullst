# Rullst Master Roadmap 🗺️
### *"The Path to the Ultimate Full-Stack Rust Framework" — an aspiration, not a guarantee*

Rullst's ambition is an asset. This roadmap preserves that ambition while
separating what exists today from what is only a prototype, a research program,
or a vision. An idea is never deleted merely because it is unfinished.

Our philosophy: **"Security, Developer Experience and Performance, Architected
for Humans and AI."**

> **Single roadmap source:** `ROADMAP.md` is canonical.
> `docs/src/roadmap.md` embeds it directly in mdBook instead of maintaining a
> divergent copy. The deeper evidence and decision record is
> `docs/src/capability-ledger.md`. The release gates and executable checklist
> for the next major version live in [`docs/src/v12.md`](docs/src/v12.md).

## Status language

- `[x] Implemented`: a bounded, testable implementation exists. This never means
  that every imaginable provider or production environment is covered.
- `[~] Partial`: useful foundations exist, and the parenthetical says whether the
  remaining work is worthwhile and why.
- `[ ] Not implemented`: the idea is preserved, and the parenthetical says
  whether it is worth pursuing and under which conditions.
- `[!] Do not promise`: the absolute wording cannot be an honest framework
  guarantee; a narrower measurable goal is retained when useful.

Target windows are planning intentions, not release guarantees. Promotion to
`[x]` requires code, focused tests, truthful documentation, and the release gates
at the end of this document.

## Audit of the detailed crate roadmaps

The per-crate roadmaps are intentionally preserved as detailed design backlogs.
Some predate this status policy, so an old `[x]` can record the original author's
milestone claim rather than today's verified end-to-end contract. This table is
the current interpretation; the [capability ledger](docs/src/capability-ledger.md)
contains the evidence boundary and recommendation for the highest-risk claims.

| Detailed roadmap | What is verifiably implemented now | Partial, experimental, or not implemented |
| :--- | :--- | :--- |
| [`rullst-ai`](rullst-ai/ROADMAP.md) | Guarded high-level client; OpenAI/Gemini/Anthropic/DeepSeek/Ollama adapters; deterministic offline paths and eval corpus; JSON/schema distinction; guarded local tools; bounded tenant-aware audited RAG orchestration and process-local cosine retrieval. | Streaming/cancellation, derived JSON Schema, provider-native authorized tool loop, durable memory/ORM hooks, first-party external retriever adapters, and live/adaptive model evals are not implemented. |
| [`rullst-auth`](rullst-auth/ROADMAP.md) | Argon2/local sessions, RBAC middleware, declarative `Gate`, OAuth/OIDC re-exports, and a substantial custom ES256 passkey foundation. | WebAuthn is partial until normative conformance; first-class application JWT, TOTP with recovery codes inside Auth, magic links, and device/session management are not implemented. They are worthwhile, with WebAuthn first. |
| [`rullst-capital`](rullst-capital/ROADMAP.md) | Provider trait/adapters, explicit offline mocks, canonical fail-closed webhook verification with Axum/Actix adapters, provider-specific coupon/trial contracts, shared idempotent team/workspace quotas with a four-dialect SQL store, billing scaffolding, analytics, and bounded NFS-e preparation. | Live method coverage varies by gateway; durable cross-instance webhook idempotency, Alipay RSA2, full tax/proration contracts, and homologated live NFS-e are not implemented. NFS-e is extraordinary and worthwhile only as a dedicated homologation program. |
| [`rullst-connect`](rullst-connect/ROADMAP.md) | OAuth2/OIDC/social providers, a bounded tower-sessions state/PKCE/nonce lifecycle, fallible credential modes, pluggable HTTP client, refresh/revoke foundations, discovery/JWKS validation, retry, mocks, and Axum/Actix callback extraction. | Some checked DX/provider conveniences are narrower than their wording; SAML/SCIM/DPoP/JWE/mTLS/risk ML and all Phase 9 message queues are not implemented. Messaging is worthwhile in a separate crate, not the OAuth-focused Connect. |
| [`rullst-iot`](rullst-iot/ROADMAP.md) | `no_std` frames/telemetry and the Ed25519 OTA manifest/hash/target/version/anti-rollback verification gate. | Download, durable counter, flash/boot/rollback, MQTT/CoAP/LoRaWAN, real hardware, HSM and PQC are not implemented; deterministic `Simulated*` types are experimental fixtures only. Keep the vision, but require target hardware and interoperability programs. |
| [`rullst-mail`](rullst-mail/ROADMAP.md) | Core REST/SMTP/log/memory/mock drivers, failover, attachments/scheduling foundations, mandatory security/deliverability pipeline, deterministic mocks, tenant resolution, tracking tokens, factories and background worker integration. | A checked item does not prove universal provider/deliverability behavior. Compile-time mailables/CSS inlining, inbound MIME, AI dunning, DMARC/DKIM/S-MIME, Studio Mail Radar and extra gateways are not implemented; add providers only with a shared contract suite. |
| [`rullst-nexus`](rullst-nexus/ROADMAP.md) | Fail-closed authenticated admin construction, compile-tested `Nexus` derive, server-side CRUD/search/pagination/sorting, explicit typed widgets, selected-record delete/deactivate, threat radar and AI assistant surfaces. | Enum variants/multiline intent require explicit metadata; tenant ownership and durable audit remain host contracts. Custom dashboard injection and a visual SQL builder are not implemented; AI/data mutation remains host-policy-bound. |
| [`rullst-orm`](rullst-orm/ROADMAP.md) | SQLx pools/dialects, Active Record/repository/query/schema foundations, fail-closed tenant scopes, strict DB modes, transactions, relations/soft deletes, audit/privacy, typed Turso primary, bounded MongoDB/DuckDB/SurrealDB adapters, Qdrant vectors and Redis native structures. | Several historical `[x]` entries remain partial or absent: transparent edge replication, universal external-search durability, autonomous schema/index changes, automatic graph traversal, Wasm drivers and PQC. The 45 unique claims are now individually classified in `v12.md`. |
| [`rullst-security`](rullst-security/ROADMAP.md) | Bounded honeypot, sanitizer/CSP, RBAC, HMAC audit chain, RASP/DLP, AES-GCM vault, headers, applied Login Jail tarpit, TOTP with SVG QR, CSWSH origin policy, strict JSON/log guards, file-backed SRI, CEF formatting, timing/prompt filters and fail-closed CLI evidence/SBOM/doctor tools. | “Autonomous”, live reputation/SIEM delivery, A+ guarantees, zero-leak/zero-latency, certification and total OWASP/memory-safety claims are not established. CSRF WebSocket tickets/frame crypto, distributed rate limits/audit sinks, KMS/rotation, adaptive WAF, SQL firewall and all PQC/kernel/Wasm containment items remain partial or absent. |
| [`rullst-studio`](rullst-studio/ROADMAP.md) | Read/filter SQLx browser, supplied-OpenAPI playground, bounded queue snapshot with opt-in pruned SQLite completion history, relational ER diagram, DB-backed flag toggles with same-process cache invalidation, redacted environment/typed-config view and local telemetry with unavailable states. | Browser writes, automatic route-to-OpenAPI inference, secret-bearing request capture, cross-process flag invalidation, Redis queue inspection, N+1 profiling and Cache/Redis inspection remain roadmap work. |

This audit does not downgrade ambitious ideas merely because they are difficult.
Capabilities that require continuous operations, homologation, hardware, or a
separate release lifecycle can follow the
[Maybe SaaS incubation strategy](docs/src/maybesaas.md) instead of being forced
into the framework core.
It prevents a checkbox from becoming a production promise before the necessary
code, tests, provider/hardware environment, and operational semantics exist.

## Executive milestone tracker

| ID | Pillar and capability | Honest status and recommendation | Target window |
| :---: | :--- | :--- | :---: |
| **M1** | DX: CLI empowerment and `make:*` generators | `[~] Partial` *(worth finishing — the commands exist, but every generator/blueprint combination still needs a compiling temp-project matrix)* | v12 hardening |
| **M2** | DX: fast linkers, build tuning, and the sub-100ms aspiration | `[~] Partial` *(worth benchmarking — linker/build settings are useful, but sub-100ms depends on the machine and dependency graph and must not be guaranteed)* | Continuous |
| **M3** | DX: Axum/SQLx escape hatches, granular features, proc-macro diagnostics, and ejection | `[~] Partial` *(worth improving as migration tooling — bare Core is now runtime-only, ORM/SQLite queues are explicit features, and the umbrella maps them; universal “zero lock-in” is still not worth promising because optional subsystems carry migration cost)* | Next SemVer cycle |
| **M4** | DX: `make:resource` and Ignition-style error console | `[x] Implemented (scoped)` — resource scaffolding and a local developer error console exist; autonomous mutation is evaluated separately in M37 | v12 hardening |
| **M5** | DX: documentation hub (mdBook), OpenAPI, and AST TypeScript generation | `[~] Partial` *(worth finishing — generators exist, but generated-project and serialization contract tests are still needed; AST inference is not a complete API contract)* | v12.1 |
| **M6** | ORM: Active Record, repository pattern, seeders, and Turso/libSQL vision | `[~] Partial` *(SQLx foundations and the bounded Turso-primary Hrana transport/matrix exist; relation/hook/auto-diff parity and transparent synchronization do not)* | v12.x |
| **M7** | Edge/data: portable Wasm request/response runtime, distributed data, and autonomous upgrades | `[~] Partial` *(worth the portable edge runtime; distributed replication should use vendor-specific semantics, and autonomous upgrades are not worth enabling without signed artifacts, rollback, and operator approval)* | v13 research |
| **M8** | ORM/AI: intent-based modeling and self-optimizing production indexes | `[ ] Not implemented` *(worth an advisory, explain-and-approve implementation — automatic production DDL without review is not worth the operational risk)* | v12.1 research |
| **M9** | Auth: local auth, OAuth/OIDC, TOTP, passkeys, and WebAuthn | `[~] Partial` *(worth completing at high priority — useful auth pieces exist, but normative WebAuthn conformance and a first-class application JWT policy remain incomplete)* | v12.1 |
| **M10** | Security utilities: mail, DTO validation, rate limiting, and Shield | `[~] Partial` *(worth completing — local controls and mail transports exist, while distributed rate limiting and some provider invariants require real backends and conformance tests)* | v12.1 |
| **M11** | SaaS: hardened Nexus, Omni vision, billing, and entitlements | `[~] Partial` *(worth building in bounded modules — Nexus and billing foundations exist, but Omni, uniform live gateway coverage, and declarative entitlements are not complete)* | v12.1+ |
| **M12** | Defense in depth: RASP/WAF, Vault, honeypots, HMAC audit, secure headers, Login Jail, DLP, TOTP, fingerprinting, CLI inspection, and Threat Radar | `[~] Partial` *(worth continuous hardening — concrete controls exist, but they do not prove universal OWASP coverage, zero leakage, external intelligence, or certification)* | Continuous |
| **M13** | Post-quantum web architecture, `rullst-quantum`, NIST PQC, and sandboxed Wasm plugins | `[ ] Not implemented` *(worth later only for a concrete protocol and threat model, using audited primitives; home-grown “quantum-safe” crypto is not worth implementing)* | v13 research |
| **M14** | Frontend: HTMX-first SSR and Leptos/Dioxus interoperability | `[~] Partial` *(worth improving — HTMX/HTML support is real, while the current Leptos/Dioxus types are compatibility wrappers rather than full framework integrations; “zero bundle” is a selectable architecture, not a universal guarantee)* | v12.1 |
| **M15** | Runtime: queues, cache, scheduler, and multi-stage Docker | `[~] Partial` *(worth finishing through backend contracts — bounded Memory/SQLite/Redis foundations exist; RabbitMQ, Kafka, Redis Streams, NATS, SQS/SNS, and GCP Pub/Sub are not implemented)* | v12.1+ |
| **M16** | Wasm islands and `#[client_component]` | `[~] Partial` *(worth finishing — macros now preserve typed function signatures and generated native examples compile, but argument serialization, server-side RPC registration, browser hydration, packaging, and Wasm end-to-end compatibility still need a versioned protocol and proof)* | v12.1 |
| **M17** | Real-time, object storage, media, and `cargo rullst pkg` | `[~] Partial` *(worth modular expansion — WebSocket/SSE and local storage foundations exist; S3/R2, image processing, and a production package-registry contract do not)* | v12.1+ |
| **M18** | LiveView-style server-driven UI and `make:live` | `[~] Partial` *(worth hardening — a WebSocket component loop exists, but auth, reconnect, backpressure, diff semantics, and browser E2E coverage remain)* | v12.1 |
| **M19** | AI/telemetry: Radar, agent tool schemas, spans, and Prometheus `/metrics` | `[x] Implemented (bounded)` — local telemetry and export surfaces exist; unavailable sources must remain unavailable rather than becoming invented values | v12 hardening |
| **M20** | Persistence: zero-copy event streaming and immutable ledger engine | `[ ] Not implemented` *(interesting but lower priority — worth implementing only after defining persistence, consistency, recovery, and verification semantics; the HMAC audit chain is not a distributed ledger)* | v12.1 research |
| **M21** | Omni-frontend protocol and mobile hypermedia bridge | `[ ] Not implemented` *(worth pursuing only with a real mobile client and versioned protocol contract; a universal frontend claim without interoperability tests is not worthwhile)* | v12.1 research |
| **M22** | Agentic DevOps and autonomous infrastructure provisioning | `[~] Partial` *(worth keeping as human-reviewed recommendations — telemetry advice exists; unattended infrastructure mutation is not worth enabling by default without preview, scoped credentials, audit, rollback, and policy)* | v13 |
| **M23** | Polymorphic core and auto-healing runtime/database | `[~] Partial` *(worth keeping as diagnostics — a schema-error suggestion helper exists; automatic code/schema mutation is not worth enabling by default without validated plans, approval, and rollback)* | v13 |
| **M24** | Embedded IoT: `no_std` frames and an Ed25519 OTA manifest gate | `[~] Partial` *(worth expanding after target selection — the verification foundation exists; download, persistent anti-rollback, flashing, boot slots, HSM/PQC, and transport interoperability do not)* | v12 foundation / v13 integrations |
| **M25** | Async embedded IoT with Embassy | `[ ] Not implemented` *(worth implementing after transport and hardware traits stabilize, because executor integration before those boundaries would create churn)* | v12.1+ |
| **M26** | Guided PaaS/VPS deploy for Fly, Railway, Render, and Caddy | `[~] Partial` *(worth hardening — scaffolding and helpers exist, but “one click” and zero downtime are not framework guarantees because credentials, DNS, migrations, health, and rollback remain operator concerns)* | v12.1 |
| **M27** | Kubernetes manifest scaffolding and `/health`/`/ready` probes | `[x] Implemented (scaffolding scope)` — generated manifests remain deployment inputs that operators must review | v12 hardening |
| **M28** | Compile-time DI and `Inject<T>` | `[x] Implemented (foundation)` — the typed container exists; “zero cost” remains a benchmarkable goal rather than a guarantee | v12 hardening |
| **M29** | Scalar playground at `/docs` and OpenAPI generation | `[~] Partial` *(worth finishing — the UI/router/generator exist, but full OpenAPI fidelity requires typed schemas and validation rather than syntax inference)* | v12.1 |
| **M30** | Tonic/gRPC and Protobuf scaffolding | `[~] Partial` *(worth finishing — `make:grpc` emits a starting service, but a distinct supported `rullst-grpc` crate and generated-project conformance matrix do not yet exist)* | v12.1 |
| **M31** | Aerospace, autonomous vehicles, robotics, and defense (`rullst-orbit` / `rullst-auto`) | `[ ] Not implemented` *(extraordinary, but not worth placing inside the web-framework Core; consider a separate safety-critical project only after hardware, standards, certification, and governance exist)* | Separate future program |
| **M32** | Architecture: first-class Axum/Tower escape hatches and precise proc-macro diagnostics | `[x] Implemented (bounded)` — router conversion/interoperability and `syn::Error` diagnostics exist; continue compatibility tests | v12 hardening |
| **M33** | SaaS: `#[rullst::gate]` and `GateGuard` declarative entitlements | `[ ] Not implemented` *(worth implementing for SaaS only if enforcement is server-side, tenant-bound, auditable, and independent of hidden UI controls)* | v12.1 |
| **M34** | Multi-target SDK generator for TypeScript, React, Dart, and Swift | `[ ] Not implemented` *(worth implementing from one canonical typed API schema; multiplying AST heuristics across languages is not worth the drift)* | v12.1+ |
| **M35** | Distributed OpenTelemetry trace-waterfall visualizer in Studio | `[~] Partial` *(worth implementing — Studio has trace surfaces, but a distributed OTel waterfall needs real ingestion, clock/skew handling, sampling metadata, and unavailable states)* | v12.1+ |
| **M36** | Natural-language-to-SQL Studio data copilot | `[ ] Not implemented` *(worth a read-only, explainable assistant with schema allowlists, parameterization, preview, limits, and approval; autonomous production writes are not worth the risk)* | v12.1 research |
| **M37** | One-click AI error-console autofix | `[~] Partial` *(worth retaining as a local, reviewable patch workflow — an autofix endpoint exists, but autonomous edits need diff preview, workspace confinement, audit, tests, and rollback)* | v12.1 |
| **M38** | In-memory/local-NVMe SQLite read replicas with background synchronization | `[ ] Not implemented` *(worth vendor-specific adapters when demanded; generic “transparent replication” is not worth claiming because consistency and failover semantics belong to the selected database)* | v13 research |

## AI-native vision, without absolutes

The original goal of becoming the first **AI-Native Web Framework** is preserved
as a design ambition, not a historically provable “first” claim.

1. **“Zero Runtime Magic, Pure Compilation”:** derives, typed routes, and compiler
   diagnostics can make AI-assisted changes easier to inspect. *(Partial and
   worth pursuing as an architectural preference; literal zero magic, “zero
   hallucinations,” and instant correction are not promises any framework can
   make.)*
2. **Context-rich scaffolding:** generated projects should receive a maintained
   `AGENTS.md`/AI ruleset describing the actual selected blueprint. *(Partial and
   worth implementing; do not document `.ai-rules` or `.cursorrules` as generated
   until the generator and snapshots prove it.)*
3. **Structured system discovery:** a versioned schema should expose active
   routes, controllers, models, policies, and source locations. *(Partial and
   worth completing; the CLI can inspect `rullst-schema.json`, but generation and
   freshness must become an end-to-end contract.)*

## Preserved extraordinary capability decisions

These items were previously easy to mistake for shipped functionality. They are
kept deliberately, with the opinion requested for each gap. The capability ledger
contains the more detailed evidence and acceptance boundaries.

### Architecture and product-contract ambitions

- **Runtime-only Core with optional ORM** *(implemented in current hardening —
  bare Core no longer selects SQLx/ORM, `orm` and `queue-sqlite` are independent,
  Studio/Nexus opt in explicitly, and the application umbrella retains ergonomic
  database defaults).*
- **One canonical security stack** *(partial — worth treating as high priority;
  keep policy/middleware in `rullst-security` and only minimal bootstrap contracts
  in Core so WAF, headers, and telemetry cannot drift).*
- **Static dispatch everywhere** *(partial — not worth forcing absolutely;
  generic fast paths are valuable, but runtime-selected providers legitimately
  need a documented dynamic-dispatch boundary).*
- **Every production source file below 500 lines** *(partial — worth continuous
  responsibility-based refactoring, but it is a design target rather than a
  release claim and large test fixtures may need a looser limit).*
- **Uniform `#[non_exhaustive]`, fallible builders, and `impl Into<String>`**
  *(partial — worth completing incrementally under SemVer review; a mechanical
  mass rewrite is not worth breaking consumers).*
- **Zero lock-in, zero panic/crash, zero latency/allocation, 100% memory safety,
  and 100% Pure-Rustls** *(`[!] Do not promise as absolutes` — migration tools,
  scoped zero-panic linting, benchmarks, a tiny documented unsafe allowlist, and a
  feature-specific transport inventory are all worth maintaining).*
- **Framework-wide “production-ready” badge** *(`[!] Do not promise as one
  boolean` — worth publishing stability per crate/capability because routing can
  be stable while live fiscal and hardware integrations remain unavailable).*
- **Static competitor matrix claiming other frameworks lack capabilities**
  *(`[!] Do not maintain without dated sources` — comparative research and a
  reproducible benchmark repository are worthwhile; timeless absence claims are
  not).*

### Security, identity, and compliance

- **Full WebAuthn/FIDO2 conformance** *(partial — absolutely worth completing
  before a stable passkey claim, preferably with an audited library or normative
  conformance suite).*
- **Zero-downtime key rotation and Cloud KMS** *(not implemented end to end —
  worth implementing through provider-neutral envelope/key-version contracts and
  named KMS adapters, not by embedding custody in the framework).*
- **Adaptive WAF and eBPF kernel threat containment** *(not implemented — worth
  research only as opt-in, platform-specific defense in depth; not worth making a
  portability or complete-protection promise).*
- **Anti-timing user-enumeration guard and Prompt Shield v2** *(implemented
  foundations — worth keeping and testing, but timing equalization and heuristic
  prompt filtering cannot guarantee elimination of every side channel or
  injection technique).*
- **External reputation feeds, verified audit feeds, and SIEM delivery for Threat
  Radar** *(partial — worth pluggable connectors; never render a source as healthy
  or verified unless it is connected and current).*
- **Studio automatically stripped from every release at zero cost** *(`[!] Do not
  promise` — explicit feature selection and route mounting are worth documenting;
  a universal debug/release assumption is not).*
- **Distributed rate limiting and durable tamper-evident audit storage**
  *(partial/not implemented — worth pluggable Redis and append-only sink backends
  with atomicity, tenant namespacing, retention, and verification tests).*
- **Automated SBOM, SPDX/CycloneDX, `cargo-vet`, signed provenance, and advisory
  governance** *(partial — worth making release gates; not equivalent to SLSA
  Level 3 or organizational certification without independent evaluation).*
- **Loom/Shuttle, Kani/Miri, mutation, fuzz, and unsafe governance** *(partial —
  worth scoped blocking suites plus a reviewed `cargo-geiger` inventory; a full
  mathematical proof of the whole framework is not worth claiming).*
- **An IDOR scanner that proves authorization** *(`[!] Do not promise proof` —
  the AST scanner is worth keeping as a heuristic warning tool, paired with
  route-level ownership and cross-tenant negative tests).*
- **DevSecOps git-hook installer** *(partial — `hook:install` writes pre-commit
  and Conventional Commit hooks; worth adding backup/idempotency/permission
  tests, while CI remains authoritative because local hooks are bypassable).*
- **Automatic SOC 2/ISO/FedRAMP PASS reports** *(`[!] Do not implement as an
  unconditional verdict` — evidence export is worthwhile; certification covers
  an organization and deployment, not a crate).*

### Fiscal, payments, messaging, storage, and mail

- **Live NFS-e Nacional with PKCS#12, XML C14N/XMLDSig, XSD validation, mTLS,
  official rejection parsing, and SEFIN homologation** *(not implemented — an
  extraordinary and worthwhile Brazilian-market program, but only as a dedicated
  maintained fiscal workstream with official homologation and independent crypto
  validation).*
- **Alipay RSA2 and uniform live support across every advertised gateway** *(not
  implemented/partial — worth only with provider sandbox access, demand, and a
  method-by-method capability matrix; adapter names must not imply every payment,
  subscription, payout, portal, tax, and webhook method exists).*
- **Static fee/settlement/tax tables and “zero-cost invoicing”** *(`[!] Do not
  promise` — transparent links to current provider terms are worthwhile, but
  framework docs cannot erase certificate, accounting, infrastructure, support,
  compliance, or changing commercial costs).*
- **Durable cross-instance webhook replay/idempotency** *(partial — worth a
  pluggable database/Redis uniqueness contract before multi-instance production
  billing).*
- **RabbitMQ, Kafka, Redis Streams, NATS JetStream, SQS/SNS, and GCP Pub/Sub**
  *(not implemented — worth demand-driven adapters after defining one queue
  conformance suite, preferably in `rullst-messaging` rather than the OAuth-focused
  Connect crate).*
- **S3, Cloudflare R2, and image resizing** *(not implemented — worth isolated
  optional storage/media crates with official signing, multipart/retry semantics,
  strict path/pixel limits, deterministic mocks, and fuzzing).*
- **Mailgun, Brevo, MailerSend, Plunk, and Scaleway transports** *(not implemented
  — worth demand-driven adapters only when each has a maintainer and passes the
  shared offline/live mail contract suite).*

### IoT, edge, AI, and critical systems

- **MQTT 5, CoAP, Sparkplug B, CAN/J1939, LoRaWAN, GPIO/I2C, real firmware
  download/flashing/rollback, and hardware-in-the-loop CI** *(not implemented —
  worth separate transport and target-hardware packages after named boards and
  interoperability environments are selected).*
- **Hardware HSM/secure-element and NIST ML-KEM/PQC backends** *(not implemented;
  simulators are experimental — worth audited adapters for named hardware and
  protocols, never home-grown crypto presented as secure hardware).*
- **Autonomous AI admin, NL-SQL writes, self-healing code/schema, and DevOps
  mutation** *(not implemented as a safe production contract — read-only advice,
  dry runs, and human-approved changes are worthwhile; default autonomous
  production mutation is not).*
- **Native JSON Schema enforcement on every LLM** *(partial — capability-typed
  support is worth completing; parseable JSON must remain distinct and providers
  that cannot enforce a schema should return `UnsupportedCapability`).*
- **Any local model over any arbitrary HTTP API** *(`[!] Do not promise` — named
  Ollama and OpenAI-compatible protocols are worthwhile, while arbitrary APIs
  differ in authentication, streaming, schema, and error semantics).*
- **Automatically air-gapped/zero-leak AI** *(`[!] Do not promise` — local
  endpoints can be useful, but the host network, logs, model runtime, and
  telemetry determine the real data boundary).*
- **Aerospace/autonomous/defense framework** *(not implemented — the research is
  inspiring, but it is not worth conflating safety certification with web
  framework quality; incubate it independently if expertise, hardware, and
  governance become available).*

## Execution plan aligned with `gpt.md` §15

### Phase 0 — containment and truthful boundaries

- Keep live Fiscal, unfinished IoT integrations, S3/R2, Alipay, and other absent
  provider paths fail-closed with typed `Unsupported` results.
- Keep Nexus fail-closed, generated credentials absent, production configuration
  validated, webhook secrets mandatory, local storage confined, and the release
  workflow blocked until its dependency order and evidence agree.
- Label every capability implemented, partial, experimental, not implemented, or
  intentionally unsupported; never delete the vision to obtain truthful docs.

### Phase 1 — kernel security and reliability

- Complete environment precedence, atomic/fallible DB initialization, APP_KEY
  policy, WebAuthn conformance, content-aware DLP/PII, signed-webhook composition,
  trusted proxies, tenant isolation, CSWSH, bounded workers, scheduler shutdown,
  and the production-path zero-panic policy.

### Phase 2 — product integrity and scaffolding

- Compile all generated projects in temp directories; enforce server-side Nexus
  policy; use real or explicitly unavailable Studio telemetry; and keep offline
  mocks deterministic without allowing live endpoints to fail open.

### Phase 3 — architecture and contract

- Keep the new Core/ORM feature boundary regression-tested, consolidate the
  canonical security stack, standardize public API evolution, and split OAuth
  identity from future messaging adapters. The umbrella feature map is now
  complete and must remain covered by its powerset test.
- Implement ambitious providers only where a maintainer, conformance suite, and
  real interoperability environment exist.

### Phase 4 — release engineering

- Require formatting, strict Clippy, full workspace tests, exclusive DB-feature
  checks, generated-project checks, fuzz tiers, unsafe review, package preflight,
  SBOM/advisory evidence, provenance, and topological publishing for the exact tag.

## Release strategy

| Version | Status | Honest scope |
| :--- | :---: | :--- |
| **v12.0.0** | `[ ] Unreleased hardening` | Close the audited P0/P1/P2 regressions and prove the implemented foundations. Version numbers in manifests do not make a release complete. |
| **v12.1.x** | `[ ] Candidate window, not a promise` | Generated-project matrix, WebAuthn/JWT consolidation, typed SDK/entitlement foundations, OTel trace ingestion, safe Studio assistants, and selected demand-backed adapters. |
| **v13.x** | `[ ] Research/major-architecture window` | The remaining security-stack consolidation, audited PQC protocols, human-governed agentic operations, vendor-backed edge replication, and any independently governed critical-systems project. |

The framework may call a milestone implemented only when the same commit passes
the repository's formatting, strict lint, full-test, feature-matrix, security, and
packaging gates. Performance numbers must cite a reproducible benchmark; security
and compliance claims must state their threat model and evidence scope.

---

<div align="center">
  <p><i>"All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ)."</i></p>
</div>
