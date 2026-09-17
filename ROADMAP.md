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
> `docs/src/capability-ledger.md`. The completed v12 identity and evidence map
> live in [`docs/src/v12.md`](docs/src/v12.md); this roadmap owns the v13
> programme until a dedicated release checklist is approved.

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
| [`rullst-ai`](rullst-ai/ROADMAP.md) | Guarded provider clients, deterministic mocks/eval corpus, bounded RAG, opt-in OpenAI-compatible SSE/cancellation, SQL conversational memory, guarded tools, authenticated audit export and adaptive evaluation orchestration. | Provider-native tool loops, first-party external retrievers, non-compatible streaming adapters and hosted-model conformance remain partial or application work. Local eval orchestration is not live-model validation. |
| [`rullst-auth`](rullst-auth/ROADMAP.md) | Argon2, encrypted sessions, RBAC/policies, OAuth/OIDC re-exports, bounded application JWTs, opt-in SQLite revocation/passkey-device state and a custom ES256 passkey foundation. | Shared passkey ceremony challenges, refresh/recovery workflows and normative WebAuthn conformance remain incomplete; applications own identity, roles and deployed session policy. |
| [`rullst-capital`](rullst-capital/ROADMAP.md) | Provider trait/adapters, explicit offline mocks, canonical fail-closed webhook verification with Axum/Actix adapters, shared bounded webhook replay claims and team/workspace quotas over four relational protocols, provider-specific coupon/trial contracts, billing scaffolding, analytics, and bounded NFS-e preparation. | Live method coverage varies by gateway; cross-system exactly-once/reconciliation, Alipay RSA2, full tax/proration contracts, and homologated live NFS-e are not implemented. NFS-e is extraordinary and worthwhile only as a dedicated homologation program. |
| [`rullst-connect`](rullst-connect/ROADMAP.md) | OAuth2/OIDC/social adapters, state/PKCE/nonce lifecycle, guarded refresh/revocation contracts, encrypted account-bound tokens and opt-in shared-local SQLite token persistence with generation CAS. | Live provider conformance, remote refresh leases/reconciliation, multi-host replication, SAML/SCIM/DPoP/JWE/mTLS and risk ML remain application or roadmap work. Broker adapters belong to Messaging. |
| [`rullst-iot`](rullst-iot/ROADMAP.md) | `no_std` frames/telemetry, bounded MQTT 5 PUBLISH and CoAP request encoders, the Ed25519 OTA manifest gate, and a typed durable-counter CAS boundary with restart/retry/conflict proof. | Download, a concrete hardware-backed counter, flash/boot/rollback, MQTT/CoAP/LoRaWAN transports and session state, real hardware, HSM and PQC are not implemented; deterministic `Simulated*` types are experimental fixtures only. Keep the vision, but require target hardware and interoperability programs. |
| [`rullst-mail`](rullst-mail/ROADMAP.md) | Core REST/SMTP/log/memory/mock drivers, failover, bounded attachment/CID serialization, scheduling foundations, mandatory security/deliverability pipeline, deterministic mocks, tenant resolution, tracking tokens, factories, background worker integration, opt-in bounded attachment inspection, shared-local SQLite suppression and minimized delivery observations. | A checked item does not prove provider acceptance or inbox delivery; provider limits may be tighter, the local inspector is not antivirus/CDR, and provider webhook authentication plus multi-host suppression remain open. Compile-time mailables/CSS inlining, inbound MIME, AI dunning, DMARC/DKIM/S-MIME, Studio Mail Radar and extra gateways are not implemented; add providers only with a shared contract suite. |
| [`rullst-messaging`](rullst-messaging/ROADMAP.md) | Bounded envelopes and wire/trace codec, idempotent publication, competing consumers, leases/retry/DLQ, deterministic local broker, encrypted-content SQLite state and opt-in ORM outbox relay. | Remote Kafka/RabbitMQ/Redis Streams/NATS/SQS/Pub/Sub/Pulsar adapters, replication and provider fault evidence remain unimplemented. Local durability does not provide cross-system exactly-once delivery. |
| [`rullst-nexus`](rullst-nexus/ROADMAP.md) | Fail-closed admin construction, generated metadata/forms, CRUD/search/pagination/batch actions, opt-in trusted-context tenant scope and transaction-coupled mutation audit. | Host authentication/tenant resolution, global-model/custom-route policy, immutable external audit, custom dashboards and a visual SQL builder remain application or roadmap work. |
| [`rullst-orm`](rullst-orm/ROADMAP.md) | SQLx pools/dialects, Active Record/repository/query/schema foundations, fail-closed tenant scopes, strict DB modes, transactions, relations/soft deletes, audit/privacy, typed Turso primary, bounded MongoDB/DuckDB/SurrealDB adapters, Qdrant vectors and Redis native structures. | Several historical `[x]` entries remain partial or absent: transparent edge replication, universal external-search durability, autonomous schema/index changes, automatic graph traversal, Wasm drivers and PQC. The 45 historical claims are preserved in the [immutable v12 audit](https://github.com/Rullst/Rullst/blob/v12.0.0/docs/src/v12.md); the current capability ledger owns their boundaries. |
| [`rullst-security`](rullst-security/ROADMAP.md) | Bounded honeypot, sanitizer/CSP, RBAC, HMAC audit chain, RASP/DLP, AES-GCM vault, headers, applied Login Jail tarpit, TOTP with SVG QR, CSWSH origin policy, strict JSON/log guards, file-backed SRI, CEF formatting, compatible unsigned and opt-in HMAC-chained bounded local SIEM journals, timing/prompt filters and fail-closed CLI evidence/SBOM/doctor tools. | “Autonomous”, live reputation/external SIEM delivery, A+ guarantees, zero-leak/zero-latency, certification and total OWASP/memory-safety claims are not established. Trusted whole-tail checkpoints, spool compaction/remote acknowledgement, CSRF WebSocket tickets/frame crypto, distributed rate limits/audit sinks, KMS/rotation, adaptive WAF, SQL firewall and all PQC/kernel/Wasm containment items remain partial or absent. |
| [`rullst-studio`](rullst-studio/ROADMAP.md) | Verified-loopback developer UI, SQLx browser, supplied-OpenAPI playground, bounded queue history, ER diagrams, flags/config, authenticated trace ingestion, query heuristics and metadata-only Memory/Redis cache inspection. | Durable/OTLP trace storage, shared operator authorization, cross-process flag invalidation, Redis queue inspection and general database writes remain open. Query heuristics do not prove every N+1 or performance defect. |

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
| **M2** | DX: fast linkers, build tuning, and responsible hot reload | `[~] Partial` *(v12 uses supervised process restart with build coalescing, retained service on compile failure and a development-only browser-refresh probe. The retained experimental DLL loader is not the public v12 workflow or a stable Rust ABI. Further build-latency improvements and optional state handoff need reproducible cross-platform evidence; universal sub-100ms reloads must not be promised)* | Continuous / v13 research |
| **M3** | DX: Axum/SQLx escape hatches, granular features, proc-macro diagnostics, and ejection | `[~] Partial` *(worth improving as migration tooling — bare Core is now runtime-only, ORM/SQLite queues are explicit features, and the umbrella maps them; universal “zero lock-in” is still not worth promising because optional subsystems carry migration cost)* | Next SemVer cycle |
| **M4** | DX: `make:resource` and Ignition-style error console | `[x] Implemented (scoped)` — resource scaffolding and a local developer error console exist; autonomous mutation is evaluated separately in M37 | v12 hardening |
| **M5** | DX: documentation hub (mdBook), OpenAPI, and AST TypeScript generation | `[~] Partial` *(worth finishing — generators exist, but generated-project and serialization contract tests are still needed; AST inference is not a complete API contract)* | v13 |
| **M6** | ORM: Active Record, repository pattern, seeders, and Turso/libSQL vision | `[~] Partial` *(SQLx foundations and the bounded Turso-primary Hrana transport/matrix exist; relation/hook/auto-diff parity and transparent synchronization do not)* | v13 |
| **M7** | Edge/data: portable Wasm request/response runtime, distributed data, and autonomous upgrades | `[~] Partial` *(worth the portable edge runtime; distributed replication should use vendor-specific semantics, and autonomous upgrades are not worth enabling without signed artifacts, rollback, and operator approval)* | v13 research |
| **M8** | ORM/AI: intent-based modeling and self-optimizing production indexes | `[ ] Not implemented` *(worth an advisory, explain-and-approve implementation — automatic production DDL without review is not worth the operational risk)* | v13 research |
| **M9** | Auth: local auth, OAuth/OIDC, TOTP, passkeys, and WebAuthn | `[~] Partial` *(bounded application JWT policy and shared local revocation exist; complete refresh/recovery/session flows, shared passkey challenges and normative WebAuthn conformance remain incomplete)* | v13 |
| **M10** | Security utilities: mail, DTO validation, rate limiting, and Shield | `[~] Partial` *(worth completing — local controls and mail transports exist, while distributed rate limiting and some provider invariants require real backends and conformance tests)* | v13 |
| **M11** | SaaS: hardened Nexus, Omni vision, billing, and entitlements | `[~] Partial` *(worth building in bounded modules — Nexus and billing foundations exist, but Omni, uniform live gateway coverage, and declarative entitlements are not complete)* | v13+ |
| **M12** | Defense in depth: RASP/WAF, Vault, honeypots, HMAC audit, secure headers, Login Jail, DLP, TOTP, fingerprinting, CLI inspection, and Threat Radar | `[~] Partial` *(worth continuous hardening — concrete controls exist, but they do not prove universal OWASP coverage, zero leakage, external intelligence, or certification)* | Continuous |
| **M13** | Post-quantum web architecture, `rullst-quantum`, NIST PQC, and sandboxed Wasm plugins | `[ ] Not implemented` *(worth later only for a concrete protocol and threat model, using audited primitives; home-grown “quantum-safe” crypto is not worth implementing)* | v13 research |
| **M14** | Frontend: HTMX-first SSR and Leptos/Dioxus interoperability | `[~] Partial` *(worth improving — HTMX/HTML support is real, while the current Leptos/Dioxus types are compatibility wrappers rather than full framework integrations; “zero bundle” is a selectable architecture, not a universal guarantee)* | v13 |
| **M15** | Runtime: queues, cache, scheduler, multi-stage Docker, and brokered messaging | `[~] Partial` *(bounded Core Memory/SQLite/Redis foundations plus `rullst-messaging` envelopes, idempotency, groups, leases, retry/DLQ, deterministic broker, contract suite and durable local SQLite state exist; remote codec/replication and RabbitMQ, Kafka, Redis Streams, NATS, SQS/SNS, GCP Pub/Sub and Pulsar adapters do not)* | Foundation v12; remote adapters v13+ |
| **M16** | Wasm islands and `#[client_component]` | `[~] Partial` *(the bounded `#[server_function]` transport is now implemented over `rullst.client` v1 with a generated Axum route, Wasm caller, compile diagnostics and native/Wasm/scaffold evidence; island hydration, packaging, real-browser interoperability and a stable component ABI remain open)* | v13 |
| **M17** | Real-time, object storage, media, and `cargo rullst pkg` | `[~] Partial` *(worth modular expansion — WebSocket/SSE and local storage foundations exist; S3/R2, image processing, and a production package-registry contract do not)* | v13+ |
| **M18** | LiveView-style server-driven UI and `make:live` | `[~] Partial` *(worth hardening — a WebSocket component loop exists, but auth, reconnect, backpressure, diff semantics, and browser E2E coverage remain)* | v13 |
| **M19** | AI/telemetry: Radar, agent tool schemas, spans, and Prometheus `/metrics` | `[x] Implemented (bounded)` — local telemetry and export surfaces exist; unavailable sources must remain unavailable rather than becoming invented values | v12 hardening |
| **M20** | Persistence: zero-copy event streaming and immutable ledger engine | `[ ] Not implemented` *(interesting but lower priority — worth implementing only after defining persistence, consistency, recovery, and verification semantics; the HMAC audit chain is not a distributed ledger)* | v13 research |
| **M21** | Omni-frontend protocol and mobile hypermedia bridge | `[~] Partial` *(the web-first Tauri shell, shared `rullst.client` v1 envelope and bounded native offline-state foundation exist; v13 must make signed Android distribution a first Omni hardening milestone with guided application-owned keystores, exact artifact selection, `apksigner` verification, debug/release separation and fail-closed rejection of unsigned release APKs. Platform persistence/secure keys, concrete network/background orchestration, native capabilities, physical-device evidence and store publication remain open)* | v13 research |
| **M22** | Agentic DevOps and autonomous infrastructure provisioning | `[~] Partial` *(worth keeping as human-reviewed recommendations — telemetry advice exists; unattended infrastructure mutation is not worth enabling by default without preview, scoped credentials, audit, rollback, and policy)* | v13 |
| **M23** | Polymorphic core and auto-healing runtime/database | `[~] Partial` *(worth keeping as diagnostics — a schema-error suggestion helper exists; automatic code/schema mutation is not worth enabling by default without validated plans, approval, and rollback)* | v13 |
| **M24** | Embedded IoT: `no_std` frames and an Ed25519 OTA manifest gate | `[~] Partial` *(the frame/MQTT-PUBLISH/CoAP-request encoders, verification foundation and durable-counter CAS adapter contract exist; download, a hardware-backed store, flashing, boot slots, HSM/PQC, and transport interoperability do not)* | v12 foundation / v13 integrations |
| **M25** | Async embedded IoT with Embassy | `[ ] Not implemented` *(worth implementing after transport and hardware traits stabilize, because executor integration before those boundaries would create churn)* | v13+ |
| **M26** | Guided PaaS/VPS deploy for Fly, Railway, Render, and Caddy | `[~] Partial` *(worth hardening — scaffolding and helpers exist, but “one click” and zero downtime are not framework guarantees because credentials, DNS, migrations, health, and rollback remain operator concerns)* | v13 |
| **M27** | Kubernetes manifest scaffolding and `/health`/`/ready` probes | `[x] Implemented (scaffolding scope)` — generated manifests remain deployment inputs that operators must review | v12 hardening |
| **M28** | Compile-time DI and `Inject<T>` | `[x] Implemented (foundation)` — the typed container exists; “zero cost” remains a benchmarkable goal rather than a guarantee | v12 hardening |
| **M29** | Scalar playground at `/docs` and OpenAPI generation | `[~] Partial` *(worth finishing — the UI/router/generator exist, but full OpenAPI fidelity requires typed schemas and validation rather than syntax inference)* | v13 |
| **M30** | Tonic/gRPC and Protobuf scaffolding | `[~] Partial` *(worth finishing — `make:grpc` emits a starting service, but a distinct supported `rullst-grpc` crate and generated-project conformance matrix do not yet exist)* | v13 |
| **M31** | Aerospace, autonomous vehicles, robotics, and defense (`rullst-orbit` / `rullst-auto`) | `[ ] Not implemented` *(extraordinary, but not worth placing inside the web-framework Core; consider a separate safety-critical project only after hardware, standards, certification, and governance exist)* | Separate future program |
| **M32** | Architecture: first-class Axum/Tower escape hatches and precise proc-macro diagnostics | `[x] Implemented (bounded)` — router conversion/interoperability and `syn::Error` diagnostics exist; continue compatibility tests | v12 hardening |
| **M33** | SaaS: `#[rullst::gate]` and `GateGuard` declarative entitlements | `[ ] Not implemented` *(worth implementing for SaaS only if enforcement is server-side, tenant-bound, auditable, and independent of hidden UI controls)* | v13 |
| **M34** | Multi-target SDK generator for TypeScript, React, Dart, and Swift | `[ ] Not implemented` *(worth implementing from one canonical typed API schema; multiplying AST heuristics across languages is not worth the drift)* | v13+ |
| **M35** | Distributed OpenTelemetry trace-waterfall visualizer in Studio | `[~] Partial` *(worth implementing — Studio has trace surfaces, but a distributed OTel waterfall needs real ingestion, clock/skew handling, sampling metadata, and unavailable states)* | v13+ |
| **M36** | Natural-language-to-SQL Studio data copilot | `[ ] Not implemented` *(worth a read-only, explainable assistant with schema allowlists, parameterization, preview, limits, and approval; autonomous production writes are not worth the risk)* | v13 research |
| **M37** | One-click AI error-console autofix | `[~] Partial` *(worth retaining as a local, reviewable patch workflow — an autofix endpoint exists, but autonomous edits need diff preview, workspace confinement, audit, tests, and rollback)* | v13 |
| **M38** | In-memory/local-NVMe SQLite read replicas with background synchronization | `[ ] Not implemented` *(worth vendor-specific adapters when demanded; generic “transparent replication” is not worth claiming because consistency and failover semantics belong to the selected database)* | v13 research |
| **M39** | Optional self-hosted Rullst Gateway and load balancer | `[ ] Not implemented` *(worth a phased v13 design as a separate opt-in `rullst-gateway` crate/binary, preferably on a maintained proxy foundation such as Pingora. It should consume explicit readiness/drain signals and begin with bounded upstream selection, health checks, WebSocket forwarding and telemetry. It must not live inside `rullst-core` or claim parity with a managed global cloud service, whose network, DDoS controls, multi-zone operations and SLA are external infrastructure.)* | v13 research/foundation |
| **M40** | Isolated programming labs and learning-game execution | `[ ] Not implemented` *(worth a phased v13 design as opt-in `rullst-labs` contracts plus a separately deployed `rullst-labs-runner`. The web process must never execute learner code or receive a container control socket. Full offensive CTF arenas require independently operated, isolated infrastructure; see the dedicated roadmap.)* | v13 research/foundation |
| **M41** | Privacy defaults and proportional age assurance | `[~] Initial foundation` *(opt-in unpublished `rullst-privacy` age policies, signed evidence and replay contracts; real age providers, guardian verification, consent/rights/retention workflows and reviewed regional profiles remain open. See the [delivery plan](docs/src/privacy-age-assurance-roadmap.md). No automatic worldwide compliance claim.)* | v13 P0 |

## Quantified planning horizon through v13

This second progress lens answers a different question from release readiness:
how much of the **canonical long-term milestone programme through v13** remains
if every milestone that is not yet `[x]` stays in scope?

The snapshot below was recalculated on 17 September 2026 from M1–M41. It includes
v12 hardening, continuous, next-SemVer, v13 and v13-research rows. M31 is excluded
because the tracker explicitly assigns aerospace/autonomous/defence work to a
separately governed future programme rather than the general v12/v13 framework suite.
Detailed crate-roadmap checkboxes are not added again: they overlap with and
decompose these canonical milestones, so a raw sum would double-count work.

| State | Milestones | Share of the 40-milestone horizon |
| :--- | ---: | ---: |
| `[x]` bounded completion | **5** | **12.5%** |
| `[~]` useful but incomplete foundation | **25** | **62.5%** |
| `[ ]` not implemented | **10** | **25.0%** |
| **Total in scope through v13** | **40** | **100%** |

Two calculations are intentionally retained:

- **Strict closure:** 5/40 are closed, so **87.5% remains open** (35
  milestones). This is the correct answer when a partial milestone counts as
  unfinished.
- **Weighted engineering maturity:** `(5 + 25 × 0.5) / 40` is **43.75% complete**,
  leaving **56.25% equivalent work**. That remainder is the ten untouched
  milestones (25 percentage points) plus the unfinished half of the 25
  partial milestones (31.25 points).

This is a scope/maturity indicator, not a duration estimate. Provider accounts,
physical hardware, store acceptance, fiscal homologation, independent audits
and research-grade cryptography cannot be completed by repository code alone.
The 56.25% must not be added to the historical-claim campaign or the v12 release
checklist because those lenses substantially overlap.

## AI-native vision, without absolutes

The original goal of becoming an **AI-native Rust framework suite** is preserved
as a design ambition, not a historically provable “first” claim.
The dedicated
[AI maintainability and project-building roadmap](docs/src/ai-maintainability-roadmap.md)
defines the v13 acceptance work for generated instructions, bounded
context, golden tasks and reproducible model evaluation.

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
- **A first-party load balancer embedded in every application** *(`[!] Do not
  make the default` — an opt-in `rullst-gateway` process is worth researching
  for self-hosted deployments, but application serving and edge proxying need
  independent failure, upgrade and privilege boundaries. Matching a managed
  cloud load balancer's global infrastructure or SLA is not a repository-code
  claim).*
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
  *(remote adapters not implemented — the separate `rullst-messaging` crate now
  provides the bounded envelope, in-memory broker, durable local SQLite adapter
  and common contract foundation;
  add providers only after their delivery semantics pass provider-specific
  restart and fault evidence).*
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
- **Any local model over any arbitrary HTTP API** *(`[!] Do not promise` —
  named Ollama and a capability-declared OpenAI-compatible local/cloud adapter
  are implemented, while arbitrary APIs differ in authentication, streaming,
  tools, schema, and error semantics and use the public provider trait).*
- **Automatically air-gapped/zero-leak AI** *(`[!] Do not promise` — local
  endpoints can be useful, but the host network, logs, model runtime, and
  telemetry determine the real data boundary).*
- **Aerospace/autonomous/defense framework** *(not implemented — the research is
  inspiring, but it is not worth conflating safety certification with web
  framework quality; incubate it independently if expertise, hardware, and
  governance become available).*

## Audited execution plan

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

### v13 execution priorities

**Execution order: verification efficiency, then the compatible 12.1.0 update
experience, then concentrated v13 product development.** The published v12
packages remain immutable; important maintenance fixes remain independently
reviewable. Carry the compatible updater and applicable fixes into v13 without
merging unrelated breaking work into the v12 line. The phased efficiency plan is in
[WORKFLOWS.md](WORKFLOWS.md#verification-efficiency--v12-maintenance-and-v13).
Application API changes still belong to the appropriate release line.

Complete the compatible **12.1.0** update experience after the verification
foundation and before concentrating new capability work on v13. The
[maintenance checkpoint](docs/src/v12.md#1210-delivery-checkpoint-unreleased)
distinguishes the working source from the published release and lists the
remaining delivery gates.

The released v12 baseline and documentation closeout have been merged into
v13 while preserving its separate Labs proposal and planning commits. Keep
subsequent applicable stable fixes synchronized through reviewed changes. A
development branch name alone does not prove that it includes later fixes.

The compatible maintenance source through `236579f3` is now carried into v13:
SaaS provider/WAF fixes, Stripe customer/checkout/event contracts and transactional
inbox, ORM driver/enum isolation, Windows cache and Android signing corrections.
The unpublished age-assurance package and v13 Labs/privacy plans remain separate.
This synchronization is not publication or completion of the remaining 12.1 gates.

| Order | Outcome | Acceptance boundary |
| :--- | :--- | :--- |
| **P0 — verification efficiency** | Shorter local and hosted feedback, with measured cold/warm build and queue times | Compare test inventories; select affected crates and their consumers; preserve broad scheduled/release checks and a full-run fallback for unknown changes. Bind reusable evidence to source, dependencies, tools and policy. Prove that security, workflow, manifest and generator changes cannot silently skip required checks. |
| **P0 — safe update experience** | Discover, prepare, verify and approve CLI/project updates through one guided entry point | Compatible opt-in 12.1.0 delivery first, carried into v13. Discovery, private Unix/Windows caching and recovery hardening exist; verified installation and isolated project acceptance remain release blockers. |
| **P0 — SaaS maintenance** | Contain affected live operations and repair confirmed examples feedback | Follow the [15-finding triage plus Nexus configuration fix](docs/src/saas-v12-1-v13-triage.md). Compatible v12.1 fixes remain independently deliverable; new payment contracts need durable ownership/idempotency and provider acceptance evidence. |
| **P0 — privacy and age assurance** | Reusable privacy defaults and age checks proportionate to risk across SaaS, LMS and examples | Complete the [M41 delivery plan](docs/src/privacy-age-assurance-roadmap.md) before additional learning/monitoring features. Reject production mocks and unverifiable results; minimize data, offer alternatives and review jurisdiction profiles. No automatic legal certification. |
| **P1 — navigable API documentation** | Developers can find a capability, understand its contract and run a realistic example | Connect versioned Rust API references, task-based guides and tested REST examples. Document errors, feature flags, security boundaries and migration paths alongside each prioritized API; see the [documentation plan](#api-documentation-quality). |
| **P1 — Omni application delivery** | Predictable desktop/mobile builds, diagnostics and installation guidance | Detect SDK/toolchain/signing/identifier/version/ABI mistakes, distinguish unsigned build output from installable signed packages, and test lifecycle, navigation and interrupted networks. Device and store acceptance need their own evidence. |
| **P1 — coherent application contracts** | One clear path for sessions, ownership, tenant context and typed client APIs | Consolidate existing Auth/Core/Security boundaries, complete selected session/passkey flows and validate API/SDK serialization. Preserve explicit configuration and negative authorization tests. |
| **P2 — interactive learning products** | Server-authoritative progress, gamification and isolated programming exercises | Build on the current LMS scaffolds; version grading rules, persist idempotent results and prove tenant isolation. Follow the existing v13 `rullst-labs`/`rullst-labs-runner` proposal; untrusted execution stays outside the web process. |
| **P2 — transparent monitoring patterns** | Application examples for consent-based exam supervision and age-appropriate parental controls | Explicit device permissions, visible collection state, bounded retention/export/deletion and access audit. Automated observations assist human review; they do not prove misconduct. Browser capabilities and OS-level parental controls require different platform contracts. |
| **P3 — selected integrations** | One complete real-provider or broker journey at a time | Add an adapter only with a concrete product need, protocol/failure tests, documented limits and an available acceptance environment. Gateway/load-balancer research remains opt-in rather than blocking the core release. |

This is an execution order, not a promise that the entire historical backlog
fits one month. Prefer completed user journeys and measured acceptance criteria
over increasing the number of crates. The
[workflow roadmap](WORKFLOWS.md#preserved-next-generation-roadmap) retains the
assurance experiments; move a proposal into the implementation column only
after its code and evidence exist.

### Safe update experience

**Status: planned for 12.1.0 and carried forward into v13.** The goal is
the easiest practical update journey without hiding risk: one guided entry point, a clear
plan, minimal repeated input, useful progress, verification and recoverable
application of the approved changes. Ease and speed are acceptance criteria,
not reasons to skip compatibility or security checks.

The existing `cargo rullst upgrade` already provides workspace-aware plans,
versioned migration rules, controlled file snapshots, compiler fixes and Cargo
checks. It does **not** install the CLI or run the application's full acceptance
suite. Extend this boundary in `cargo-rullst`; do not introduce another crate
or count this proposal as completed work in the capability ledger.

Implementation and acceptance order:

Initial discovery hardening is in the working source, not a completed 12.1.0
delivery: interactive-only, offline/CI-aware notices use bounded HTTPS metadata
and reject redirects, yanked versions, prereleases and unsolicited major jumps.
The legacy shared temporary cache is removed; the current result is deliberately
process-local. Explicit `cargo rullst update check` now provides exact-target,
MSRV/platform and versioned JSON discovery with separate major/prerelease
opt-ins; it grants no installation or execution authority. Explicit discovery
now reuses bounded, owner/permission-checked Unix metadata for six hours and
supports offline reads, forced refresh and cache opt-out. Windows persistence
has a private owner/DACL implementation with native acceptance recorded in the
[maintenance checkpoint](docs/src/v12.md#1210-delivery-checkpoint-unreleased).
The installation/preparation/application stages below remain unfinished. The
published v12.0.0 release currently contains source crate archives and evidence,
not an inventory of trusted prebuilt CLI executables; adding those artifacts
requires release-pipeline work, not an assumed download URL.

Working-source preparation now builds native CLI candidates on four explicit
targets and binds their version, source, platform, sizes and digests in a
bounded inventory. The admitted tag workflow separately attests the files and
adds release assets; ordinary CI inventories have no release tag. Native and
release evidence is still pending, and verified client-side download/staging,
installation, ownership/locking, recovery and project acceptance are unfinished.
The explicit local `update verify` command now authenticates a private manifest
snapshot with the caller-installed GitHub CLI and checks both native binary
digests. It grants no installation authority or registry eligibility and does
not download, execute or install candidates. Platform/release acceptance is
still required; this is one verifier boundary in the unfinished flow below.

1. **Discover and explain.** Make update notices useful without blocking normal
   CLI startup. Respect offline/CI settings and explicit notification opt-out;
   use bounded responses, timeouts and a private, path-safe cache. Default to
   supported stable releases within the selected major, with exact target pins;
   prereleases and major migrations require explicit selection. Show release
   notes, MSRV/platform requirements and unsupported migration paths before
   proposing changes. Metadata or a notification never grants installation
   authority.
2. **Update the CLI safely.** Offer verified prebuilt binaries for supported
   OS/architecture pairs, with a pinned source-install fallback when appropriate.
   Bind artifact identity/version/platform/digest to a trusted publisher identity
   through signatures or verified provenance; a checksum from the same untrusted
   download is insufficient. Respect package-manager ownership and permissions;
   stage replacements with concurrency locks, interrupted-download recovery and
   Windows executable-lock handling. Test rejected tampering, unexpected
   redirects/archives, stale metadata and unauthorized downgrades. Keep an
   explicit, verified known-good recovery path, not a silent downgrade.
3. **Prepare and verify the project.** Reuse the versioned migration catalog in
   an isolated working copy, preserving uncommitted user work. Show dependency,
   lockfile and source diffs and the validation commands before execution; Cargo
   build scripts, procedural macros and tests execute code, so preparation is
   not a sandbox or permission to run an untrusted project. Resolve the chosen
   target reproducibly and validate the candidate lockfile, supported feature
   sets and application-owned tests before accepting changes. Unknown or breaking
   migrations stop with actionable instructions instead of guessed rewrites.
4. **Apply with consent and recover.** Make the interactive happy path concise;
   expose structured reports and explicit non-interactive policy for automation.
   Verify that reviewed inputs have not changed before applying. Preserve bounded
   backups and prove cancellation, concurrent edits, disk-full recovery and
   restoration on Linux, Windows and macOS. State exactly which files are
   restored: file rollback does not undo arbitrary test side effects, database
   changes or external services. Opening an application or running a build must
   never silently replace its framework, migrate its database or deploy it.
5. **Prove usability and speed.** Exercise published-package/generated-project
   fixtures, pinned versions, offline operation, unsupported targets and failed
   migrations. Measure cold/warm discovery, download, compilation and validation
   separately. Reuse only correctly keyed caches and applicable test evidence.
   A prebuilt CLI can avoid CLI compilation; updating a Rust application can
   still require rebuilding, testing and a separate deployment. Do not promise
   instant upgrades, zero downtime or automatic production readiness.

The 12.1.0 delivery must preserve v12's public APIs, CLI/configuration behavior
and opt-in boundaries. It prepares discovery and installation of a compatible
migration CLI, not guesses about a future major's source changes. Actual
v12-to-v13 automation requires v13's published migration catalog and tested
application fixtures; the same-major restriction of the current `upgrade`
command must not be silently removed. Reserve incompatible changes for v13.

After this bounded minor is implemented, validated and released, concentrate
new capability work on v13, with v12 maintenance by exception. The website
redesign is a separate documentation delivery, not a reason to bump framework
versions or postpone verification work. This plan neither bumps package
versions nor authorizes publication, and it does not claim that any major
application migration is already automatic.

### API documentation quality

**Status: planned for v13; improve documentation alongside each implemented
contract, not only at release time.** Distinguish the Rust framework API
(types, traits, functions and features) from guides for building HTTP/REST
APIs. The existing REST quickstart intentionally covers one JSON endpoint;
routing, authentication and Scalar/OpenAPI guidance live in separate chapters.
More pages alone will not make those paths easier to discover or complete.
The [v12 navigation index](https://github.com/Rullst/Rullst/blob/v13/docs/src/api-reference.md)
now connects existing guides to exact-version crate references and identifies
the remaining REST walkthrough gaps. This first navigation improvement is not
completion of the reference and behavioral-example programme below.

Use [Qt's reference navigation](https://doc.qt.io/qt-6/reference-overview.html)
and [a concrete class reference](https://doc.qt.io/qt-6/qnetworkaccessmanager.html)
as organizational inspiration, not as a reason to adopt Qt or copy its text.
Adapt the pattern to Rust with a searchable crate/module/task index linking the
book and version-pinned rustdoc pages. For prioritized public interfaces,
document purpose, imports/Cargo features, arguments, results and typed errors,
security/ownership/concurrency constraints, runnable examples, related APIs,
version availability and migration notes. Start with the umbrella facade,
Core/routing, Auth/Security and ORM; extend coverage with each v13 increment.

Provide a coherent REST learning path covering typed input validation, error
responses, CRUD and pagination, authentication, owner/tenant authorization,
OpenAPI, tests and deployment boundaries. Validate examples against the
documented release and feature set, including rejected input and access denial.
Keep the existing book-doctest integration and add behavioral fixtures where
compilation alone cannot prove the documented result. Check rendered book links
as well as repository-local links. Record which API surfaces
were reviewed; do not infer complete reference coverage from a green book build
or label unimplemented v13 contracts as available in v12.

### Published and planned release lines

| Version | Status | Honest scope |
| :--- | :---: | :--- |
| **v12.0.0** | `[x] Published stable` | Tag `v12.0.0` at `eb11f892` completed the protected release workflow and published all sixteen packages on September 15, 2026. |
| **v12.0.x** | `[~] Maintenance if needed` | Preserve the published stable line; separately review important compatible fixes when necessary. |
| **v12.1.0** | `[ ] Planned compatible minor` | Guided CLI/project updates and separately reviewable SaaS/Nexus maintenance from the [examples triage](docs/src/saas-v12-1-v13-triage.md). Preserve v12 contracts and validate artifact trust, recovery, generated migrations and platform behavior; not yet published. |
| **v13.x** | `[ ] Next feature line` | Compatible and breaking improvements move together into the next deliberate cycle: generated-project coverage, auth/session consolidation, typed SDKs, selected adapters, security-stack consolidation and research-heavy architecture all require fresh acceptance boundaries. |

The framework may call a milestone implemented only when the same commit passes
the repository's formatting, strict lint, full-test, feature-matrix, security, and
packaging gates. Performance numbers must cite a reproducible benchmark; security
and compliance claims must state their threat model and evidence scope.

---

<div align="center">
  <p><i>"All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ)."</i></p>
</div>
