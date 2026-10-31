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
> `docs/src/capability-ledger.md`. What each release delivered is in
> [`CHANGELOG.md`](CHANGELOG.md); the completed v12 identity and evidence map
> live in [`docs/src/v12.md`](docs/src/v12.md). This roadmap owns the v13
> programme until a dedicated release checklist is approved.

## Next twelve months (October 2026 – October 2027)

Intentions, not promises; dates follow the evidence gates described below.

- **v13:** `13.0.0-alpha.1` to `alpha.3` were published from 11 to 24
  October 2026. `13.0.0-alpha.4`, prepared for 31 October, is the 13.0 feature
  and public API freeze. Then `13.0.0-rc.1` (fixes and documentation only,
  driven by user feedback) and `13.0.0` stable, from which the
  [maturity tiers](docs/src/maturity.md) apply; see the
  [path to 13.0.0](#path-to-1300).
- **13.x:** compatible minors after 13.0.0, ordered by user demand (see
  [13.x minor candidates](#13x-minor-candidates-and-decided-non-goals));
  breaking ideas wait for [v14](#v14-candidates).
- **v12:** end of life on 16 October 2026; 12.3.0 is the last release and the
  `v12` branch is frozen. Its unpublished fixes (PR #500 and the RustSec
  updates) stay on the branch; new 12.x projects no longer resolve their
  dependencies after crates.io yanked the `zip` releases its Studio needs.
- **Assurance:** keep the OpenSSF Best Practices Silver badge (granted on
  10 October 2026) current, extend mutation testing from the measured crates
  to the whole workspace ([campaign status](docs/src/mutation-testing.md#campaign-status)),
  add the test-only checks of the [verification roadmap](#verification-roadmap),
  and prepare an independent review with the
  [external audit kit](docs/src/external-audit-kit.md).
- **Selected programmes:** the [green software roadmap](docs/src/green-software-roadmap.md)
  and the [legal compliance roadmap](docs/src/legal-compliance-roadmap.md).

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
listed with the [release lines](#published-and-planned-release-lines).

Planning records: the [investment proposal](docs/src/v13-priorities.md)
(M1–M41 grouped into 25 themes to invest in and 16 to defer), the
[maintenance scope](docs/src/v13-maintenance-scope.md), the
[framework comparison plan](docs/src/v13-framework-comparison.md) and the
[delivery plan](docs/src/v13-delivery-plan.md) with each increment's evidence.
[Capability status](docs/src/capability-status.md) counts the labels below
(5 implemented, 27 partial, 8 not started, M31 excluded); the count measures
neither release readiness nor remaining effort.

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
| **M9** | Auth: local auth, OAuth/OIDC, TOTP, passkeys, and WebAuthn | `[~] Partial` *(bounded application JWT policy, shared local revocation and opt-in PostgreSQL passkey ceremonies exist with hosted/database/browser evidence; complete refresh/recovery/session flows and normative WebAuthn conformance remain incomplete)* | v13 |
| **M10** | Security utilities: mail, DTO validation, rate limiting, and Shield | `[~] Partial` *(worth completing — local controls and mail transports exist, while distributed rate limiting and some provider invariants require real backends and conformance tests)* | v13 |
| **M11** | SaaS: hardened Nexus, Omni vision, billing, and entitlements | `[~] Partial` *(worth building in bounded modules — Nexus and billing foundations exist, but Omni, uniform live gateway coverage, and declarative entitlements are not complete)* | v13+ |
| **M12** | Defense in depth: RASP/WAF, Vault, honeypots, HMAC audit, secure headers, Login Jail, DLP, TOTP, fingerprinting, CLI inspection, and Threat Radar | `[~] Partial` *(worth continuous hardening — concrete controls exist, but they do not prove universal OWASP coverage, zero leakage, external intelligence, or certification)* | Continuous |
| **M13** | Post-quantum web architecture, `rullst-quantum`, NIST PQC, and sandboxed Wasm plugins | `[ ] Not implemented` *(worth later only for a concrete protocol and threat model, using audited primitives; home-grown “quantum-safe” crypto is not worth implementing)* | v13 research |
| **M14** | Frontend: HTMX-first SSR and Leptos/Dioxus interoperability | `[~] Partial` *(worth improving — HTMX/HTML support is real, while the current Leptos/Dioxus types are compatibility wrappers rather than full framework integrations; “zero bundle” is a selectable architecture, not a universal guarantee)* | v13 |
| **M15** | Runtime: queues, cache, scheduler, multi-stage Docker, and brokered messaging | `[~] Partial` *(bounded Core Memory/SQLite/Redis foundations plus `rullst-messaging` envelopes, idempotency, groups, leases, retry/DLQ, deterministic broker, contract suite and durable local SQLite state exist; the Redis Streams profile passed source admission in PR #236 and ships in the 13.0.0-alpha.1 prerelease; replication and RabbitMQ, Kafka, NATS, SQS/SNS, GCP Pub/Sub and Pulsar adapters remain future work)* | Foundation v12; remote adapters v13+ |
| **M16** | Wasm islands and `#[client_component]` | `[~] Partial` *(the bounded `#[server_function]` transport is now implemented over `rullst.client` v1 with a generated Axum route, Wasm caller, compile diagnostics and native/Wasm/scaffold evidence; island hydration, packaging, real-browser interoperability and a stable component ABI remain open)* | v13 |
| **M17** | Real-time, object storage, media, and `cargo rullst pkg` | `[~] Partial` *(WebSocket/SSE and local storage foundations exist. The Bunny Stream lifecycle, first published in the 13.0.0-alpha.1 prerelease, passed hosted/browser/archive source acceptance in PR #227; real provider/CDN interoperability remains unvalidated. Optional S3/R2 private operations passed source/package admission in PR #236; final release and provider interoperability remain unvalidated; image processing and a production package-registry contract remain open)* | v13+ |
| **M18** | LiveView-style server-driven UI and `make:live` | `[~] Partial` *(an opt-in typed recovery candidate adds mandatory authorization, full snapshots, revision conflicts, bounded connections/actions and actual Chromium restart/disconnect evidence; source/package admission passed in PR #236; final release admission remains separate. Legacy components/scaffolds stay separate; DOM diffing, broadcast and automatic migration remain open. See [the supported contract](docs/src/live-recovery.md))* | v13 |
| **M19** | AI/telemetry: Radar, agent tool schemas, spans, and Prometheus `/metrics` | `[x] Implemented (bounded)` — local telemetry and export surfaces exist; unavailable sources must remain unavailable rather than becoming invented values | v12 hardening |
| **M20** | Persistence: zero-copy event streaming and immutable ledger engine | `[ ] Not implemented` *(interesting but lower priority — worth implementing only after defining persistence, consistency, recovery, and verification semantics; the HMAC audit chain is not a distributed ledger)* | v13 research |
| **M21** | Omni-frontend protocol and mobile hypermedia bridge | `[~] Partial` *(the shared `rullst.client` v1 envelope exists; 13.0.0-alpha.3 retires the Tauri shells and the native offline-state foundation in favour of installable PWAs: `make:pwa`, a Google Play Trusted Web Activity with Digital Asset Links, and Microsoft Store packaging with PWABuilder. Native capabilities, physical-device evidence and store publication remain open)* | v13 research |
| **M22** | Agentic DevOps and autonomous infrastructure provisioning | `[~] Partial` *(worth keeping as human-reviewed recommendations — telemetry advice exists; unattended infrastructure mutation is not worth enabling by default without preview, scoped credentials, audit, rollback, and policy)* | v13 |
| **M23** | Polymorphic core and auto-healing runtime/database | `[~] Partial` *(worth keeping as diagnostics — a schema-error suggestion helper exists; automatic code/schema mutation is not worth enabling by default without validated plans, approval, and rollback)* | v13 |
| **M24** | Embedded IoT: `no_std` frames and an Ed25519 OTA manifest gate | `[~] Partial` *(the frame/MQTT-PUBLISH/CoAP-request encoders, verification foundation and durable-counter CAS adapter contract exist; download, a hardware-backed store, flashing, boot slots, HSM/PQC, and transport interoperability do not)* | v12 foundation / v13 integrations |
| **M25** | Async embedded IoT with Embassy | `[ ] Not implemented` *(worth implementing after transport and hardware traits stabilize, because executor integration before those boundaries would create churn)* | v13+ |
| **M26** | Guided VM/VPS deploy with Caddy | `[~] Partial` *(worth hardening — `cargo rullst deploy` writes a Dockerfile, `compose.yaml` and `Caddyfile` for a Linux VM with Docker or Podman, with guides for Hetzner Cloud, DigitalOcean, Oracle Cloud and Google Cloud; 13.0.0-alpha.4 removed the Fly.io, Render and Railway targets. “One click” and zero downtime are not framework guarantees because credentials, DNS, migrations, health, and rollback remain operator concerns)* | v13 |
| **M27** | Kubernetes manifest scaffolding and `/health`/`/ready` probes | `[x] Implemented (scaffolding scope)` — generated manifests remain deployment inputs that operators must review | v12 hardening |
| **M28** | Compile-time DI and `Inject<T>` | `[x] Implemented (foundation)` — the typed container exists; “zero cost” remains a benchmarkable goal rather than a guarantee | v12 hardening |
| **M29** | Scalar playground at `/docs` and OpenAPI generation | `[~] Partial` *(worth finishing — the UI/router/generator exist, but full OpenAPI fidelity requires typed schemas and validation rather than syntax inference)* | v13 |
| **M30** | Tonic/gRPC and Protobuf scaffolding | `[ ] Not implemented — removed` *(13.0.0-alpha.4 removed the `make:grpc` starting-service generator to keep the CLI lean; gRPC stays possible with an application's own `tonic` setup, and a supported `rullst-grpc` crate is not planned)* | v13 |
| **M31** | Aerospace, autonomous vehicles, robotics, and defense (`rullst-orbit` / `rullst-auto`) | `[ ] Not implemented` *(extraordinary, but not worth placing inside the web-framework Core; consider a separate safety-critical project only after hardware, standards, certification, and governance exist)* | Separate future program |
| **M32** | Architecture: first-class Axum/Tower escape hatches and precise proc-macro diagnostics | `[x] Implemented (bounded)` — router conversion/interoperability and `syn::Error` diagnostics exist; continue compatibility tests | v12 hardening |
| **M33** | SaaS: `#[rullst::gate]` and `GateGuard` declarative entitlements | `[~] Bounded foundation` *(typed current-state plan authorization and its authenticated SaaS consumer passed PR #221; the proposed attribute/guard syntax and broader declarative entitlement model remain open)* | v13 |
| **M34** | Multi-target SDK generator for TypeScript, React, Dart, and Swift | `[~] Bounded TypeScript profile` *(one explicit OpenAPI 3.1 profile generates Rust codecs and a strict TypeScript HTTP client, with hosted consumer/archive acceptance in PR #221; React, Dart, Swift and broader schemas remain open)* | v13+ |
| **M35** | Distributed OpenTelemetry trace-waterfall visualizer in Studio | `[~] Partial` *(Core/facade now has a minimized OTLP propagation/export candidate with local standard-collector TLS and independent-process ancestry evidence; source/package admission passed in PR #236; final release admission remains separate. Studio keeps its separate authenticated ingestion/local viewer; a unified OTLP-backed waterfall, durable storage and cross-host clock/skew presentation remain open. See [the tracing profile](docs/src/distributed-tracing.md))* | v13+ |
| **M36** | Natural-language-to-SQL Studio data copilot | `[ ] Not implemented` *(worth a read-only, explainable assistant with schema allowlists, parameterization, preview, limits, and approval; autonomous production writes are not worth the risk)* | v13 research |
| **M37** | One-click AI error-console autofix | `[~] Partial` *(the development error page no longer edits files and makes no network request. The server keeps a bounded, expiring, header- and body-free panic context that only a direct loopback request can read. The terminal assistant that fixed it (`cargo rullst ai fix`) was removed in 13.0.0-alpha.4; the page's "Ask your AI assistant" panel now gives a copyable prompt and the recorded-context path to the developer's own assistant, and the dead `/_rullst/autofix` endpoint (`410 Gone`) points there. Still open: a one-click flow, an edit audit trail beyond the checkpoint ref, evidence of fix quality with live providers, and the hot-reload server, which does not record errors)* | v13 |
| **M38** | In-memory/local-NVMe SQLite read replicas with background synchronization | `[ ] Not implemented` *(worth vendor-specific adapters when demanded; generic “transparent replication” is not worth claiming because consistency and failover semantics belong to the selected database)* | v13 research |
| **M39** | Optional self-hosted Rullst Gateway and load balancer | `[ ] Not implemented — abandoned` *(decided on 8 October 2026: not worth building. A Rullst-owned `rullst-gateway` crate/binary will not be developed. Self-hosted deployments use an established reverse proxy configured by generated files instead: `cargo rullst deploy` writes a Caddyfile and the [two-replica deployment contract](docs/src/deployment-acceptance.md) exercises Caddy; generated nginx configuration is planned. Readiness/drain signals stay in `rullst-core` for any proxy to consume. Parity with a managed global cloud load balancer remains external infrastructure. The idea stays recorded here and in the `[ ]` count.)* | Abandoned (replaced by generated proxy configuration) |
| **M40** | Isolated programming labs and learning-game execution | `[~] Experimental implementation — bring your own runner` *(experimental `rullst-labs` contracts, in the 13.0 release inventory, provide durable submissions, exact grading, cancellation, recovery and retention, plus a documented [controller contract](docs/src/labs-runner-contract.md) and a non-executing example controller. The separately deployed Linux Rust/Wasmi `rullst-labs-runner` candidate passed 26 isolated checks and source admission in PRs #228 and #239, but was removed from the 13.0 workspace on 30 September 2026; its source remains in git history. Applications deploy and review their own runner; final release acceptance of `rullst-labs` remains pending. The web process never executes learner code or receives a container control socket. Broader language packs and offensive CTF infrastructure remain separate; see the dedicated roadmap.)* | v13 research/foundation |
| **M41** | Privacy defaults and proportional age assurance | `[~] Initial foundation` *(opt-in `rullst-privacy` (first published in the 13.0.0-alpha.1 prerelease) age policies, signed evidence, asynchronous replay claims, trusted-clock rechecks, shared-local SQLite and optional PostgreSQL storage across application hosts with focused real-database tests; native declarations, versioned optional consent and generated SaaS preference/profile-export journeys have focused evidence (the LMS consumers were retired with the complete Academy scaffold). The package candidate adds optional facade features and passed hosted/archive source acceptance in PR #221; final release admission, live age providers, guardian verification, broader rights/retention and reviewed regional profiles remain open. See the [privacy roadmap](docs/src/privacy-age-assurance-roadmap.md). No automatic worldwide compliance claim.)* | v13 P0 |

## Audit of the detailed crate roadmaps

The per-crate roadmaps (`rullst-*/ROADMAP.md`) are detailed design backlogs;
their current interpretation is the
[detailed crate roadmaps table](docs/src/capability-ledger.md#detailed-crate-roadmaps)
in the capability ledger.

## Future

Long-term ideas worth keeping; none is scheduled. Each needs a maintainer,
focused tests and an acceptance environment first. The
[capability ledger](docs/src/capability-ledger.md) holds each idea's status
and recommendation;
[capability status](docs/src/capability-status.md#claims-that-are-impossible-as-framework-guarantees)
lists the absolute claims no framework can honestly make.

- **AI-assisted maintenance:** blueprint-specific `AGENTS.md` instructions and
  one versioned discovery schema for routes, controllers, models, policies and
  source locations (today `inspect schema` and `inspect routes --json` cover
  models and recognized routes); see the
  [AI maintainability roadmap](docs/src/ai-maintainability-roadmap.md).
- **Architecture:** one canonical security stack in `rullst-security`, with
  only bootstrap contracts in Core; a documented dynamic-dispatch boundary for
  runtime-selected providers; `#[non_exhaustive]`, fallible builders and
  `impl Into<String>` completed under SemVer review, not by a mass rewrite.
- **Security and identity:** normative WebAuthn/FIDO2 conformance before a
  stable passkey claim; key rotation and Cloud KMS through provider-neutral
  envelope and key-version contracts; distributed rate limits and durable
  tamper-evident audit sinks; reputation and SIEM connectors that never show an
  unconnected source as healthy; adaptive WAF and eBPF containment only as
  opt-in research; compliance building blocks reviewed by counsel
  ([legal compliance roadmap](docs/src/legal-compliance-roadmap.md)).
- **Data and integrations:** remote brokers (RabbitMQ, Kafka, NATS, SQS/SNS,
  GCP Pub/Sub, Pulsar) on the `rullst-messaging` contract after
  provider-specific fault evidence; direct presigned uploads and image
  resizing; further mail transports only with a maintainer and the shared mail
  contract suite; explain-and-approve index advice (M8) and vendor-specific
  replicas (M38) instead of automatic production DDL or generic replication.
- **IoT, edge and AI:** MQTT, CoAP and LoRaWAN transports, firmware
  download/flash/rollback, hardware-in-the-loop CI and audited HSM/PQC
  adapters for named hardware; capability-typed JSON Schema enforcement;
  read-only, approved assistance instead of autonomous production writes.
- **Separate programmes:** aerospace, autonomous-vehicle and defence work (M31)
  and homologated NFS-e issuance belong outside the framework.

## Verification roadmap

Assurance experiments that are not current guarantees. The workflows that run
today are described in [WORKFLOWS.md](WORKFLOWS.md); an experiment moves there
only when its code and evidence exist. The goal is stronger, reproducible
evidence, not more badges or absolute claims.

| Experiment | Status and plan |
| :--- | :--- |
| Differential database testing | **Planned for the 13.0.0-rc.1 window (test-only).** Run the same generated ORM operations on SQLite, PostgreSQL, MySQL and MariaDB and compare normalized results, errors and transaction behaviour; keep provider-specific semantics explicit instead of forcing false equivalence. |
| Cross-browser and accessibility testing | **Planned for the 13.0.0-rc.1 window (test-only).** Exercise the generated starters with Playwright on Chromium, Firefox and WebKit, add automated accessibility checks and keep traces and screenshots of failures. Complements ZAP and the server-level tests; it is not WCAG certification. |
| `cargo-careful` | **Planned for the 13.0.0-rc.1 window (test-only)** as a weekly job. |
| Loom and Shuttle | **13.x, after 13.0.0.** Only for specific shared-state primitives with explicit concurrency invariants: the rate limiter, the Login Jail and queue leases. Not applied to the whole workspace. |
| Chaos testing with `fail-rs` | **13.x, after 13.0.0.** Fault injection around queues, database retries and provider timeouts, on top of the existing deterministic failure contracts. |
| `cargo-vet` dependency review | **13.x, after 13.0.0.** Start by importing published audits (Mozilla, Google, Bytecode Alliance), then define review ownership and criteria; an empty policy file would add ceremony without assurance. |
| Live provider sandbox conformance | **13.x, after 13.0.0.** Opt-in suites against Stripe test mode, Resend and OAuth providers, recording provider, version and region. Deterministic offline fixtures stay the contribution baseline. |
| Verus proofs for selected production contracts | **13.x: hosted acceptance.** A locally verified candidate exists: `verus.yml` is manual-only, with a pinned archive, compiler and solver, a syntax-linked `AgePolicy::permits` projection, one verified predicate, three mandatory postcondition-failing controls and source/resource receipts. Normal CLI CI checks the source linkage without installing Verus. The pilot stays outside release admission and adds no public crate; Auth (M9), Capital (M11), age evidence/time/storage and promotion to a mandatory check (after reproducibility, reviewed assumptions, negative controls and measured CI cost) remain separate work. See the [measured scope and assumptions](docs/src/verus-roadmap.md). |
| Zero-allocation assertions | **Deferred.** Allocation claims need stable benchmarks and explicit hot paths before they become gates. |
| PGO and BOLT | **Deferred** until production profiles exist; fixed throughput gains are not promised in advance. |
| AFL.rs/honggfuzz differential fuzzing | **Deferred** until the 42 libFuzzer targets have healthy corpora and triage ownership. |
| Sigstore Cosign signing | **Deferred.** Rullst distributes crates, which already carry checksums and build provenance; reconsider for separately distributed binaries or containers. |
| Cross-platform CI acceleration | **Deferred**, under the acceptance conditions below. Rust CI already runs seven `cargo test` shards per operating system with `sccache`; nextest schedules only the coverage pass. Speed must never reduce assertions or supported-platform evidence. |
| Upstream OSS-Fuzz integration | **Needs the maintainer** (an external pull request). The `oss-fuzz/projects/rullst` draft must pass `helper.py build_fuzzers` and `check_build` for every intended target before submission; do not imply acceptance before the upstream merge. |
| Independent security review and penetration test | **Needs the maintainer** (external evidence), strongly recommended before broad production assurances. Define scope, remediation ownership and retest criteria with the [external audit kit](docs/src/external-audit-kit.md); repository automation cannot award this evidence to itself. |

TLS is a decided policy, not an experiment: aws-lc-rs is the rustls provider
for every TLS connection and ring is used only for primitives (see
[Cryptography providers](docs/src/security-architecture.md#cryptography-providers)).

A CI acceleration change is acceptable only when all of these hold:
(1) repository and generated-project builds use distinct target roots, and a
fixture cleans only its own; (2) cold and warm times, cache hits, test counts
and doctests are recorded on all three systems for equivalent commits, queue
time separately; (3) the all-feature inventory, outbox contract and Linux
live-provider matrices are preserved without weakening feature unification;
(4) caches are keyed by OS, compiler, lockfile and profile, bounded, never
evidence, and not writable by untrusted pull requests; (5) nextest replaces
`cargo test` only with equivalent semantics, separate doctests and `cargo test`
kept as recovery; (6) promotion follows repeated green cold and warm runs on
every OS with a material gain, no new flakes or lost tests, and a rollback path.

## Release strategy

### Path to 13.0.0

13.0.0-alpha.4 froze the features and public API of 13.0
([changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha4---2026-10-31)).
13.0.0-rc.1 carries fixes for defects found in alpha.4, documentation
corrections and the test-only checks planned for its window in the
[verification roadmap](#verification-roadmap); it adds no feature. 13.0.0 follows once rc.1 leaves no open defect in the
stability scope, and the [maturity tiers](docs/src/maturity.md) and
[13.x support window](docs/src/compatibility-policy.md#supported-release-window)
apply from it. The September 2026 execution order (verification efficiency,
the 12.1.0 update experience, then v13 product work) is complete; its
acceptance boundaries remain in the [delivery plan](docs/src/v13-delivery-plan.md)
and the [priority revision](docs/src/v13-priorities.md).

### Planned after 13.0.0-alpha.4

Work items for later prereleases, not shipped features. They add no milestone
to the counts above.

- **13.0.0-rc.1: fixes and documentation only.** 13.0.0-alpha.4 froze the
  features and the public API of 13.0. rc.1 adds no feature: it carries fixes
  for defects found in alpha.4 (by its release admission, the first-hour and
  deployment jobs and the platforms upgraded to it) and documentation
  corrections. A change to a `.github/public-api/` listing must be such a fix
  and is listed in the changelog and the migration guide. The
  [maturity tiers](docs/src/maturity.md) state which packages the 13.x
  promise covers. After rc.1, changes are limited to defects found in the
  release candidate.
- **Turso/libSQL beyond the blank and API starters**, only if users ask for it
  and the maintenance cost stays low. Turso can be the primary database only
  in those starters and does not work with `#[derive(Orm)]`, `Schema`,
  `artisan!`, `Server::with_db`, Nexus or Studio. A partial bridge (`Schema`
  emitting Turso migrations, `#[derive(Orm)]` also generating the Turso model)
  could be a 13.x candidate on that condition. Full ORM parity is a decided
  non-goal (see "13.x minor candidates and decided non-goals").
- **Feedback from the platforms upgraded to 13.0.0-alpha.3 and alpha.4.**
  Their reports choose the fixes and priorities of the next prerelease, as
  the alpha.1 and alpha.2 reports did before.
- **Then 13.0.0**, once rc.1 leaves no open defect in the stability scope.

### Published and planned release lines

| Version | Status | Honest scope |
| :--- | :---: | :--- |
| **v12.0.0** | `[x] Published stable` | Tag `v12.0.0` at `eb11f892` completed the protected release workflow and published all sixteen packages on September 15, 2026. |
| **v12.x** | `[x] End of life` | Ended on 16 October 2026 with 12.3.0 as the last release; the `v12` branch is frozen. New 12.x projects no longer resolve dependencies after crates.io yanked the `zip` releases its Studio needs; use v13. |
| **v12.1.0** | `[x] Published compatible minor` | All sixteen packages were published from `b62390b4` on 20 September 2026 UTC. Guided CLI/project updates, SaaS/Nexus fixes, account mail and the other bounded contracts are recorded with immutable source and registry evidence in the [publication record](docs/src/v12.md#1210-published-maintenance-release). |
| **v12.1.1** | `[x] Published patch` | All sixteen packages were published from `d27db26c` on 24 September 2026 UTC; see the [publication record](docs/src/v12.md#1211-published-maintenance-release). |
| **v12.1.2** | `[x] Published patch` | Tagged `v12.1.2` on 28 September 2026 UTC from the `v12` maintenance line; its publication evidence is kept in the [frozen v12 release record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md). |
| **v12.2.0** | `[x] Published compatible minor` | Tagged `v12.2.0` at `565e222a` on 3 October 2026 from the `v12` maintenance line; see the [frozen v12 release record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md). |
| **v12.3.0** | `[x] Latest published stable` | Tagged `v12.3.0` at `67b2d0e5` and published on 9 October 2026: hickory 0.26.3, fewer WAF/RASP false positives and deprecations of the Capital and Mail APIs that 13.0 removes. It is the last 12.x release: the `v12` line reached end of life on 16 October 2026 and no further patch will be published. |
| **v13.0.0-alpha.1** | `[x] Published prerelease` | Tag `v13.0.0-alpha.1` at `fc44bf76`; all twenty packages were published on 11 October 2026 for evaluation. See the [delivery plan](docs/src/v13-delivery-plan.md). |
| **v13.0.0-alpha.2** | `[x] Published prerelease` | All twenty packages were published on 17 October 2026 from tag `v13.0.0-alpha.2` at `0daf4941` ([release run 39041726912](https://github.com/Rullst/Rullst/actions/runs/39041726912)): the fixes from the alpha.1 dogfooding report and the breaking `html!` → `Markup`, `Schema` date/time and Nexus changes. A first tag at `669befe6` failed release verification before publishing anything and was replaced. |
| **v13.0.0-alpha.3** | `[x] Published prerelease` | All twenty packages were published on 24 October 2026 from tag `v13.0.0-alpha.3` at `be336833` ([release run 40129821475](https://github.com/Rullst/Rullst/actions/runs/40129821475)) after all 25 required workflows passed on that commit. See the [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha3---2026-10-23). |
| **v13.0.0-alpha.4** | `[ ] Release candidate` | Prepared on 29 and 30 October 2026 for a tag on 31 October and not yet published; the coordinator records the tag, release run and publication here. Twenty packages; the 13.0 feature and public API freeze. See the [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha4---2026-10-31). |
| **v13.0.0-rc.1** | `[ ] Planned` | Fixes and documentation only after the alpha.4 feature and API freeze. See [Planned after 13.0.0-alpha.4](#planned-after-1300-alpha4). |
| **v13.0.0** | `[ ] Planned stable` | Follows once rc.1 leaves no open defect in the stability scope; the [maturity tiers](docs/src/maturity.md) apply from this release. |
| **v13.x** | `[ ] After 13.0.0` | Compatible capabilities continue in 13.1 and later minors: generated-project coverage, auth/session consolidation, typed SDKs, selected adapters, security-stack consolidation and research-heavy architecture all require fresh acceptance boundaries. |

The framework may call a milestone implemented only when the same commit passes
the repository's formatting, strict lint, full-test, feature-matrix, security, and
packaging gates. Performance numbers must cite a reproducible benchmark; security
and compliance claims must state their threat model and evidence scope.

### 13.x minor candidates and decided non-goals

Recorded on 30 October 2026 with the maintainer. Minor releases may add these
without breaking the 13.x API ([support window](docs/src/compatibility-policy.md#supported-release-window));
none is scheduled, and real user demand orders them.

- **Production monitor** (13.1 candidate): desktop notifications in the
  standalone Studio, remote traces and security radar, and proxy support.
- **`green` on macOS and Windows**: measure idle memory and CPU time there too
  (for example through `sysinfo`), not only through Linux `/proc`.
- **`OtaManager` getters** (Experimental tier): private fields with getters of
  the same names, as announced in 13.0.0-alpha.4.
- **Unquoted text in `html!`**: an ergonomic grammar addition, only if users
  ask for it; quoted literals keep working.
- **More i18n locales and currency formats**, added as CLDR data when requested.

Decided non-goals (not planned, so they are documented as scope rather than as
pending work):

- **Full ORM parity for Turso/libSQL.** It would need a second database client
  path beside SQLx for every ORM feature (`#[derive(Orm)]`, `Schema`,
  migrations, Studio and Nexus pages), doubling the maintenance and test
  matrix. Turso stays a bounded primary database for the Blank and API
  starters with its own client.
- **Reading the CSRF `_token` from any position of a multipart body.** It would
  mean buffering uploads before the CSRF check. Generated forms already send
  `_token` first, and clients can send the `X-CSRF-Token` header.

### Safe update experience

**Delivered for the documented compatible-update scope** in 12.1.0 and
narrowed in 13.0.0-alpha.4, which removed the self-updater's download and
binary replacement. A once-a-day notice prints the update command,
`cargo install cargo-rullst --version =X --locked`. Project preparation,
verification, review, digest-approved application and recovery
(`update project …`, `update guided`) remain, and `cargo rullst upgrade`
reports v12 → v13 findings (`rullst-upgrade-rules-v4`) linked to
[migration-guide](docs/src/migration-v13.md) rows. See
[assisted framework upgrades](docs/src/tutorials/36-assisted-framework-upgrades.md),
the [CLI reference](docs/src/cli_reference.md) and the 12.1.0
[publication record](docs/src/v12.md#1210-published-maintenance-release).
Still open:

- Measure cold and warm discovery, compilation and validation separately on
  published-package and generated-project fixtures, offline and with
  unsupported targets and failed migrations.
- Interruption evidence covers child kills, file-size limits and injected
  faults, not power loss or a complete disk-full matrix.
- Major-version migration stays a reviewed list of findings; file recovery
  does not undo database changes, test side effects or external services.
  Instant upgrades and zero downtime are not promised.

### API documentation quality

**Planned for v13, improved with each contract.** The
[API navigation index](docs/src/api-reference.md) links guides to
exact-version crate references and lists the REST walkthrough gaps. Still
open: per-interface reference pages for the facade, Core/routing,
Auth/Security and ORM first (purpose, features, arguments, typed errors,
security and concurrency constraints, runnable examples, version and migration
notes), with [Qt's reference navigation](https://doc.qt.io/qt-6/reference-overview.html)
as organizational inspiration only; and one REST learning path (validation,
errors, CRUD, pagination, authentication, owner/tenant authorization, OpenAPI,
tests) validated against the documented features, including denied access.
A green book build does not prove reference coverage.

### Ideas recorded for after 13.0

Recorded on 8 October 2026; ideas, not commitments.

- **Offline IoT/health example:** sensor readings stored in SQLite and shown on
  a dashboard without internet, built on the experimental `rullst-iot`
  (`SensorTelemetry`, `AnomalyDetector`) and a maintained transport crate such
  as `rumqttc`. A health variant treats readings as sensitive personal data
  and is not a medical device.
- **Multiplayer quiz example:** Core realtime and presence (`BroadcastManager`,
  `PresenceTracker`) with `rullst-labs` grading for server-authoritative,
  idempotent scoring.
- **Reverse-proxy configuration instead of a gateway:** the self-hosted
  `rullst-gateway` (M39) is abandoned; extend the generated Caddy output and
  add nginx, using the readiness/drain and trusted-proxy contracts of
  `rullst-core`.

### v14 candidates

Breaking ideas raised during 13.x are collected here instead of shipping in a
13.x release ([support window](docs/src/compatibility-policy.md#supported-release-window)).
None is scheduled; v14 development starts only when they justify it.

- Make the internals hidden from the documentation in 13.0.0-alpha.4
  (`#[doc(hidden)]`) crate-private.
- Replace third-party types in public error variants and `From` conversions
  (`reqwest`, `jsonwebtoken`, `base64`, `redis`, `serde_json`) with Rullst
  error types, so fewer crates are [public dependencies](docs/src/compatibility-policy.md#public-dependencies).
- Make the `dashmap` fields of `rullst_security::SecurityStore`
  (`banned_ips`, `honeypot_route_hits`) private, behind methods.
- Keep one path per item: remove `rullst::rullst_orm` (same as
  `rullst::orm`), `rullst::security_runtime` (same as
  `rullst::security::runtime`) and Mail's alias modules `rullst_mail::mail`
  and `rullst_mail::drivers::mock` (same as the crate root and
  `rullst_mail::mock`).

---

<div align="center">
  <p><i>"All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ)."</i></p>
</div>
