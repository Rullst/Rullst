<!-- Read this specification before generating framework code or documentation. -->

# Rullst Specification 📄
### *"The Single Source of Truth (SST) for Framework Architecture & Conventions"*

This document is the **Single Source of Truth (SST)** for the **Rullst Framework**. It specifies the exact conventions, API structures, naming rules, directory standards, and subsystem maturity lifecycles across all monorepo crates.

> [!IMPORTANT]
> **AI & Human Alignment Directive:**
> Whenever updating, refactoring, or generating code/documentation for Rullst, **always** refer to this specification as the baseline. 
> Every capability in the framework is strictly tagged with its implementation lifecycle status:
> - 🟢 **`[Implemented / Bounded]`**: A defined implementation exists with automated tests for the stated scope. This is not a deployment, provider-homologation, or certification claim.
> - 🟠 **`[Partial]`**: Useful foundations exist, but a named interoperability, architecture, or conformance boundary is still incomplete.
> - 🟡 **`[Offline Test Mock]`**: Deterministic offline sandbox fixtures for local development and offline CI without external API dependencies.
> - 🔵 **`[Roadmap / Under Development]`**: Planned architecture, traits or domain models; consult each entry for what exists and what remains unimplemented.

---

## 📂 1. Directory Structure Conventions

A standard Rullst application scaffold strictly adheres to this folder hierarchy:

```text
my-app/
├── src/
│   ├── controllers/      # Route controllers (async request handlers)
│   │   └── mod.rs
│   ├── models/           # Active Record & Repository Models (rullst-orm entities)
│   │   └── mod.rs
│   ├── pages/            # Shared HTML views, templates, and layouts
│   │   └── mod.rs
│   ├── middlewares/      # Custom application middleware layers
│   │   └── mod.rs
│   └── main.rs           # Application entrypoint, server bootstrap & central routing
├── Cargo.toml            # Project cargo dependencies
└── Rullst.toml           # Framework configuration (database, environment, secrets)
```

---

## 🛠️ 2. Naming Conventions

To guarantee consistency, both humans and AI coders must adhere to the following naming normalization rules:

* **File Names:** Standard Rust `snake_case` (e.g. `users_controller.rs`, `post_model.rs`, `billing_service.rs`).
* **Struct / Model / Trait Names:** Standard `PascalCase` (e.g. `UsersController`, `PostModel`, `PaymentProvider`).
* **URL Paths:** Lowercase kebab-case (e.g. `/users`, `/user-profiles`, `/billing/webhooks`).
* **Database Identifiers:** Snake case (e.g. `user_id`, `created_at`, `billing_accounts`).

---

## ⚡ 3. Framework Crates & Capability Matrix

The [v13 maintenance scope decision](v13-maintenance-scope.md) governs future
investment and proposed extension boundaries. It does not describe an already
completed package split or change current APIs, defaults, support obligations
or publication inventory. The implemented capability matrix below remains the
source of truth until individual migrations are reviewed and validated.

| Crate | Responsibilities | Status & Capabilities |
| :--- | :--- | :--- |
| **`rullst-core`** | Kernel HTTP runtime, `routes!`, Server bootstrap, HTML engine, async task queues, WebSockets, circular telemetry buffers, storage facade, and the default baseline CSRF/WAF/header/PII stack. | 🟢 **`[Implemented / Bounded]`**: Routing, server lifecycle, `html!` engine, graceful shutdown, backpressure guard, queues, and local storage with path-traversal protection. `ApplicationLifecycle` adds an opt-in process-local monotonic startup/ready/draining/stopped state, at most 32 immutable component readiness bits, secret-minimized `/ready`, fail-closed admission and a bounded drain wait. `Server` marks ready after binding, begins drain before Axum's graceful wait, and accepts an explicit supervisor shutdown future; deterministic tests cover startup failure, in-flight completion, rejection after drain and lock poisoning. It does not run dependency checks, coordinate replicas/load balancers or authorize domain requests. SQLite and Redis persist `dispatch_at` timestamps for at most 366 days and never claim them early; Redis promotion uses server time and a digest-pinned live CI/release contract. Execution starts on the first later worker poll and is at-least-once. Custom drivers fail closed for future scheduling until implemented. `TenantStorage`, `TenantCache`, `TenantRealtime` and `TenantPresence` bind those facades to a validated `TenantContext`, apply immutable tenant namespaces and prove same-name local non-interference; the realtime wrappers also bound channel/event/identity names and payload size. Memory and Redis caches expose an opt-in, at-most-200-entry metadata snapshot containing logical key, UTF-8 value length and remaining TTL but never the value; custom drivers fail explicitly unless they implement that method. Remote bucket policy, distributed transport/liveness, cache operator authorization and application room authorization remain deployment/application work.<br/>🟢 **`[Implemented / Bounded]`**: The in-memory upload admission contract enforces a hard size/allowlist boundary, canonical tenant/name, recognized signature versus MIME/extension, active-text denial, randomized tenant quarantine keys, SHA-256 binding and fail-closed scanner release. It is not multipart streaming, a deep parser, remote persistence or a production malware engine.<br/>🟢 **`[Implemented / Bounded]`**: Validated environment precedence is `RULLST_ENV`, legacy `APP_ENV`, then `[app].env`; invalid values fail instead of silently enabling development.<br/>🟢 **`[Implemented / Bounded]`**: `apply_security_baseline` and `Server` compose configured CSP nonce headers, exact-origin CORS with explicit credential opt-in, bounded WAF, double-submit CSRF and optional PII masking in one tested order, with the per-application config installed outside every middleware. Browser/proxy/TLS deployment evidence and application-owned session/auth/tenant/authorization remain separate. A fail-closed typed Academy boundary-assessment contract records those application observations without certifying them, and the extended `rullst-security` stack is still composed explicitly.<br/>🟢 **`[Implemented / Bounded]`**: `client_contract` exposes the portable `rullst.client` v1 typed JSON envelope, positive version negotiation, bounded correlation/idempotency/failure tokens, server-authored time and a fail-closed 2 MiB codec on native and Wasm. It deliberately contains no role, tenant or authorization assertion; durable replay and domain policy remain server/application work.<br/>🟢 **`[Implemented / Feature-gated Foundation]`**: native `offline-sync` adds bounded account state, FIFO idempotent proposals, server revisions/cursors, explicit conflicts/full resync/recovery/logical erasure, account-bound AES-256-GCM snapshots and a static-dispatch foreground coordinator with request budgets, timeout and cursor-stall checks. Platform persistence/secure-key adapters, browser offline storage, concrete authenticated HTTP/retry/background orchestration, future-schema migrations and device evidence remain application/platform work.<br/>🟢 **`[Implemented / Bounded, unpublished v13]`**: Optional `storage-s3` private S3/R2 operations passed hosted source/package admission in PR #236; final release admission and actual owner-account interoperability remain outstanding. See section 4.4. |
| **`rullst-orm`** | Active Record & Repository patterns, parameterized SQLx connection pool (PostgreSQL, MySQL/MariaDB, SQLite), typed Turso/libSQL primary profile, schema migrations, AES-256-GCM privacy, Scout search, typed pgvector/Qdrant queries, Redis native structures, and optional capability-oriented persistence adapters. | 🟢 **`[Implemented / Bounded]`**: Relational CRUD, eager loading, type-safe queries, migration runner, versioned field encryption, and connection-pool resilience for supported SQLx drivers/features. PostgreSQL, MySQL, MariaDB and SQLite have distinct executable matrix contracts, while MariaDB intentionally shares SQLx's MySQL protocol/backend.<br/>🟢 **`[Implemented / Bounded]`**: `#[derive(Orm)] #[orm(backend = "turso")]` supplies typed CRUD, equality filters, ordering, pagination/counts and generated/app-assigned keys through a process-wide `TursoOrm`. Its migrations are ordered, checksummed, drift-detecting and reversible. The blank/API CLI profile generates, compiles, migrates, reports status and rolls back locally, while the same typed contract passes against the official remote libSQL server. Unsupported SQLx-specific model behaviors fail during macro expansion rather than being ignored. Other SQLx-specific blueprints, ORM relations/hooks, schema auto-diff, seed generation and transparent embedded-replica synchronization are not part of this bounded Turso profile.<br/>🟢 **`[Implemented / Bounded]`**: The optional persistence boundary supplies portable document CRUD for MongoDB and SurrealDB, parameterized OLAP queries through in-process DuckDB, explicit parameterized Turso/libSQL SQL/transactions, and bounded read-only ISO GQL through SurrealDB. These capability APIs do not claim shared semantics or cross-store transactions. External adapters select deterministic offline behavior for empty or `mock_*` credentials where documented; SurrealDB uses its HTTP protocol rather than embedding the BSL-licensed SDK.<br/>🟢 **`[Implemented / Feature-gated]`**: `scout-http` provides bounded Meilisearch, Elasticsearch and Algolia indexing/search adapters plus deterministic mocks. Meilisearch has a digest-pinned live lifecycle; Elasticsearch/Algolia have protocol fixtures, not hosted-provider certification. Generated projections are process-local post-commit effects unless the application explicitly composes the transactional outbox.<br/>🟢 **`[Implemented / Feature-gated]`**: `pgvector` with `strict-postgres` supplies typed SQL vector helpers. `qdrant` supplies a separate bounded dense-vector collection/upsert/delete/cosine-query contract, while `redis` supplies namespaced Hash, Set and Sorted Set operations. All three have digest-pinned live lifecycles; RAG orchestration, authorization, production ANN tuning and Redis cluster/failover remain application/deployment boundaries.<br/>🟢 **`[Implemented / Benchmark Evidence]`**: A lockfile-pinned Criterion target compares five equivalent typed-SQLite shapes through one Rullst, Diesel and SeaORM connection under the same schema, seed and SQLite policy. It is per-run evidence, not a superiority, negligible-overhead, networked-database or full-application claim. |
| **`rullst-auth`** | Argon2id password hashing, encrypted cookie sessions (AES-256-GCM), opt-in application JWTs, Passkey ceremony foundations, RBAC context guards. | 🟢 **`[Implemented / Bounded]`**: Non-blocking `spawn_blocking` Argon2id hashing, versioned expiring AES-256-GCM sessions, fail-closed `RequireRoleLayer`, compile-validated `#[rullst::require_role]`, named `Policy<User, Resource>` decisions, and a feature-gated application JWT policy with required versioned claims, bounded TTL/scopes, strong HS256 keys, `kid` rotation and revocation contracts that reject process-local state in production mode.<br/>🟢 **`[Implemented / Feature-gated]`**: `sqlite` supplies bounded shared local auth state. `SqliteJwtRevocationStore` persists JTI expiry and monotonic subject session versions through serialized transactions, stored quota/configuration and async verification. `SqlitePasskeyStore` persists validated public credentials, bounded device inventory/rename/revocation and optimistic signature-counter CAS; executable restart, replay, quota, corruption/configuration and two-instance contention evidence covers both stores. Authentication, role persistence, resource/tenant/device ownership, trusted file permissions/encryption, backup and multi-host replication remain application/deployment boundaries.<br/>🟠 **`[Partial]`**: Passkey registration/assertion validates the documented ES256/`none`-attestation scope; the compatible `PasskeyAuth` challenge state remains process-local. The optional `passkey-postgres` v13 candidate adds a separate shared manager with bound single-use PostgreSQL ceremonies; independent-process/database-restart and Chromium virtual-authenticator contracts pass, and PR #223 passed hosted workspace/archive source acceptance. Final release admission remains separate. The v13 opaque-session inventory/logout increment extends the recovery store with process/database evidence and passed hosted source/package admission in PR #236. Final release admission remains separate. Normative WebAuthn conformance or adoption of an audited full server library, refresh tokens and complete recovery/session UX remain required before a general stable claim. |
| **`rullst-security`** | Explicit extended defense-in-depth layers: bounded RASP, authenticated Vault, Login Jail, Secure Headers, rate limiting, DLP and security telemetry. | 🟢 **`[Implemented / Bounded]`**: AES-256-GCM envelopes with rotation/AAD, bounded URI/header/body RASP heuristics, local abuse controls, CSWSH origin guard, OS-random TOTP with SVG enrollment QR, strict JSON transport inspection plus an explicitly mounted reusable JSON Schema 2020-12/OpenAPI 3.1-component policy, explicit log redaction, file-backed SRI hashes, and a versioned/bounded `LiveSecurityEvent` v1 dashboard envelope. `DurableSiemSpool` preserves the compatible unsigned local format, while `AuthenticatedSiemSpool` offers an explicit HMAC-SHA256-chained format with named active/historical keys, zeroized key material, sequence/predecessor validation and byte/record quotas. Restart, forgery, wrong/missing keys, reordering, interior deletion, quota, symlink and external-length-change paths fail closed. Whole valid-tail rollback requires a separately trusted checkpoint, and the caller owns directory/key trust, permissions, retention and exclusive-writer operation. Schema construction caps bytes/nodes/depth, accepts only local references, disables network/filesystem resolution and uses linear-time regexes; auth/ownership/domain rules and query/header/form validation remain application contracts. A deterministic Sentinel classifies three caller-supplied aggregate patterns and can issue HMAC-authenticated, subject-bound, expiring, one-shot process-local proof-of-work challenges; it is not AI attribution, automatic blocking or distributed replay protection. The CLI emits bounded fail-closed evidence and a CycloneDX 1.5 Cargo SBOM; it does not certify the application.<br/>🟢 **`[Implemented / Feature-gated]`**: `redis-rate-limit` provides namespaced atomic Redis fixed-window counters, hashes client keys and exposes an explicit process-local offline mode that production can reject with `require_distributed()`.<br/>🟠 **`[Partial]`**: Recovery-code consumption must be persisted transactionally by the application. Real Redis cross-instance/eviction/failover evidence is still required. CSP nonce composition is shared, but Core and Security are not yet one canonical Server stack; WebSocket CSRF tickets/frame crypto, trusted rollback checkpoints, spool compaction/remote acknowledgement and external SIEM delivery are not implemented. |
| **`rullst-ai`** | Multi-provider LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama, explicit OpenAI-compatible endpoints), prompt injection defenses, PII masking, bounded tenant-aware RAG, guarded local tools, and conversational memory. | 🟢 **`[Implemented / Bounded]`**: Guarded `AiClient`, heuristic prompt filter, PII masking, machine-readable provider capabilities, configurable bounded live-request deadlines, a versioned deterministic injection/jailbreak/PII regression corpus, and a capability-declared OpenAI-compatible adapter. The adapter separates literal-loopback-IP HTTP(S) from HTTPS cloud configuration, supports optional Bearer authentication, disables ambient proxies/redirects, and bounds image/response bodies; unrelated protocols use `AiProvider`. A separate static-dispatch `StreamingAiClient<P>` enforces chunk/output ceilings and explicit cancellation; exact OpenAI-compatible configurations may declare strict incremental SSE with a required terminal marker and cancellation raced against request/body reads. `AdaptiveAiEvaluator<P>` runs caller-defined multi-turn strategies with independent turn/prompt/response/deadline limits, cancellation, typed pass/fail/inconclusive decisions and a versioned report that retains no raw prompt, response or provider error. Repository fixtures prove orchestration, not live-model behavior.<br/>🟢 **`[Implemented / Bounded]`**: Strict URL/resolved-IP/redirect/resource policy plus an opt-in deny-by-default HTTPS fetcher with exact-host allowlist, DNS pinning, proxy bypass, peer verification and streaming limits. Explicit vision helpers accept application-admitted bytes, canonical exact-root local files or URLs only through that fetcher; capability and prompt checks precede I/O, input is capped at 10 MiB, supported image signatures are sniffed and a supplied remote media type must match. Local tool dispatch separately requires allowlist, principal authorization, closed bounded JSON, call budget, audit sink, and payload-bound approval for destructive/financial calls. `RagPipeline::answer` composes guarded embedding, a static-dispatch application retriever, Unicode-safe context budgets, guarded generation, source metadata, and required secret-minimized terminal audit under a trusted `TenantContext`; a bounded tenant-partitioned process-local cosine retriever supplies the offline contract. `DurableRagAuditTrail` and `DurableToolAuditTrail` synchronously append minimized events to distinct versioned local files under byte/record quotas and fail closed on restart corruption, symlink targets, competing-writer growth or durability uncertainty. Their SHA-256 frames detect corruption but do not authenticate events. The separate opt-in `AuditDeliveryClient` exports a caller-minimized, at-most-16-KiB JSON envelope with an exact HMAC-SHA256 signature, key/timestamp metadata, stable event identity across bounded transient retries, explicit cancellation and a closed event-bound acknowledgement. Cloud delivery requires HTTPS and literal-loopback HTTP(S) is development-only; the receiver still owns freshness verification, deduplication, authorization, persistence, retention and key operations.<br/>🟢 **`[Implemented / Feature-gated]`**: `StatefulChat<M>` loads bounded tenant/conversation history, performs guarded generation and atomically appends one user/assistant exchange through a static `ChatMemory`. The bounded in-memory store is always available; `sql-memory` supplies fixed-schema SQLite/PostgreSQL/MySQL/MariaDB storage with an even monotonic revision and transactional compare-and-swap, so stale cross-process writers fail instead of silently reordering. Raw message encryption/retention, authenticated ownership within a tenant, provider audit, backups and conflict UX remain application contracts; the CLI scaffold remains the Turso/custom-model path.<br/>🟠 **`[Partial]`**: The egress fetcher is not automatically mounted around provider transports, RAG or arbitrary application clients; hosted-provider SSE conformance, non-compatible streaming protocols, image decoder safety, host path trust/authorization, provider-native tool calling, cancellation for ordinary non-streaming calls, automatic provider retries, durable audit outbox/receiver operations, approver authentication, first-party external vector-store retrievers, authoritative datastore/domain authorization, ingestion/deletion, maintained application-specific eval corpora and output policy remain application or roadmap work. Exact live-model evaluation execution/results and provider behavior remain external evidence.<br/>🟡 **`[Offline Mock]`**: Deterministic offline chat/vision/embedding fallbacks. |
| **`rullst-capital`** | Multi-gateway billing, SaaS MRR/ARR metrics, constant-time webhook signatures, contractor payouts, and a bounded National NFS-e preparation pipeline. | 🟢 **`[Implemented / Bounded]`**: Provider-specific payment/payout adapters, pooled HTTP clients, explicit mock credentials, and signature/freshness/replay foundations for the methods documented by each adapter.<br/>🟢 **`[Implemented / Feature-gated]`**: `webhook-sql` persists bounded provider-scoped payload digests or caller-supplied stable event identifiers across processes on SQLite, PostgreSQL, MySQL, and MariaDB. Immutable capacity/TTL, serialized claims, expiry, restart, contention, configuration drift, and fail-closed full/storage states have executable evidence. A caller-owned relational transaction can bind one semantic event claim to its domain mutation. Middleware admission and cross-system exactly-once are not implied; external effects still require an outbox, idempotent consumers, and reconciliation.<br/>🟢 **`[Implemented / Bounded]`**: Static-dispatch metered billing uses the current Stripe Meter Events and Lemon Squeezy Usage Records request shapes, binds accepted responses to the original event, caps response bodies and exposes deterministic non-live mocks. Stripe forwards a bounded provider identifier; Lemon Squeezy explicitly requires application-outbox deduplication.<br/>🟠 **`[Partial]`**: Uniform live method coverage, provider-account interoperability, cross-system exactly-once, and reconciliation are incomplete; Alipay RSA2 fails closed.<br/>🟢 **`[Implemented / Feature-gated]`**: `nfse` pins the current official 1.01 production/restricted artifact profiles by SHA-256, builds a strict ordinary-service DPS subset without floating-point money, and validates extracted official XSD sources from a closed in-memory catalogue. After hash verification, the production profile receives exactly one declared compatibility normalization: .NET-style `^...$` anchors are removed from the known DPS-series pattern so the XSD-regex engine applies the authority's apparent intent instead of treating the anchors as literals. The same feature signs `infDPS/@Id` with PKCS#12 RSA-SHA256/inclusive-C14N 1.0, verifies its local XMLDSig test fixture, and constructs a bounded rustls mTLS identity/client. Its offline protocol codec requires the signed `tpAmb`, emits the exact `dpsXmlGZipB64` JSON object deterministically and parses bounded synchronous 201 authorization or 400/403/500 rejection responses, binding environment, submitted DPS, access key and a cryptographically valid embedded NFS-e XMLDSig. A single-active-writer HMAC-chained local command journal records idempotent prepared/terminal digests, recovers unresolved descriptors after restart and supports independently retained exact-tip checkpoints without storing XML, access keys or response messages. Certificate secrets are redacted and zeroized where owned by Rullst.<br/>🟡 **`[Offline Mock]`**: Deterministic `NfseEnvironment::Mock` fixture, unmistakably not a tax authorization.<br/>🔵 **`[Roadmap / External Evidence]`**: Live transmission, full emitter-certificate/ICP-Brasil trust policy, authoritative request/outbox and multi-writer operations, restricted-environment certificate tests, independent review and SEFIN homologation. Homologation/production transmission remains fail-closed. |
| **`rullst-connect`** | Social login / OAuth2 / OIDC providers (Google, Apple, GitHub, Discord, Auth0, Cognito) with PKCE and rotating JWKS. | 🟢 **`[Implemented / Bounded]`**: OAuth2/OIDC clients with constant-time PKCE comparison, validated discovery, bounded JWKS refresh/cache policy, deterministic mock credentials and a credential-free `UniversalProfile` projection. `ConnectUser` serialization omits access/refresh tokens. Category-aware remote revocation rejects malformed or oversized tokens before transport: Google, Discord and Apple accept the documented access/refresh categories, GitHub accepts access tokens, and Auth0/Cognito accept refresh tokens; protocol fixtures bind method, endpoint, client authentication and form/JSON shape, while request/response `Debug` omits credentials, bodies and URL query data. Other providers fail explicitly as unsupported, and remote success does not clear application sessions or persistence. `AutoRefreshingSession<P>` validates and user-binds token generations, detects expiry with a bounded early-refresh window, serializes provider refresh through static dispatch, retains/rotates refresh credentials and swaps state only after a complete valid response; callers waiting behind a successful refresh reuse that state. Its state/leases redact secrets. `EncryptedTokenSnapshot` supplies a bounded, versioned AES-256-GCM envelope that authenticates key ID, provider and trusted local-account binding, preserves the validated generation and rejects copied-owner/tampered records. The optional `sqlite` store persists only a pseudonymous binding digest, generation/key metadata and that ciphertext under an immutable row ceiling; `BEGIN IMMEDIATE`, exact-successor compare-and-swap and conditional deletion reject stale shared-local writers, with restart, contention, quota, configuration, corruption, key and symlink evidence. The application still owns secret-manager key custody/rotation, account authorization, a lease around the remote provider call, losing-call reconciliation, retry/backoff, trusted directory/backup, reauthentication and multi-host replication. The optional Axum/tower-sessions lifecycle generates a ten-minute state + PKCE challenge, adds nonce for OIDC, keeps verifier/nonce server-side, removes and immediately saves the sole active challenge before validation and rejects sequential replay/expiry/mismatch. The host still owns durable session storage and cookie/TLS/account policy; the generic session-store API is not distributed compare-and-delete, so simultaneous already-loaded callbacks require idempotent effects or a stronger application store. `ReqwestClient` also exposes explicit HTTP(S) corporate-proxy constructors: endpoint shape is bounded, URL credentials are rejected, authenticated remote proxies require HTTPS, system-proxy lookup is disabled and a local protocol fixture proves routing/auth headers.<br/>🟢 **`[Implemented / Local Test Fixture]`**: The explicitly mounted Axum Mock IdP accepts only configured HTTP-loopback issuer/callback origins, binds one exact client, bounds process-local grants/tokens, consumes expiring authorization codes once, verifies S256 PKCE, signs nonce-bound EdDSA ID tokens and publishes discovery/JWKS. The deterministic key and credentials are public test fixtures; interactive login/consent, refresh/device/federation flows, durability, rotation, public exposure and OIDC conformance are not claimed.<br/>🔵 **`[Roadmap]`**: PAC/WPAD, SOCKS, proxy mTLS identity and enterprise deployment certification are not implied. Message brokers live in `rullst-messaging`, not this OAuth-focused crate. |
| **`rullst-messaging`** | Broker-neutral event envelopes, idempotent publication, consumer groups, acknowledgement leases, retry, dead letters, durable local SQLite state, and future remote adapters. | 🟢 **`[Implemented / Bounded Foundation]`**: `rullst.messaging.v1` envelopes, bounded identifiers/headers/payloads/batches/retention, topic-scoped exact-replay idempotency, fan-out between groups, competing consumers, expiring single-use ACK leases, bounded retry/attempt ceilings, dead-letter views, explicit terminal purge, injectable time and a reusable static-dispatch contract suite. Debug output redacts keys/tokens/header values/payloads. A canonical bounded v1 envelope codec rejects unknown versions, non-canonical/truncated/oversized frames and namespace mismatch; a deterministic digest fixture freezes its bytes. Validated W3C version-00 `traceparent` and a conservative `tracestate` subset propagate through only those two allowlisted headers; arbitrary baggage, sampling and export remain host work. `InMemoryBroker` remains deterministic/process-local. The opt-in `SqliteBroker` uses a fixed schema and serialized `BEGIN IMMEDIATE` mutations for publications, subscriptions, claims, ACK/retry/DLQ and purge; exact limits are persisted per namespace. Its explicit AES-256-GCM profile encrypts header values plus payload with randomized nonces and AAD binding to immutable row metadata. A bounded primary/prior-key ring rejects missing keys until old records are purged, and plaintext/encrypted profiles cannot mix. The opt-in static `OrmOutboxRelay` binds one relational outbox stream to one topic, validates claimed JSON, publishes the durable event key as broker idempotency and only then ACKs the exact ORM lease; a publish-before-ACK crash/reclaim test produces an exact replay and one broker message. Shared-contract, raw-storage/restart, wrong-key/tamper/row-swap, symlink, rotation, expired-lease, two-instance contention, configuration-drift and malformed-row repair regressions are executable.<br/>🟠 **`[Partial]`**: Delivery is at least once. The default profile is plaintext. Even in the encrypted profile, topic/event/content metadata, IDs, timestamps, idempotency keys, fingerprints, rotation key IDs and delivery state remain visible; key custody, permissions, backup/rollback detection, retention, disk operations, topic/tenant authorization and destination-side idempotency belong to the host. Profile migration requires a new namespace/database and application-owned republishing. The outbox database and broker publication are not one atomic transaction; worker supervision, cleanup and destination idempotency remain application work. The local adapter does not provide replication or automatic failover. The envelope codec is not a remote transport and does not preserve caller publication keys or broker acknowledgements by itself. The optional standalone Redis Streams candidate passed source admission in PR #236, with Rullst-owned indexes, verified TLS, exact replay, server-time lease fencing, partial-write quarantine and disposable restart/fault/outbox tests; final release admission remains separate (see the Redis messaging guide below). Kafka, RabbitMQ, NATS/JetStream, SQS/SNS, Google Pub/Sub and Pulsar adapters, replication/failover and native Redis group interoperability remain roadmap work. |
| **`rullst-iot`** | `no_std` sensor telemetry/protocol helpers and an Ed25519-signed firmware-manifest verification gate. | 🟢 **`[Implemented / Bounded]`**: Ed25519 manifest verification with target/hash/length/counter checks, an explicit durable monotonic-CAS store boundary, `no_std` telemetry/frame models, bounded MQTT 5 PUBLISH and RFC 7252 CoAP base-request encoders, a credential-free local HTML snapshot renderer, and a safe telemetry-module CLI scaffold. Protocol vectors and restart/retry/conflict tests prove these local contracts, not a broker, network or physical device.<br/>🟠 **`[Partial]`**: GPIO state, I2C/Modbus frames, BLE GATT records, RSSI topology, power recommendation and Digital Twin JSON are data/helpers only, not hardware, network or realtime drivers.<br/>🟡 **`[Simulador Dev]`**: Deterministic MQTT-value/HSM/PQC fixtures require `feature = "experimental-simulators"` and never represent broker or cryptographic capabilities.<br/>🔵 **`[Roadmap]`**: Concrete hardware-backed counter/boot integration, firmware download/flashing, MQTT/CoAP transports and state machines, hardware drivers/HSM and audited ML-KEM. |
| **`rullst-mail`** | Transactional email engine with Resend, SendGrid, Postmark, SendPulse, Mailjet, Mailtrap, ACS, optional SMTP, optional native AWS SES v2, and offline fixtures. | 🟢 **`[Implemented / Bounded]`**: Mandatory pre-flight pipeline, anti-CRLF validation, bounded disposable-domain/security/DLP heuristics, provider-specific transports, seven safe scaffold variants (including provenance-aware fiscal receipts and explicit D+1/D+3/D+7 dunning), and expiring purpose-bound HMAC tracking tokens. `TenantMailResolver` selects an in-process driver directly from an explicit authenticated Core `TenantContext`; invalid IDs and unavailable registry state fail closed, and tests prove two contexts do not cross-deliver. `MailError` classifies permanent/transient/rate-limit outcomes; the in-process `FailoverDriver` sends another provider only transport/HTTP 5xx/429/transient-SMTP failures, captures bounded delta `Retry-After`, fails closed on circuit-state errors and emits structured tracing without provider response bodies. `Mail::enqueue` preserves tenant and bounded due-time metadata through SQLite/Redis without early claims; the worker consumes that timestamp only after it is due. Direct Resend/SendGrid retain provider-native scheduling, while real SMTP/Postmark/Log and SES paths reject future direct delivery; offline fixtures may retain it for assertions. The shared attachment contract accepts at most 32 items, 20 MiB each and 25 MiB raw aggregate; validates safe basenames, parameter-free MIME and unique HTML-referenced CIDs; redacts bytes from `Debug`; and feeds provider-native Resend, SendGrid, Postmark, native SES and nested SMTP MIME serialization. The opt-in static `AttachmentInspectionGuard` fails before transport on executable magic, spoofed known types, active PDF/SVG, recognized secrets and unsafe text links; external scanners can implement the same contract. The provider-neutral `SuppressionGuard` checks process-local or opt-in shared-local SQLite state before transport; verified event identities are replay-bound, suppression reasons escalate monotonically and immutable quotas are transactional. `ObservedMailDriver` emits only a bounded provider label, terminal outcome, latency, attachment count and scheduling/tenant booleans through a non-failing observer. With `aws-ses`, `AwsSesDriver` sends SES v2 Simple messages through the official AWS SDK and SigV4, including temporary credentials, caller-owned rotating providers/config, HTML/text, RFC 8058 headers and attachments/CID; it rejects provider field limits and an encoded estimate over 40 MiB before network, caps `Retry-After`, and a loopback contract asserts the signed regional `ses/aws4_request` request plus typed/redacted rejection. The legacy constructor remains only an offline-fixture or explicit trusted bearer-proxy boundary, never an unsigned AWS request. Fiscal mock responses remain visibly unauthorized; dunning does not infer billing state or scheduling.<br/>🟠 **`[Partial]`**: Exact execution time, exactly-once delivery, live-account SES acceptance and inbox delivery are not implied. The local attachment inspector is not antivirus, sandboxing, recursive archive inspection or CDR; provider/account limits may be tighter. Provider webhook authentication/adapters, multi-host suppression replication, file encryption, distributed breaker/telemetry operations, durable encrypted tenant credentials, rotation and cross-process distribution remain application/deployment concerns; tracking payloads are authenticated but not confidential. SES identity/domain verification, sandbox exit, IAM least privilege, quotas, reputation and provider operations remain AWS/account/deployment work.<br/>🟡 **`[Offline Mock]`**: Memory/Log plus empty or `mock_*` provider credentials. |
| **`rullst-studio`** | Local Developer Control Room (`http://127.0.0.1:5555`), clean route navigation, live system telemetry visualizers. | 🟢 **`[Implemented / Bounded]`**: Local control center, `RadarSnapshot` telemetry, database/migration surfaces when configured, and explicit `Unavailable` states for unconnected probes. The data browser reads/filters SQLx tables and, only after the verified debug-loopback/same-origin middleware installs an unforgeable request marker, can update primitive non-key values or delete exactly one complete-primary-key-selected row. Values are bound, request/schema/value cardinality is bounded, backend-specific types remain read-only and SQLite/PostgreSQL/MySQL/MariaDB have executable mutation contracts. This is not application tenant/RBAC, audit, rollback or shared-production administration. The supplied queue snapshot exposes only backend records; SQLite can explicitly retain 1–100,000 successful jobs with atomic pruning and purge while deleting them by default. Retained payload access/policy belongs to the host. An explicitly supplied memory/Redis `Cache` exposes at most 100 metadata rows in the UI; logical keys become process-bound HMAC tokens, values never leave the driver, and only individual local invalidation is available. A separately mounted push-only trace router accepts 1–128 attribute-free v1 spans under 128 KiB after HMAC-SHA256, source/ID/clock/nonce validation and atomic replay rejection; the bounded in-process viewer derives slow-query and repeated-label heuristics without SQL or bindings. It is not OTLP, durable trace storage, a key manager or remote Studio authentication. Successful feature-flag toggles invalidate all warm `DbFeatureDriver` caches in the same process through a constant-size epoch. Cross-process/direct-writer invalidation remains TTL-bound unless the application distributes the signal. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), dynamic model CRUD, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. | 🟢 **`[Implemented / Bounded]`**: `#[derive(Nexus)]` emits registered named-field metadata with inferred primitive or explicit semantic widgets; the panel provides parameterized CRUD/search/sort/pagination plus bounded selected-record delete/deactivate. Construction is fail-closed, requires an authentication policy and admin role layer, validates bounded unambiguous model/field/enum/relation metadata, enforces server-side field policy, caps form pairs and field bytes, rejects unknown/protected/duplicate or semantically invalid form values, minimizes database errors returned to clients, and escapes record/model metadata on audited paths. Boolean widgets are inferred; enum options and multiline intent are explicit because an unrelated Rust field type does not expose those semantics to the struct derive. Deactivation requires a writable Boolean `is_active`/`active`.<br/>🟢 **`[Implemented / Opt-in Bounded]`**: a registered text `tenant` column scopes every built-in read, create, update, delete and batch operation to a trusted Core `TenantContext`; create injects the context value and missing context fails closed. `with_required_audit` transactionally couples successful mutations to a minimized fixed-schema row containing the built-in authenticated actor, optional tenant, table/action, optional known key, count, committed outcome, correlation ID, timestamp and format version; missing audit storage rolls back the mutation. The audit table is in the same relational database, mutable by its administrators, records no denied attempts, and may omit an automatically generated create key. Host identity/membership/domain policy, global-model and custom-route authorization, database privileges, schema/type compatibility, retention/backup/replication and immutable external audit delivery remain application/deployment contracts. |
| **`rullst-macros`** | Procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`) and compatibility helpers. | 🟢 **`[Implemented / Bounded]`**: Compile-time `html!` escaping with explicit `RawHtml`, model/runtime macros, and `trybuild` diagnostics. A concrete async `#[server_function]` returning `RpcResult<T>` generates a matching explicit native router and Wasm caller over the bounded `rullst.client` v1 JSON envelope: owned Serde parameters/results, same-origin `/api/rpc/...` path, 256 KiB request/response policy, request correlation, media-type/version/schema checks, CSRF-cookie forwarding and message-free failure codes. The host must mount the route inside production security, authenticated identity, tenant, authorization and rate-limit layers; application idempotency and browser/network interoperability beyond CI are not inferred. `#[island]` hydration remains experimental. |
| **`cargo-rullst`** | Developer CLI toolkit, scaffolding generators (`make:*`), project blueprints, AST IDOR static route scanner. | 🟢 **`[Implemented / Bounded]`**: Interactive wizard, generators, heuristic IDOR scanner, CycloneDX exporter, toolchain doctor and a fail-closed Academy evidence diagnostic that explicitly does not certify a deployment. Version 12 deterministic generation can explicitly select the blueprint, primary database or database-free blank profile, AI, Redis and additive persistence capabilities. It deliberately fixes generated database-backed application code to Active Record and full-stack rendering to server-side `html!` plus HTMX; Repository/Data Mapper and the LiveView, Wasm Island, Pico.css and Tera foundations remain application APIs rather than equivalent v12 generator profiles. The optional storage multi-select remains public and accepts zero or more Turso/libSQL, MongoDB, DuckDB, SurrealDB and Qdrant add-ons with their distinct capability boundaries. SQLx manifests disable umbrella defaults and select one strict primary backend. A structural gate retains 18 internal layouts: nine directly linked public shapes and nine legacy DLL regression shapes. A minimal eight-case matrix still checks legacy templates and release boundaries without advertising DLL runtime support. Seven additional public-CLI profiles exercise all six blueprints plus distinct database/AI/Redis/polyglot axes; the CLI-level polyglot case compiles while dedicated ORM matrices own adapter runtime evidence. `dash` uses bounded logs/input, probes application and Studio availability, observes its child process, reports configured rather than presumed-connected persistence, runs migrations asynchronously and restores terminal/process state on exit; neon motion is optional and has reduced-motion/color-free modes. The public development commands now use supervised process restart: coalesced source/asset/configuration changes trigger a real build, compile failures retain the current application, and successful candidates run from owned executable snapshots. A debug/development-only same-origin generation probe drives browser refresh and verifies startup identity; process state resets and shutdown is bounded. The CLI rejects legacy DLL profile generation after the Windows LMS/ORM state-split finding. The retained experimental loader is not a public v12 workflow or stable Rust ABI; see the release audit and supervised-reload tutorial. `make:chat-session` emits registered SQLx or Turso-primary models, reversible migrations and application-owned bounded chat memory; materialized contracts run persistent mock conversations on both backends and prove collision refusal. `make:billing --model` likewise emits SQLx/Turso-primary persistence plus Stripe/LemonSqueezy pricing, authenticated checkout/portal and mandatory signed-webhook code; its materialized contract compiles, migrates, persists, denies cross-owner subscription mutation before customer binding and refuses existing outputs on both backends.<br/>🟢 **`[Implemented / Bounded]`**: The LMS starter supplies bounded curriculum, school-scoped learning/assessment/publication/progress/completion, roles, leaderboard, automation/outbox/workers, localized in-app notifications and a minimized privacy-request foundation. Its SSR catalog performs limited, ORM-parameterized title/category filtering; generated auth/catalog/course/player shells consume the Core CSP nonce without remote page dependencies or inline style attributes and include keyboard landmarks, visible focus and reduced-motion handling. Lesson presentation distinguishes video/audio, rejects non-HTTPS non-local sources, requires a WebVTT track for video and a bounded transcript/language for both; materialized tests cover escaping and fail-closed negatives, not real-browser playback or caption quality. Privacy claims use exact leases, retry/dead-letter with a hard ten-attempt ceiling, actor/digest-bound completion and a supervised static-dispatch executor with an explicit protocol-only mock; the product must still supply the adapter that performs application-specific export/deletion/anonymization. Materialized SQLite exercises catalog/player escaping/nonce, privacy hard limits and the documented vertical/cross-school boundaries. Detached `--lms-modules auth`, `auth,learning` and `auth,learning,assessment` profiles remain small compiling foundations; the assessment profile grades versioned quizzes authoritatively without pulling score/leaderboard/outbox verticals. The complete starter is the default.<br/>🟠 **`[Partial]`**: Other detached combinations, profile hot reload, complete generated frontend alternatives, full Turso-primary parity beyond Blank/API, media upload/hosting/transcoding, advanced/localized search, caption/transcript quality and localization, WCAG/browser evidence, distributed failover, PostgreSQL/MySQL isolation, visual authoring, exported telemetry and the separately operated Academy remain roadmap or release-engineering work. |

### v13 roadmap package boundaries

The [`rullst-labs`](rullst-labs-roadmap.md) library and separately
deployed `rullst-labs-runner` are unpublished implementation candidates whose
source admission passed in PR #228. Final release admission and independent
isolation review remain outstanding. The named Linux execution journey and
remaining patch-coverage gap have
[recorded hosted evidence](labs-first-profile.md#recorded-linux-acceptance).
The former owns trusted, versioned orchestration and grading
contracts; the latter owns isolated execution. Neither may become a default
framework dependency, execute learner code inside the HTTP process, or require
the application to expose a container control socket. A complete offensive CTF
arena is external, separately governed deployment infrastructure even when it
uses Rullst identity, challenge, score and receipt contracts.

The selected first Labs profile is a bounded Rust pure-function exercise,
compiled with a pinned Rust toolchain to import-free WebAssembly and executed
by a pinned Wasmi interpreter in a separate restricted Linux process. This is
not a native Rullst server, Cargo dependency, WASI or arbitrary shell profile.
`rullst-labs` must not depend on an executor or spawn submitted code. Its default
surface is validated contracts; opt-in shared-local SQLite stores dedicated,
encrypted job content with current application authorization, idempotent
submission, cancellation, leased execution, retention and result reconciliation.
It must not reuse the application's authentication/database secrets as job keys.

Coverage measures the trusted controller through the same real isolated
acceptance journey, preserving identical controller/worker executable hashes.
A CI-only LLVM runtime hook initializes profile output only when the trusted
host supplies its explicit path. Workers retain the exact cleared environment
and never initialize or export counters. No extra mounts, descriptors or output
permissions may weaken isolation. Ordinary and instrumented acceptance are
distinct evidence; the hook must never enter a distributed runner.
Keep both existing 90% line-coverage floors and include the new v13 application
libraries in the framework-library aggregate; the runner remains counted in
the whole repository as a separate executable.
Controller keys require protected root/controller-owned ancestors and owned
private regular files. Validate the opened no-follow descriptor and exact key
length; a prior path metadata check alone does not bind the bytes read.

The independently deployed runner accesses only that dedicated job plane and
runner-owned tools. It must never give submitted code the job database, signing
keys, application secrets, inherited environment or control sockets. The first
Linux backend requires delegated cgroups v2, an unprivileged namespace launcher,
at most 32 job/probe groups enforced by the delegated root's kernel descendant
limit, and recovery of authenticated expired/cancelled attempts before a new
preflight needs an empty group. Recovery never releases source or grants a grade.
New source still requires successful live preflight and a current lease, plus
restricted mounts/egress, no-new-privileges, syscall restrictions, a fully enforced
Landlock filesystem policy and bounded
compiler/interpreter resources. Both processes require observed enforcement. The compiler and interpreter must enter separate
Landlock domains before source is released. Landlock does not mediate a process's
own anonymous pipes through `/proc`; compiler-to-interpreter descriptor access
must instead fail through the domain/ptrace boundary. The compiler's standard
input/output are null during compilation, with only bounded diagnostics retained.
The syscall policy permits only the `FIONBIO` ioctl request needed for Rust's
captured linker pipes; other ioctl requests and all socket creation remain denied.
Observed isolation and resource enforcement are
mandatory: accepted configuration properties or a successful launcher exit do
not prove the required boundary. Unsupported local/hosted environments fail
closed, without a less restrictive execution fallback.

Only an integrity-bound result matching the current job/lease, tenant, learner,
exercise, grader, toolchain, source and execution profile may become a grade.
Expected answers remain in the trusted grader; worker outputs are bounded data,
not a passing-grade authority. Cancellation/expiry fence late results; worker
loss never means success. Offline simulation is explicit and cannot establish
execution evidence. Authorized course maintenance must expire queued submissions
and remove their source without requiring a supported/available executor. It
must not clear a running lease or infer teardown. Withdrawn exercise revisions
may be removed only after every referencing job has been purged; immutable
revision identifiers must not be reused after removal. See
[the first-profile decision and threat model](labs-first-profile.md).
Independent isolation review and the roadmap's adversarial acceptance remain
required before any production-ready untrusted-code claim.

### v13 managed-video implementation boundary

The optional, unpublished `rullst-media` candidate owns managed private video,
starting with Bunny Stream. This differs from Core's object-storage facade:
remote video creation, upload capabilities, asynchronous processing, playback
grants and lifecycle reconciliation require a separate explicit domain.
It has no Core/Auth/ORM runtime dependency and no default network/database features.
Core is a test-only dependency for the real HTTP/browser security composition.
`bunny` selects the reviewed HTTP/signature adapter; `sqlite` selects durable
shared-local asset/operation state and static-dispatch application orchestration.
PRs #227 and #239 supply hosted source and extracted-package evidence. The
standalone package now enters the v13 release inventory, without a facade
dependency. The publication configuration requires its own hosted package
validation, reviewed initial registration and final release acceptance. Actual
Bunny account/CDN interoperability remains explicitly unvalidated.

The host supplies authenticated actor/course membership and current management
or playback entitlement through a checked authorization trait. Local asset IDs
bind an immutable tenant/course/provider-library scope; provider IDs and signed
webhooks never establish that ownership. Service operations recheck permission
and local revision/state after external work and use bounded durable leases to
reject concurrent/stale results. SQLite state binds its schema, provider mode,
library and capacity, refuses clock rollback and requires trusted local files,
backup policy and operator-owned keys. Multi-host replication is separate work.
Only confirmed-deleted local tombstones may be purged, in batches up to 100 and
after at least 24 hours; the host must retire purged creation IDs because their
idempotency memory ends at that point. Provider backups/cache erasure is separate.

Creation is journaled before remote dispatch. Bunny's documented creation API
does not supply an idempotency key: ambiguous creation must reconcile a persisted
random opaque creation marker, never blindly retry or claim exactly-once remote
creation. Updates/deletion are reconciled against authoritative reads. Webhook
v1 authenticates exact body bytes with the read-only library key but has no
signed timestamp; bounded durable duplicate suppression and serialized provider
refresh prevent replay/reordering from granting access or publishing assets.
Webhook status numbers and API video status numbers are distinct protocols.

Publication is an explicit application mutation after current provider readiness;
processing completion alone cannot publish. Playback requires current entitlement,
fresh ready state and configured provider protection; withdrawal/deletion stop
new grants but cannot immediately invalidate previously issued bearer tokens or
provider caches. Upload grants are scoped and expiring, not cryptographic proofs
of a one-shot upload, file type or byte limit. Library upload quotas and direct
file protection remain required configuration. Embed SHA-256, TUS SHA-256 and
CDN advanced HMAC-SHA256 directory tokens have independent documented formats.
Redirects/ambient proxies/arbitrary remote endpoints are disabled; request,
response, retry and total-operation budgets are finite. Secret-bearing errors,
credentials, grants and debug output must not leak keys or bearer URLs.

Empty or `mock_*` credentials select deterministic offline behavior; mixed modes
fail configuration. Production rejects mock or loopback-fixture capabilities.
Automated protocol/browser/disposable-state evidence is required. The owner's
no-live-account-testing instruction remains in force: actual Bunny account,
transcoding/CDN interoperability and paid DRM are not validated or implied.
See the [supported delivery target](managed-video-roadmap.md).

### v13 privacy and age-assurance boundary

Core and Security header layers share `apply_referrer_policy`: a response that
explicitly supplies a canonical `no-referrer` value retains that restriction
through composition, normalized to one header even if duplicate values exist.
Other values are replaced by the layer's configured policy; missing/invalid
optional Security configuration retains its existing behavior. This narrow rule
does not attempt to order every Referrer-Policy value or weaken other headers.

`rullst-privacy` is an opt-in, unpublished v13 release candidate. Its initial
`age-assurance` feature owns bounded risk policies, server-issued challenges,
minimal signed age attestations, explicit decisions and replay-store contracts.
Low-risk declarations, facial estimates and verified age attributes have
different assurance semantics; configured policy determines their eligibility.
No camera, image retention, external inference, database or Core dependency is
enabled implicitly. Production must reject offline mock evidence and
process-local replay protection. The host owns authentication, tenant/subject
binding, risk/legal assessment and durable shared state.

The v13 persistence contract uses static-dispatch asynchronous nonce claims.
Verification samples a trusted server clock before validation and after the
claim; expiry or clock rollback during storage cannot return permission.
The optional `sqlite` adapter is shared-local only: a private file-backed pool,
WAL with full synchronization, serialized quota/expiry/claim transactions and
persisted configuration/clock high-water state. It stores only domain-separated
nonce digests and expiry. No unexpired claim may be evicted. Cancellation or an
uncertain commit returns no assessment; the host must request fresh evidence
when consumption is uncertain. File custody, clock synchronization and storage
durability are deployment obligations. Restoring an older database requires
quiescing verification and invalidating all outstanding challenges through a
new policy/key epoch; SQLite cannot detect arbitrary backup rollback. Network
filesystems and multi-host replication are outside this adapter's boundary.

The v13 PostgreSQL adapter uses a separately enabled `postgres` feature and a
private bounded pool against one authoritative writable database. Explicit
deployment initialization creates the fixed `rullst_age_replay` schema; normal
connection never creates or repairs missing state. Transactional startup locking
and a metadata row lock serialize schema initialization and quota/expiry/nonce
claims across application hosts. Metadata persists the schema version, capacity
and accepted clock high-water mark. Tables must be permanent and WAL-logged;
`fsync`, full-page writes and synchronous commit are required. Claims store only
the same domain-separated nonce digest and expiry as SQLite. Remote connections
require certificate/hostname-verified TLS; loopback and local sockets support
disposable development databases. Errors must not expose connection credentials.
Bounded pool acquisition, statement/lock waits and idle transactions prevent
unbounded database waits; the caller still owns an overall request deadline.
All verifiers must use that same authoritative database and synchronized trusted
clocks. Replication configuration, failover fencing, durable hardware and backup
restore remain operator obligations; asynchronous replica promotion or database
rollback cannot be advertised as preserving consumed proofs automatically.
The adapter's focused real-database tests cover concurrent initialization and
claims, a restricted runtime role, quota, schema/durability drift, cancellation,
clock changes during a row-lock wait and server restart. Combined hosted
acceptance and deployment-specific failover/TLS evidence remain separate.

This first contract does not implement a facial model, vendor transport,
guardian verification or global privacy compliance. The
[privacy and age-assurance roadmap](privacy-age-assurance-roadmap.md) defines
the remaining consent, rights, retention, provider and jurisdiction work.
The owner-approved 13.0.0 delivery scope prioritizes the independent foundation
and an authenticated first-party declaration journey for policies that permit
that assurance. An external provider is optional and requires separate native
protocol and sandbox acceptance before any live claim. A local facial engine is
follow-up work. Neither an absent provider nor a failed check may downgrade the
required assurance or make a declaration authorize a stronger-policy action.
Publication of the privacy package still requires explicit acceptance of its
advertised API, durable state, consumer behavior and release configuration.

The packaging candidate adds this independent package to the seventeen-package
release inventory before the umbrella. The facade exposes only explicit
`privacy-*` features and `rullst::privacy`; default builds acquire no privacy,
age, consent or database dependency from this addition. CLI age/privacy consumers
select the matching registry version by default, with an explicit matching local
source override for development. Existing dependency sources, versions and
features must be checked before composing two consumers; no silent source switch
is permitted. Both commands refresh the bounded project context after scaffolding,
reporting a refresh failure without concealing the completed source edits.
Archive-only acceptance must install the packaged CLI and compile
generated SaaS/LMS consumers with both opt-ins and no workspace source paths.
This is packaging eligibility, not a published version or registration claim.
The stable publisher still refuses an unregistered crate; initial registration,
reviewed ownership and Trusted Publishing configuration are separate prerequisites.

The native `DeclarationGate` contract processes an authenticated first-party
`AgeDeclaration` without requiring an external issuer or a signing key for the
declaration itself. It accepts only a retained server-issued `SelfDeclaration`
challenge under the exact current policy and tenant/subject/session/action
binding. It shares the signed verifier's durable one-use claim and post-storage
expiry/clock checks. An affirmative answer can return only `Assurance::Declared`;
negative or declined answers deny the operation. An estimated/verified method
or stronger policy cannot be downgraded through this entry point. The host owns
authentication, CSRF protection, explicit user choice and trusted challenge
retention/transport. This contract is not a determination of the person's age.

The optional `challenge-tokens` transport uses a bounded, versioned,
HMAC-SHA256-authenticated server challenge with an explicit active key and at
most seven previous verification keys. It authenticates the version, key ID and
exact encoded payload before decoding challenge JSON. Opening revalidates the
current policy, authenticated binding, clock and exact configured lifetime.
There is no client-supplied algorithm or key discovery. Tokens contain only the
existing opaque challenge references and are authenticated, not encrypted;
applications must not put cookies, email addresses or document numbers into
those references. A token restores a server-issued challenge, never an age
decision or an authorization grant. Production still consumes its nonce in the
shared durable replay store. The host owns independent secret provisioning,
rotation/retirement, TLS, CSRF and endpoint limits; no external age provider is
needed for this transport.

The v13 `make:age-gate` consumer is an explicit opt-in for recognized SaaS and full
LMS starters. It protects the existing dashboard action with an authenticated GET
challenge and POST declaration; it never sets a reusable age-verified account
flag. Both routes retain the starter's authentication, CSRF, headers and Server
baseline. Policy version and threshold are chosen explicitly at generation time;
the SaaS profile also requires its fixed application tenant. Low-assurance
declarations are identified as such. Opaque
binding references are domain-separated keyed digests of server-resolved user,
tenant and authenticated session, never request-supplied identities. Application
keys and replay storage are mandatory; initialization, timeout, stale context or
unknown/negative answers cannot grant access. SQLite and PostgreSQL are explicit
profiles with their existing deployment obligations. Generation refuses unknown
or already-modified route shapes before writing. This command selects the matching
registry dependency unless the caller explicitly supplies a matching local source.
Before that version is published, development consumers must use the explicit
source override or a reviewed archive-only registry patch. Other blueprint adapters,
provider flows, persistent age permissions and deployed browser acceptance remain
separate work.

The extension of this same generator to the full LMS starter uses
the authenticated `TenantContext` produced by active school-membership resolution.
For the dashboard only, a bounded `school` query value may act as a selection
hint before the existing authentication middleware. It is never authorization:
unknown/conflicting selections, inactive membership and ambiguous selection fail
closed. The existing policy may select an explicitly stored default school.
The form action carries the already-resolved school selection, allowing
ordinary browser submission without a custom header. The challenge binds the
resolved school, user and session again on POST. The declaration does not update
the existing subject age band or guardian-consent records, and the age-state
middleware is not mounted around unrelated learning actions. The generated local
contract exercises cross-school/user denial, selector ambiguity/conflicts,
ordinary form submission, replay and membership revocation between issuance and
submission. Hosted and deployed-browser acceptance remain separate.

The v13 fuzz inventory adds two isolated privacy targets for authenticated
challenge transport and signed attestations. Deterministic fuzz-only keys permit
mutation beyond signature validation; assertions bind accepted results to the
current context, assurance and one-use replay contract. These targets enable no
database or network integration. They expand v13's required inventory to 42;
the immutable v12 release line retains 40. Short diagnostics are not full release
evidence, and the durable database lifecycle keeps its separate acceptance gates.

The unpublished `consent` contract is independent of age assurance and concerns only
optional processing that the operator has assigned to an explicit purpose and
notice version. Absence, refusal, withdrawal, version mismatch and expiry deny
processing. An affirmative choice uses the exact observed revision; withdrawal
atomically advances that revision even when its form is stale, preventing an
earlier affirmative form from undoing it. The authenticated subject and tenant
are server-owned inputs, never browser identity fields. The explicit submission
also binds the displayed purpose/notice version to current server configuration,
so a policy change cannot borrow an old form's unchanged revision. A bounded store must
serialize reads/updates against a persisted clock high-water mark and preserve
withdrawal tombstones; production rejects process-local state. Permission must
be checked immediately before each processing action, including deferred jobs.
Already-started external effects are not cancelled retroactively by a later
withdrawal, and consent does not authorize essential processing or establish
age/guardian authority. The optional `consent-sqlite` adapter initializes a new
local file explicitly; ordinary opening never creates or repairs missing state.
It serializes reads/updates with `BEGIN IMMEDIATE`, WAL/full synchronization,
a bounded private pool, immutable quota and persisted clock metadata. It retains
scope digests, latest choices/versions/revisions and timestamps, without raw
subject IDs. Those digests are pseudonymous data. Expired records and withdrawal
tombstones are never evicted to create capacity. All processes must use that same
trusted local file; multi-host replication is unsupported. Restoring stale state
requires quiesced processing, reconciled withdrawals and fresh purpose versions.
Deployment retention/restore review and hosted acceptance remain required
before release admission.

The owner-selected PostgreSQL consent increment is under implementation; it is
not yet source-admitted. Its separate `consent-postgres` feature exposes
`PostgresConsentStore::initialize` for deployment bootstrap and `connect` for
ordinary runtime access. Both accept a connection string and immutable capacity.
One authoritative writable database owns the fixed `rullst_consent` schema.
Explicit initialization serializes concurrent bootstrap without resetting
existing state. A private bounded pool enforces verified TLS for remote TCP,
fixed name resolution, synchronous commits and bounded acquisition/SQL waits.
Every read/update validates permanent WAL-logged tables and locks the metadata
row, serializing clock observations, quota and revision changes across hosts.
Withdrawals remain durable tombstones; no expiry/capacity cleanup removes them.
Stored subjects remain domain-separated digests. Replication fencing, clock
synchronization, hardware durability and stale-backup reconciliation remain
deployment obligations. The facade feature `privacy-consent-postgres` must not
enable age assurance or add a database dependency to default builds. The
existing CLI privacy consumer remains explicitly SQLite until separately
extended; an ordinary typed application can use the PostgreSQL store directly.
Disposable-database concurrency, restricted-role, cancellation, clock, restart,
consumer and hosted acceptance are required before completion is claimed.

The opt-in `make:privacy` consumer mounts authenticated preferences,
an explicitly optional personalized greeting and an own-account JSON export
inside the starter's existing CSRF/header/security boundary. It composes
with the age consumer while preserving both consumers' dependency features.
Choice forms bind the displayed notice/version and current revision, with a
bounded expiring HMAC proof tied to the authenticated account, tenant and session;
an old tab cannot apply a choice after switching accounts or sessions. Identity
comes only from the authenticated user and, for LMS, active school membership.
The export projects only account ID, name and email with explicit output bounds;
it exposes no credential fields, produces no public artifact and does not mark
broader queued rights requests as fulfilled. Its direct private response needs no
retained export file and remains independent of optional-consent storage
availability. Refusal/withdrawal changes the next greeting to generic content.
Local materialized SaaS/full LMS fixtures cover both age/privacy installation
orders, real authenticated profile queries, explicit choices, withdrawal/stale
forms, session/account/school changes, CSRF, expiry, bounded input and missing or
failed state. A normal SaaS using PostgreSQL as its primary database compiles;
this is not live PostgreSQL consumer or browser/deployment acceptance. Bootstrap
explicitly initializes a new private consent file; ordinary opening never
recreates it. The generated consent profile remains shared-local SQLite only.

The separate [SaaS triage](saas-v12-1-v13-triage.md) assigns the examples' reported
defects to compatible v12.1 maintenance and v13 contracts; it is not fix evidence.

### v13 source-train and adoption preparation

The development train uses `13.0.0-alpha.1` consistently for the existing
publishable packages and their internal requirements. This prepares the CLI's
actual major-version behavior; it is not a publication or stable-release claim.
The privacy package joins the candidate inventory with explicit optional facade
features; registry publication remains subject to package and ownership admission.

The `rullst-upgrade-rules-v2` migration catalog recognizes source major 13 as
well as 5, 6, 11 and 12, keeps exact target-major CLI selection and rejects
downgrades. Preparations from the previous catalog require fresh preparation
and verification. The [12.1-to-13 source inventory](migration-v13.md) adds opt-in privacy APIs,
consumer generators and security header composition, without a known required
replacement of existing application APIs. Automatic preparation is therefore
limited to supported dependency manifests and the existing compiler/check/test
workflow; it must not invent code rewrites, enable age or consent policies,
bootstrap privacy state, replace authentication or deploy an application.
Migration evidence must distinguish small offline updater protocol fixtures
from generated SaaS/LMS consumers compiled against actual framework source.
Release-time inventory and package checks must revisit this boundary as more
v13 changes land.

The current candidate intentionally tightens the Android release command's
configuration contract: the expected public certificate and trusted verifier
paths are now required in addition to the existing signing inputs. An unchanged
Rust helper signature does not make that behavior minor-compatible. Retain this
break in the migration inventory. The major-aware SemVer job is not evidence
that the candidate could instead be released as 12.2; that question requires a
minor-level comparison and review of the full documented compatibility surface.

### v13 formal-verification pilot boundary

The [Verus pilot](verus-roadmap.md) has a locally verified candidate for production
age-policy method decisions, with later evaluation of Auth authorization predicates
and Capital integer money calculations. Specifications must remain linked to
the executable implementation, with explicit trusted assumptions and external
contracts. No dedicated public crate is proposed. A pinned, isolated verifier
and manual workflow precede any required v13 check; compatibility, reproducible
proofs, negative controls and measured CI cost are promotion criteria. This
plan adds no v12.1 release gate or framework-wide correctness claim.

The first implementation candidate verifies the existing `AgePolicy::permits`
body with its actual enum variants and policy fields, extracted from Rust syntax.
No public API or production dependency changes are required. Extraction must
check the reviewed signatures/types and original structural equality derives,
copy the executable body unchanged and record source/type/body fingerprints.
Only verifier annotations, structural-equality support, a specification accessor
and an erased reveal step may be added to the projection. The proof covers all
nine risk/method combinations without narrowing valid runtime inputs; it does
not prove age evidence, policy validation, timestamps, storage or authorization.
The isolated verifier must reject weakened restricted/elevated policies and a
denied low-risk policy as negative controls. This is a bounded pilot candidate,
not a promotion into required release checks.

### v13 shared passkey ceremony increment

The next Auth increment follows the [shared ceremony contract](shared-passkey-ceremonies.md).
Keep the existing synchronous process-local API compatible. Add a separate generic
shared manager in `rullst-auth`, backed first by opt-in authoritative PostgreSQL,
with tenant/account/session/RP binding, bounded expiry/quotas, atomic single use
and fail-closed outage/cancellation. Reuse private cryptographic validation; do
not expose a stateless acceptance bypass. Application credential ownership,
revocation and signature-counter CAS remain mandatory before session issuance.
Real independent-manager/database/process and HTTP/browser acceptance must precede
an implemented claim. No new crate or broad blueprint expansion is required.

### v13 deployment acceptance increment

Before expanding deployment generators, exercise two independent application
processes behind one digest-pinned existing reverse proxy. Keep readiness,
unhealthy-node exclusion, bounded shutdown, forwarded identity and WebSocket
behavior explicit. Ordinary HTTP admission must remain counted until the response
body completes, errors or is dropped, including streaming data and trailers;
returning response headers alone is not completion. This counts application body
lifetime, not client receipt or TCP acknowledgement. Upgraded connections and
background tasks require separate application-owned termination. Prove the body
boundary locally before using it in deployment acceptance. No new gateway crate,
automatic host administration or general availability guarantee is introduced.

### v13 local deployment diagnostic

Add an offline, read-only `deploy:doctor` executable command in the existing CLI.
Inspect the selected bounded `Rullst.toml` snapshot (or documented defaults) and
optionally one explicit literal environment file or the three allowlisted process
variables `RULLST_ENV`, `APP_ENV`, `APP_KEY`. Never silently combine environment
sources or search parent dotenv files. Reuse Core environment resolution and
security validation; report obvious application-key mistakes separately from
unobserved key generation/custody. Config validity cannot prove mounted middleware,
webhook verification, proxy trust, shared state, TLS, backups or deployed behavior.

Emit deterministic text/JSON with fixed check codes, source categories, actionable
messages and explicit uninspected controls. Do not print values, input paths,
parser details or secret-derived hashes. Reject malformed, duplicate, oversized,
linked/special-file and unsupported interpolated inputs without executing them.
No subprocess, network access, configuration/global-environment mutation or
automatic repair belongs in this command. Test the installed command with real
generated configuration and negative inputs; retain full hosted acceptance.

### v13 supervision observation extension

The owner explicitly requested the reusable proctoring base on September 20.
Extend `rullst-supervision`, not a second proctoring crate. Keep legacy visibility
APIs working, but new sessions must persist an exact selected collection set and
require matching acknowledgement at start/resume. Default selection remains
page visibility. New browser event kinds are focus, copy/cut/paste occurrence
and fullscreen transitions, never clipboard content or other-window inventories.
Optional camera/microphone/screen capture-status reports are distinct from
optional camera-presence/audio-activity adapter observations; preserve provenance
and inconclusive/unavailable outcomes without a misconduct score.

Add bounded, typed, statically dispatched analysis interfaces. Permission and
active session/tenant/subject/revision must be checked before analysis and again
before storing its result. Keep borrowed media samples bounded and out of durable
event state/logs. No camera/audio model, identity inference, hidden recording,
grading or automated accusation is introduced. Selected capabilities do not
establish browser permission or prove media belongs to the learner; the host
must bind capture to the authenticated session and implement adapter cancellation.

The unpublished SQLite candidate may move to schema v2 with explicit rejection
of old schemas. Never silently rewrite a v1 database: preserve it for review and
require a fresh independently named store/epoch and authority provisioning for
this prepublication transition. Session pause/end/expiry and policy revisions
must reject pending results. Retain all existing capacity, sequence, clock,
reviewer authorization and retention invariants.

Expose new browser collection only by explicit CLI selection and a matching
visible notice. The collector must bound its queue and stop on session control,
expiry or failure; it must not prevent copying, request fullscreen/capture or
start media analysis automatically. Extend the focused LMS/browser fixture and
test local adapter faults, revocation races, disallowed capabilities, persistence,
source attribution and legacy visibility compatibility before hosted admission.

### Conditional v13 supervision crate

The owner requested transparent learner/exam supervision and parental controls
as the first additional priority after the required v13 deliveries have been
implemented and validated. `rullst-supervision` is now an unpublished implementation
candidate with bounded domain contracts and shared-local SQLite state. Its
local generated-consumer and Chromium journeys pass. The baseline also passed
hosted checks and archive rehearsal in PR #222; the observation extension passed
PR #226 and its retrospective archive check. The combined source passed PR #239.
The package now enters the v13 release inventory with no default facade dependency.
Its changed publication configuration still requires hosted package validation,
reviewed initial registration and final release acceptance.

Keep supervision policy/session/event and reviewer-access contracts separate
from `rullst-privacy` age and consent primitives. Exam supervision and parental
controls need distinct modules and authorization policies. The initial candidate
must deliver one real generated LMS journey, visible session status, explicit
permissions, revocation, bounded collection/retention and actual application-side
enforcement, with cross-school/subject and unauthorized-reviewer negatives.
Parental time/content restrictions initially concern this application only.
The host must establish guardian/reviewer authority independently; age results,
account ownership, a checkbox or a claimed family relationship do not prove it.

Browser observations are untrusted client reports, never proof of misconduct,
identity or an automatic reason to change grades or impose a penalty. Collect
only the documented minimal events with visible active/paused/ended state;
do not add covert camera/microphone/location capture or unrelated browsing data.
Native device-wide controls, camera inference and managed operating-system agents
need separate platform integration and acceptance. Global legal compliance is
not inferred from these controls. The exact API/storage boundary and its threat
model must be specified before scaffolding the crate. Empty contracts or a mock
alone do not meet its admission criteria, and it must not delay required release
gates; retain it for a subsequent v13 release if capacity is insufficient.

The initial implementation contract is now the
[bounded supervision design](supervision.md). Use a separate unpublished package
with optional `exam`, `parental`, `sqlite` and `analysis` features, no Core/default dependency,
and a concrete SQLite adapter generic over a trusted clock. Tests use real local
SQLite and an injected deterministic clock; no memory mock is needed for this
initial backend. One private initialized database owns scoped authority,
sessions, parental enrollment/policy and allowlisted events. Serialize operations
with `BEGIN IMMEDIATE`, persist configuration, a clock high-water mark and a
global revision counter, and enforce bounded quotas and retention. An opener
must supply the independently retained deployment epoch. This detects epoch
mismatch, not restoration of an old database with the same epoch.

`make:supervision` remains an explicit full-SQLite-LMS opt-in. Stable CLI releases
pin the supervision dependency to exactly their own version and disable default
features. `--supervision-source` optionally selects a local crate with matching
name/version and the SQLite module. A prerelease CLI requires that source path
and fails before applying edits when it is absent; it must never resolve an
unpublished registry candidate or silently substitute the stable v12 package.

Only trusted operator provisioning can establish expiring `ExamReview` or
`ParentalManage` authority after independent relationship verification. The host
binds current authenticated identity, school membership and resource access;
the store rechecks its own scoped grants atomically on each operation. Those
two authorization layers are not one transaction unless the host composes them
that way. In-flight results cannot be recalled after external membership changes.
Session start/resume requires explicit current policy/notice acknowledgement;
pause/end and policy changes use exact revisions. Events are only visible/hidden
page reports with an exact next client sequence and server receipt time. Their
absence or contents never influence grading. Parental enrollment is operator
managed: an enrolled subject with absent, expired or denied policy is denied;
an unenrolled subject retains existing application authorization. Revoking a
manager does not remove the subject's restrictions. The generated opt-in must
enforce them on original lesson/progress routes before package admission.

### Versioned release-branch boundary

Release policy schema 4 binds major 13 to development `main` and major 12 to
maintenance `v12`. Schemas 2 and 3 retain their historical bindings; no existing
tag or receipt is reinterpreted. `v13` remains a transitional integration branch
until its admitted source is promoted through a protected PR to `main`.
The source-admission gate requires a canonical matching tag, the exact
checked-out/tagged commit at the current protected branch head, and every
publishable inventory package at the tagged version before artifact builds.
All declared automatic release workflows accept both maintained source lines
and the transitional `v13` branch.
Required manual/native/security evidence and the protected crates.io approval
remain separate mandatory gates. Archive evidence requires the named archive
job to succeed on the exact candidate; a green workflow with that job skipped
is insufficient. Incompatible manual package selectors must fail explicitly.
Observational CI reports may describe failed checks, but must stop on cancellation
so an obsolete run cannot retain the concurrency slot needed by its replacement.
Fuzz evidence must come from the candidate's
release line and a source carrying that same policy; v12 results cannot be
credited to v13 merely because they are recent. The existing immutable v12
tags retain their original workflow and policy.

### v13 active-session management (source admitted, unpublished)

Extend the existing opt-in `SqlRecoveryStore` rather than introducing another
authentication registry or crate. A currently valid opaque session may list
only its account's at-most-20 active sessions, revoke a selected sibling or
revoke all siblings. Management identifiers are purpose-separated keyed
digests, never bearer credentials. Current-session logout keeps the existing
revocation API. Every protected request still verifies authoritative SQL state;
encrypted-only cookies and independent JWTs do not acquire revocation implicitly.

Inventory may expose creation/expiry times, a current-session marker and an
explicit bounded display label. Do not collect IP addresses, raw user agents,
device fingerprints or activity history. Old sessions without the additive
metadata keep working and report unknown creation/label values. Metadata follows
expired-session retention, logout and password-reset deletion in the same transactions.
An operator-owned task purges at most 100 expired sessions per transaction;
time passing alone does not physically delete rows. Session operations have a
ten-second database deadline; an error does not prove an uncertain write rolled
back, and applications reconcile authoritative state before reporting success.
Management requires current authentication inside its SQL transaction; selective
revocation binds the target to that account and rejects the current session.
Logout of siblings also advances the account session version while preserving
the current session, fencing previously issued password-authentication proofs.

The application supplies trusted server time and mounts management behind its
authenticated tenant selection, CSRF, security headers, WAF and ingress limits.
Account stores remain explicitly separated by application/tenant database and
keys as documented in the recovery contract; request data cannot choose either.
No device label proves device identity. Already-authorized in-flight operations,
WebSockets and sessions managed by another identity system need separate policy.
Required acceptance includes SQLite and PostgreSQL transactions, cross-account
and cross-store denial, independent-process request verification, restart,
expiry, password recovery, concurrent revocation and fail-closed storage errors.
Source/package admission passed in PR #236; final release admission remains separate.

### v13 email-login implementation contract

The owner-selected email-login increment is a local candidate pending hosted
source/package admission; see [the operational contract](email-login.md). It must stay
explicitly optional in Auth, with SQLite shared-local and PostgreSQL shared
state profiles composed with the existing authoritative recovery account/session
registry. It must not turn a password-reset token into a login credential or
silently enable email login for every existing account.

The selected journey requires an explicit server-configured application/tenant
namespace and account opt-in established through existing authenticated account
proof plus the host's tenant/MFA policy. The application resolves membership and
ordinary authorization; namespace strings from URLs are never authority. Email
access alone does not satisfy a stronger authentication requirement.

Issuance generates separate random 256-bit emailed and browser-held secrets,
binds their digests to the account, namespace and current authentication epoch,
and queues a minimized encrypted notice atomically. Known, unknown, disabled and
throttled accounts receive the same public acknowledgement. Storage, account and
request limits must remain bounded. Tokens expire after a fixed 15-minute
lifetime and are invalidated by replacement, account-policy changes or a changed
authentication epoch. Database time observations and post-storage expiry checks
must prevent rollback or long lock waits from authenticating stale evidence.

GET/HEAD requests may render the landing page but never consume a token or mint
a session. Redemption requires deliberate CSRF-protected POST, the matching
browser-held secret and the current configured context. Token consumption and
creation of the existing revocable opaque session must commit in one database
transaction. Any uncertain outcome returns no authentication credential.
The session retains existing limits, inventory, logout and epoch revocation.

Landing and destination URLs are explicit server configuration, with HTTPS in
production, no request-derived host/redirect and no third-party page content.
Responses require no-store and no-referrer protections; secret-bearing URLs,
cookies and messages must not enter logs. A fenced encrypted delivery outbox
supports bounded retries and stable delivery identifiers, with a dedicated Mail
security template and deterministic delivery fixtures. An email scanner's GET
must not invalidate the user's login. These token and prefetch choices draw on
[OWASP's URL-token guidance](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html#url-tokens)
and [Supabase's documented prefetch behavior](https://supabase.com/docs/guides/auth/auth-email-templates#email-prefetching).

Required acceptance includes both relational backends, concurrent redemption,
account/namespace/browser mismatch, account-policy and password-reset races,
outbox failure/retry, session limits, server restart, a real HTTP/browser journey
and extracted-package consumers. Production credentials and live inbox delivery
remain outside the owner's authorized testing scope. The existing reset journey
keeps its separate behavior and must retain its regression evidence.

### v13 application API-token implementation contract

The third owner-selected increment is a separate optional Auth lifecycle for
opaque application API tokens, with SQLite shared-local and PostgreSQL shared
state. It must compose the authoritative recovery account epoch and private
database connection safeguards without enabling email-login, JWT or OAuth by
default. Provider credentials and browser-session tokens are separate purposes.

Issuance requires a current password-authentication proof from the same account
registry plus the host's tenant/MFA/permission approval. An immutable server
namespace and literal scope allowlist bound every token. Requested scopes are
nonempty, bounded and exact (no wildcards); the application intersects its
current account permissions before issuance and checks current tenant/resource
authorization on every domain request. A client-supplied namespace or subject
never establishes authority. API tokens cannot mint other tokens through this
management API.

Return a random 256-bit secret once, with a separate opaque management ID and
an unmistakable versioned prefix; persist only a purpose/namespace-bound HMAC.
Store bounded labels, literal scopes, expiry and revision, without IP, user-agent
or usage history. Enforce a per-namespace row quota, at most 20 active tokens per
account, and a configured maximum lifetime no greater than 30 days. Verification
must read authoritative state, validate the account epoch, exact required scope
subset and trusted time, and fail on missing/unavailable/revoked state. Never
cache a positive verification as a replacement for these checks.

Owner-only inventory, idempotent revocation and atomic compare-and-swap rotation
are required. Rotation keeps the management ID and scopes, advances the revision,
replaces the secret and may renew only within the configured lifetime. A stale
rotation cannot overwrite a newer credential or revive a revoked token. Deleting
a token is authoritative revocation: caller-selected IDs cannot recreate it.
Account epoch changes invalidate all older tokens. Concurrent checks have a
documented database linearization point; already authorized external work cannot
be recalled or made atomic with a later revocation.

Reuse verified remote TLS, bounded private pools/SQL deadlines, durable storage
checks, namespace configuration binding and persisted clock observations. An
uncertain issuance/rotation returns no credential, and restart, expiry during
lock waits, rollback, foreign-account management, scope escalation, token-purpose
confusion and concurrent rotation/revocation need executable evidence. Document
backup anti-rollback and host-owned authorization boundaries. Full workspace,
native protocol/process, authenticated HTTP and extracted-package admission are
required before a supported feature claim; no external provider accounts are
needed or authorized.

HTTP composition adds an explicit Core `MachineEndpoint::verified_bearer`
constructor using the existing exact-route `MachineRequestVerifier` contract.
Auth supplies a verifier that requires one Authorization bearer header, reads
current token state and inserts a redacted principal. It never accepts cookies,
origin-bearing browser requests or URL tokens as machine authentication. The
existing static bearer constructor retains its behavior; only an authenticated
exact method/path receives the machine CSRF exception, with WAF/headers retained.


### v13 shared PostgreSQL mail-suppression implementation contract

The fourth owner-selected increment extends Mail's existing suppression traits
with an optional PostgreSQL store and facade feature. It must preserve verified
event replay binding, conflict rejection, monotonic reason precedence, immutable
recipient/event quotas and final pre-dispatch checks through `SuppressionGuard`.
It is not a new transport and must not change default Mail/SQLite features.

Each instance selects an immutable server-owned namespace, quotas and a strong
secret HMAC key. Persist purpose/namespace-bound keyed recipient/event identifiers
and event fingerprints, not raw recipient addresses, provider event IDs, message
bodies or delivery history. Keep only the authoritative reason/provider and
first/last observation times needed by the existing record contract. Configuration
and key drift fail closed. A namespace chosen in a request is never authority;
the host authenticates tenant membership and verified provider events before
selecting the corresponding store/driver. Memory remains the explicit offline
test implementation; production database failures cannot silently use it.

Provide explicit deployment initialization and ordinary startup without DDL or
missing-state repair. Runtime operates without schema mutation privileges, on
one authoritative writable PostgreSQL database with verified remote TLS,
permanent tables, durable commit settings, bounded pools/waits and whole-operation
deadlines. Serialize event application, lookup and retention through the
namespace control row. Atomically record replay evidence and recipient state;
cancellation, conflicting events or quota failures must not leave partial state.
Recheck persisted server time and storage durability before reporting outcomes.

Retention may prune replay identifiers after the host-selected provider replay
window, but never remove recipient suppression as a side effect. Lookup/storage
faults block delivery. Check immediately before dispatch, while documenting
that a suppression committed after that check cannot recall in-flight external
mail. This does not promise exactly-once provider delivery, inbox acceptance,
backup anti-rollback, automatic failover, legal compliance or automatic opt-in
after a complaint. Required evidence includes independent pools/processes,
concurrent replay/conflicts/quotas, rollback/cancellation, restricted roles,
service restart, real guard/worker composition and extracted-package consumers.

### v13 durable recurring-publication implementation contract

The fifth owner-selected increment belongs to the existing Messaging crate as
optional `schedules-postgres`, with an explicit facade feature. It coordinates
bounded UTC cron occurrences across instances through one authoritative PostgreSQL
database and relays them to the existing static `MessageBroker` contract. Core's
process-local `Scheduler` retains its API and behavior. A broker publication is
not proof of handler completion or an exactly-once external effect.

Require a server-owned namespace, immutable limits and explicit encrypted storage
keys. Schedule names are bounded application configuration, not tenant authority.
The host authenticates authoring/inspection/cancellation and selects the correct
namespace and broker. Creation is idempotent for the same immutable definition;
conflicting reuse fails. An edit requires cancellation and a new schedule name,
with immutable generation identities protecting retained occurrence keys.

Use the cron crate's five-field projection in UTC (weekday 1=Sun through 7=Sat,
calendar-day and weekday restrictions intersect; names are accepted), with a
one-minute minimum resolution, bounded
definitions/payloads, explicit first-after time and explicit missed-run policy.
Bounded catch-up emits the oldest due occurrences within the caller's tick budget;
coalescing emits one oldest outstanding occurrence and advances after the observed
current time. Advancing the schedule and persisting frozen occurrence content
must be atomic. Storage quota exhaustion cannot silently discard due occurrences.

Persist definitions and occurrence content using authenticated encryption bound
to namespace, schedule generation and occurrence identity. Do not silently adopt
plaintext storage. Lease ownership, attempts, retry deadlines and terminal state
are authoritative SQL data. Leases are unpredictable and exactly fenced; stale
workers cannot acknowledge/retry/revive a cancelled or superseded occurrence.
Broker requests use deterministic purpose-separated occurrence idempotency keys,
and retries preserve exact content. Recheck a live lease immediately before
publication and acknowledge only after successful broker acceptance. If acceptance
precedes an uncertain ACK, retain evidence of acceptance and reconcile/retry the
same key. A cancellation after the final check cannot recall an in-flight broker
request; consumers must deduplicate/authorize at their side-effect boundary.

Expose bounded inventory/terminal inspection, explicit failed-occurrence retry,
permanent cancellation and retention of terminal rows only. Automatic attempts
and the delivery window are bounded. Deadlines, clock rollback, namespace/key
drift and non-durable/unavailable state fail closed. Reuse verified TLS, bounded
pools/SQL deadlines, explicit initialization and runtime without DDL privileges.
Document UTC-only behavior, unchanged per-process cron, broker deduplication
retention, backup/failover obligations and host-owned authorization. Required
acceptance includes independent instances/processes, real PostgreSQL rollback,
restart and restricted roles, cancellation/lease races, durable broker consumption,
publication-before-ACK replay and extracted-package/facade consumers.

### v13 durable outgoing-webhook implementation contract

The sixth owner-selected increment extends Messaging behind opt-in `webhooks`
and facade `messaging-webhooks`. Compose the existing encrypted SQLite broker
as a private durable outbox rather than adding a second queue. The first profile
supports independent processes sharing one local database file; remote/multi-host
webhook state remains separate. Do not change default dependencies. App-domain
SQL transactions and this outbox are distinct; applications needing atomic
domain publication must bridge from their existing transactional outbox.

One server-approved immutable destination, signing identity, delivery window
and bounded policy belong to each server-owned namespace. Persist an encrypted
control publication with a fixed idempotency key: configuration/secret drift
must conflict, not redirect queued data. Never expose this private broker or
purge/subscribe to its control topic. Freeze bounded JSON bytes and event kind;
retries keep the stable message ID and exact body. The host authorizes publishing,
inspection, cancellation and manual retry before selecting the namespace.

Production accepts only explicitly configured HTTPS destinations, with verified
TLS, no proxies/redirects, bounded timeouts and fresh public-address resolution
pinned to the actual connection. Deny private, loopback, link-local, multicast,
reserved, documentation and transition address ranges, including mixed DNS
answers. An explicit literal-loopback development mode and deterministic offline
mode may support owned fixtures; production configuration rejects both. Empty or
mock signing credentials select offline delivery, never a network fallback.

Sign exact body bytes, stable ID, kind, key ID and attempt timestamp with a
purpose-separated HMAC-SHA256 contract. Provide strict constant-time receiver
verification with bounded time skew; receivers must also store processed IDs
and authorize the event. Do not claim signatures prevent replay by themselves.
Keep signing/storage keys separate and redact credentials, URL, payload and
response bodies from Debug/errors/terminal inspection.

Use bounded attempts/backoff and an immutable delivery window. Revalidate the
actual SQL lease immediately before external dispatch, after DNS work. A late
worker cannot acknowledge/retry/revive another worker's claim or cancelled work.
Acceptance before an uncertain ACK remains an at-least-once condition; preserve
the accepted HTTP status/ID and retry the same identity. Retry transient failures,
terminally reject redirects and permanent failures, and expose bounded minimized
terminal inspection, explicit retry within the original window, cancellation
and terminal retention. Retention must not delete active work or control state.
The host owns disciplined trusted time, persistent disk/backup policy, receiver
deduplication, endpoint approval and worker supervision. Native acceptance must
include actual HTTP/TLS protocol fixtures, SSRF/redirect/signature/replay negatives,
independent instances/process restart, SQL cancellation/outage/lease contention,
retry/dead-letter/retention and extracted-package/facade consumers. Real provider
accounts are not required or implied.

### v13 SQLite messaging lock-time correction

Before composing outgoing webhooks, real lock-contention tests demonstrated
that SQLite broker operations sampled time before `BEGIN IMMEDIATE`: an ACK
could be accepted after its lease expired while waiting, and a new claim could
already be expired on acquisition. Publication, claim, ACK, retry and dead-letter
operations must sample trusted time after acquiring their write transaction.
Retry availability and new lease duration start from that admitted instant.
Expired workers fail with `LeaseExpired`; their work remains recoverable. This
correction preserves public API/schema and does not add persistent clock
anti-rollback or change at-least-once delivery semantics.

### v12 audit correction invariants

**12.1 account mail:** `rullst-mail::ActionLink` validates an exact
application-configured origin (HTTPS, or explicit loopback HTTP); action URLs
and `Message` contents are redacted from Debug. The compatible `Message`
pipeline preserves only bounded opaque `token` query values inside safe body
URLs, including HTML-escaped query separators. This context-aware exception
does not apply to subjects, arbitrary non-URL text, URL credentials, other
secret parameters, fragments, provider errors or telemetry. Applications own
the destination authorization; a generic body URL is not an identity proof.
`AccountMail` builds versioned deterministic English, Portuguese and Spanish
account notices. Reset issuance expires after 20 minutes; delivery uses an
absolute UTC expiry so a stable idempotency key retains identical content.
Templates generate neither action tokens nor marketing consent and add no
tracking. Provider tracking configuration remains a separate obligation.
`LogDriver` records only delivery metadata.

Opt-in Auth `recovery-postgres`/`recovery-sqlite` owns an authoritative account
registry, purpose-separated keyed token digests, revocable opaque sessions and
an AES-GCM transactional outbox. Account creation enqueues welcome; password
replacement, reset consumption, sibling invalidation, session revocation and a
password-change notice commit atomically. The facade `account-mail-*` features
compose this store with Mail without another publishable crate. Existing
password tables and encrypted-only cookies require explicit migration; no
implicit application rewrite or cross-database transaction is promised.
Bounded claims, retry, idempotency propagation, Resend/Svix feedback verification
and the ACS Managed Identity transport are additive foundations. Redis recovery,
other identity-action transactions, marketing consent and audited administrative
resend are not implemented by this contract. See [account mail](account-mail-v12-1.md)
for exact limits, deployment obligations and external acceptance boundaries.

**12.1 additional Mail transports:** native SendPulse static Bearer authentication,
Mailjet v3.1 Basic authentication and Mailtrap Sending/Sandbox APIs preserve the
mandatory pipeline and deterministic offline credentials. Real sends require an
explicit verified sender, due message, bounded JSON and validated success body.
Mailjet disables open/click tracking; SendPulse/Mailtrap require account-level
tracking configuration. Mailjet remote sandbox and Mailtrap hosted capture are
explicitly separate from production and offline mocks. SendPulse rejects inline
CID/unsubscribe-header messages outside its reviewed REST contract. The local
`MailTrap` harness is not the hosted Mailtrap service. Provider account acceptance
and durable operation reconciliation remain external; no exactly-once promise
is inferred from a delivery ID. HTTP failures omit provider bodies while retaining
status and bounded retry metadata.

**12.1 one-time Stripe foundation:** a separate fixed-price `mode=payment`
contract checks active provider catalog amount/currency before creation and
binds session, owner, customer, attempt, price and mode. Receipt reads validate
the expanded PaymentIntent/charge, with partial refunds and disputes preventing
`Paid`. Signed event hints must be reconciled through application-owned durable
inbox/entitlement transactions. Existing generated billing remains recurring;
new sandbox evidence and merchant refund/dispute operations are still required.

**12.1 authenticated machine endpoints:** an immutable `MachineEndpointPolicy`
on `Server` exempts only exact write-method/path pairs from browser CSRF after
proving a strong bearer secret or an explicit host-supplied signed-webhook/mTLS
verifier. Cookie/Origin/Sec-Fetch-Site inputs are rejected on these routes; body
size is bounded and WAF/secure headers remain composed. Wildcards and weak
secrets fail at construction. Applications own ingress limits, replay storage,
certificate trust and authorization; an unverified proxy header is not mTLS.

The current [release audit](v12-release-audit.md) reopens earlier readiness
claims. A historical score, checked roadmap item or green mainline run is not
evidence that the current revision satisfies these contracts.

- Core's production COEP remains `require-corp` by default. Application
  configuration accepts only `require-corp`, `credentialless`, or
  `unsafe-none`; choosing a less isolated policy and the matching CSP/media
  allowlist is an explicit application threat-model decision. The `html!`
  parser accepts and strips source-only `<!-- ... -->` comments; comments do
  not create a raw-HTML trust boundary or client-visible output.
- Generated safe ORM projections validate column identifiers. Empty membership
  predicates match nothing. Mandatory tenant, global and soft-delete scopes
  remain grouped outside application `OR` expressions; nested queries preserve
  validation errors. Bulk mutation must not bypass model authorization.
- Queries inside a managed transaction use its executor. Callbacks that already
  borrow a mutation executor must fail explicitly on unsupported reentrant ORM
  access rather than deadlock or silently use an unrelated connection.
- Local durable stores use SQLx-owned transactions so dropping a cancelled
  operation schedules rollback. A cancellation racing an already dispatched
  commit can still have an uncertain outcome and requires reconciliation.
- Authentication validates expiry independently of clock-skew allowances;
  provider-specific identity claims, nonce and refresh semantics cannot be
  replaced by successful offline fixtures.
- Payment adapters must not fabricate live checkout prices, portal URLs or
  mutation success. Unsupported provider operations fail explicitly. Signed
  webhook bytes still require provider-specific schema and lifecycle validation.
- Development restart owns builds, application snapshots and migrations. It is
  not a stable Rust DLL ABI or production rolling-deployment mechanism. Windows
  descendant cleanup remains best effort without a Job Object implementation.

These describe required behavior, not a declaration that every final release
gate has passed. The audit records the current evidence and remaining work.

### Application-key maintenance invariant (v12.1.1 candidate and v13)

Application-key validation rejects public scaffold/documentation placeholders
after trimming and case normalization. Legacy `Rullst.toml` discovery accepts
only exact `app_key` or `key` field names, not fields sharing a prefix. It keeps
the historical value before the next `=` so valid existing sessions remain
readable; it does not rewrite configuration or silently rotate secrets.
Prefer `APP_KEY` for new values containing `=`. Moving an existing TOML value
must preserve its effective bytes or explicitly rotate the secret and invalidate
sessions. Session operations and optional JWT signing-key construction reject
the documented placeholders before accepting session/token processing.
The maintenance correction is carried from v12 PR #259; release admission is
separate from its presence in development source.

The corresponding facade fuzz harnesses exercise real configuration parsing,
authenticated session decryption/tamper rejection, tenant membership resolution
and tenant-scoped realtime publication, with deterministic positive/negative
controls. The realtime target does not exercise TCP/WebSocket frame decoding.
Historical runs of the replaced no-op/weak-fixture harnesses are not evidence
for these contracts. v13 campaign admission remains outstanding; the port does
not import the v12-specific source-equivalence policy.

### Studio browser composition invariant (12.1.0)

Studio browser composition in the published 12.1.0 maintenance release preserves
both root and `/studio`-nested same-origin asset routes. A raw browser without
a supplied cache renders an explicit unavailable state and exposes no cache
mutation endpoints. The full local builder installs the configured cache once
and retains its verified-loopback/same-origin protection. Assets and navigation
fixes do not constitute a shared-production authentication mode; existing
application-level workaround routes must be removed before upgrading.

### Mobile presentation invariant (12.1.0, carried forward to v13)

The published 12.1.0 mobile maintenance contract keeps Nexus navigation
dismissible by close control, backdrop, Escape and links, with keyboard focus
containment/return and visible no-JavaScript navigation. Portfolio scaffolds
must reflow at phone widths and wrap long content instead of hiding overflow.
These are presentation fixes, not changes to server authorization; pre-existing
generated source and showcase HTML-rewriting workarounds need explicit migration.

### Pre-release scaffold source invariant

An unpublished pre-release `cargo-rullst` may reuse only local framework crates
whose package names and versions exactly match that CLI. The invocation directory
is preferred; otherwise the exact still-present checkout from which the CLI was
compiled is used. Stable or version-mismatched packages fall back to crates.io.
This permits evaluation outside the repository without silently mixing release
trains, but generated absolute path dependencies remain non-portable until the
matching immutable release is published.

### Distributed operation tracing candidate (v13, source admitted)

The fifth approved September increment extends Core/facade's optional
`telemetry` capability, using the maintained OpenTelemetry SDK and OTLP/HTTP
protobuf transport. The existing process-local `SpanCollector` and authenticated
Studio ingestion remain separate interfaces. A new explicit configuration must
select service and bounded operation labels, a trusted collector, bounded queue,
batch and export deadlines, and an owned flush/shutdown lifecycle. It must not
infer user identity or authorization from trace identifiers.

Context propagation accepts canonical nonzero W3C version-00 `traceparent` only
after an explicit upstream trust decision. Untrusted ingress starts a new trace;
arbitrary baggage and vendor tracestate are not propagated by this minimized
profile. An actual independent producer/consumer journey must preserve parent
relationships through the existing Messaging carrier and reach a disposable
standard collector. Export only bounded approved operation/service names,
trace/span relationships, timing, kind and status; omit application attributes,
events, error descriptions, SQL, tokens, bodies and ambient resource metadata.
This profile does not sanitize other tracing/logging layers.

Export must run outside request execution with a compatible SDK processor/client,
bounded response handling and no redirects or ambient proxies. Production uses
verified HTTPS; literal-loopback HTTP is an explicit test/local-collector choice.
Remote failure never falls back to a fake success. Malformed propagation, queue
pressure, unavailable/slow collectors, TLS denial, exact package consumption and
flush/shutdown require executable evidence. Sampling, retention, collector access,
durability, availability and deployment operations remain host responsibilities;
trace delivery is best effort, not an audit ledger or application transaction.
The implementation and actual collector/TLS/process evidence are documented in
[the tracing guide](distributed-tracing.md). Hosted source/package admission
passed in PR #236; final release admission remains separate. Legacy initialization uses compatible threaded export and the corrected
OTLP/HTTP endpoint; it retains its separate general logging/metadata policy.

### Recoverable Live UI candidate (v13, source admitted)

The fourth approved September increment extends `rullst-core::live` with a
separate opt-in typed recovery API, preserving the legacy component interface.
An application-bound tenant/account/component scope and mandatory static-dispatch
authorization callback must be checked before upgrading, before actions and
outputs, and periodically while connected. Exact browser origin and protocol
validation supplement, rather than replace, application authentication.

The v1 transport sends bounded complete server snapshots on initial connection,
reconnection and conflicts. Commands carry a bounded correlation ID and expected
revision; the host must check that revision atomically with its persistent domain
write. The client must not queue or automatically replay mutations after uncertain
disconnects. Recovery converges to authoritative application state, not replay of
every missed transient event. Browser/server buffers, admitted connections,
callbacks, sends, session lifetime and event counts must be bounded. Expired or
revoked access closes the session without exposing a new snapshot.

A small optional same-origin browser module and real browser/server tests must
prove reconnect convergence, action conflict, session/tenant denial, revocation,
bounded malformed/slow peers and recovery after an application restart. Domain
persistence, transactional revision checks, HTML escaping, authorization policy,
replication and proxy configuration remain explicit application responsibilities.
No automatic durability or upgrade of legacy `LiveComponent` code is implied.
The implementation and its local protocol/Chromium acceptance are recorded in
[the recovery guide](live-recovery.md). Hosted workspace/platform/package
admission passed in PR #236; final release admission remains separate.
The API uses complete snapshots; it does not imply
DOM diffing, broadcast of other clients' changes or automatic event replay.

### Remote messaging candidate (v13, source admitted)

The first remote adapter is an opt-in Redis Streams profile in the existing
`rullst-messaging` crate. Redis stores the canonical envelope log; Rullst owns
bounded group, retry, dead-letter and single-use lease indexes. These are Rullst
consumer groups, not interoperability with arbitrary native `XREADGROUP` clients.
Every lease mutation must check authoritative server time and the exact current
capability atomically. Publication replay must retain the original receipt until
explicit terminal purge. Empty/mock credentials select an explicit process-local
fixture; a failed live connection must never downgrade to that fixture.

The initial profile targets a dedicated standalone Redis 7.4+ database with
verified TLS outside an explicit literal-loopback test mode. Exact configuration
and an application-supplied deployment generation bind persisted namespaces;
provisioning is explicit and normal connection must not recreate lost state.
The TLS adapter preserves an installed Rustls process provider and otherwise
installs ring before Redis creates its client, avoiding implicit-provider panics
when dependency features enable multiple providers. Applications requiring a
different provider install it before broker startup; trust and hostname checks
remain mandatory.
Lua mutations are isolated but do not roll back on runtime errors: a persistent
in-progress marker must quarantine partial changes rather than silently continue.
Bounded batches, retained bytes, subscriptions, operation deadlines and admission
limits are required. AOF/no-eviction, clock discipline, credentials/ACLs, backups,
restore reconciliation, topic authorization and destination idempotency remain
deployment responsibilities. Cluster, replication/failover, native-group mixing,
automatic repair and exactly-once effects are excluded from this profile.

Admission requires the shared broker contract plus an actual disposable Redis
restart, conflicting publication, concurrent consumer, expired/late ACK, retry,
DLQ, outbox replay, configuration-loss and partial-write failure evidence.
The implementation candidate and its capability/operational limits are described
in [the Redis messaging guide](redis-messaging.md). Hosted source/package
admission passed in PR #236; final release admission remains separate.

### Shared-local facade composition invariant

The umbrella features `auth-sqlite`, `capital-quota-sql`, `oauth-sqlite`,
`mail-sqlite`, `messaging-sqlite`, and `queue-sqlite` may deliberately share one
file-backed SQLite database for a bounded single-host deployment. Each subsystem
owns a fixed, distinct table namespace. The host prepares and checks every
store sequentially and keeps `ApplicationLifecycle` unready until all required
components have succeeded. Restart must reuse the same validated database URL,
quotas, namespaces, and encryption keys; all handles must close before a host
copies, restores, or replaces the file.

The executable facade contract proves persistence and idempotent replay across
restart, encrypted token/message plaintext absence from the database/WAL/SHM,
queue recovery, aggregate readiness, and fail-closed token corruption without
breaking an unrelated mail read. It does not create a cross-subsystem
transaction, consistent online backup, key manager, distributed database, or
multi-host coordination. Whole-file snapshot consistency, filesystem trust,
permissions, key/backup operations, contention policy, and recovery drills are
host responsibilities. Separate databases remain preferable where failure
isolation or write throughput matters.

### Native relational enum invariant

`rullst-orm` has one bounded native-enum contract. `#[derive(Enum)]` generates
a closed label set shared by `Display`, parsing, Serde, `RullstValue` and SQLx
codecs. A database enum has 1–64 unique labels; its type identifier and labels
use the documented bounded ASCII allowlists. `Blueprint::native_enum` emits:

- a named PostgreSQL enum with exact existing-label drift detection only under
  `strict-postgres`;
- an inline `ENUM` for MySQL/MariaDB; and
- a `TEXT CHECK` constraint for SQLite.

PostgreSQL through SQLx Any must fail before DDL because that driver cannot
decode custom PostgreSQL types. Adding, removing or reordering variants,
deployment order, dependent-object removal and rollback remain explicit,
reviewed migration work. The schema helper does not auto-migrate an existing
type or infer application compatibility.

The Capital row also includes one implemented, feature-gated quota boundary:
`BillingSubject` binds a shared team/workspace counter to trusted tenant state,
`Billable::quota_request` derives the limit from the subscription owner, and
`QuotaStore` atomically reserves idempotent units before resource creation. The
deterministic local store is always available; `quota-sql` uses fixed-schema
SQLx transactions on SQLite/PostgreSQL/MySQL/MariaDB, with exact replay/release
and a caller-owned transaction path for atomic domain writes. Live container
contracts exercise all four protocols. Membership establishment, plan state,
migrations, reconciliation and non-relational adapters remain application
boundaries.

### 3.1. Generated Academy activity boundary

The generated `ActivityEvaluator` boundary uses static dispatch and accepts an
untrusted submission, never client-supplied points. It validates authenticated
ownership, activity/ruleset identity, bounded object-shaped state, server-time
ordering and a canonical evidence digest, then constructs `ActivityResult` from
the evaluator's outcome. Built-in bounded evaluators cover single-choice, a
complete permutation of at most eight matching pairs and typed recall with a
closed answer set, 512-byte/control-character boundary, trim and optional
Unicode lowercase comparison. Typed replay persists a policy-bound SHA-256
digest rather than raw input; it does not perform Unicode normalization,
accent/fuzzy matching or make the digest non-personal data. The complete
Academy starter's `record_activity_result` rechecks the authenticated actor,
loads course/kind/maximum/ruleset/season and
the canonical evidence digest and exact evaluator configuration from persisted
activity state, rejects any divergence, then atomically appends an exact-replay
activity-attempt record,
`ScoreEvent` v2, the leaderboard update and `score_recorded`. The generated
owner-only `POST /activities/{id}/attempts` accepts only an idempotency key and
selected option. `POST /activities/{id}/attempts/matching` accepts only an
idempotency key and bounded pair IDs, while
`POST /activities/{id}/attempts/typed` accepts the key and bounded learner text.
All derive learner/activity identity, policy, answers, points, evidence and time
from authenticated/server state. Durable attempt identity is scoped by learner
and activity, and the event idempotency key is derived by the
server rather than trusted as a global client namespace. The application must
keep evaluator answer rules in trusted state and include retained attempt state
in its privacy lifecycle. Listening/game evaluators and unification with
the separately persisted quiz evaluator remain roadmap work.

Activities may opt into the exact `rullst-box-v1` review policy. For a newly
applied score, the score transaction locks and validates that versioned policy,
loads the learner/activity review state, applies a deterministic bounded
pass/lapse transition and upserts the next due time before commit. An exact
activity replay exits before this transition and therefore cannot advance the
schedule. The owner-only `GET /reviews/due` derives the learner and current time
from server state and returns at most 50 due activities after rechecking active
school membership, course scope and enrollment. Invalid policy/state or a
changed algorithm version fails the score transaction closed. This is a simple
inspectable scheduling foundation, not FSRS/SM-2 compatibility, efficacy proof,
AI personalization, generated pedagogy or a complete adaptive-learning system;
PostgreSQL/MySQL contention evidence also remains open.

---

## ⚡ 4. Core API Specifications (`rullst-core`)

`rullst-core` provides the runtime kernel. Database and queue drivers are modular and feature-gated.

### 4.1. Server & Routing (`rullst::routing`)
* **Routing Macro:** Central declarative routing declared via the `routes!` macro wrapping Axum routing handlers:
  ```rust
  use rullst::{response::Html, routes};

  async fn home() -> Html<&'static str> {
      Html("Home")
  }

  async fn posts_index() -> Html<&'static str> {
      Html("Posts")
  }

  let router = routes![
      get("/" => home),
      get("/posts" => posts_index),
  ];
  ```
* **Server Lifecycle & Graceful Shutdown:**
  ```rust,no_run
  use rullst::{Server, routes};

  async fn serve() -> Result<(), rullst::server::ServerError> {
      let router = routes![get("/" => || async { "OK" })];
      Server::new(router).run(3000).await
  }
  ```
  `ApplicationLifecycle` is the opt-in orchestration contract behind a
  lifecycle-aware server. Its phase is monotonic, its immutable registry has at
  most 32 validated required-component labels, and request admission requires
  both `Ready` and every component bit. Exact `GET`/`HEAD` health probes bypass
  admission so `/ready` can return a bounded `503` during startup, dependency
  failure or drain while `/health` stays process-only. The JSON reports counts,
  not labels or dependency errors. `Server::run_with_shutdown` accepts a
  caller-owned future; when it resolves, the lifecycle changes to draining
  before Axum waits for accepted requests and then becomes stopped. Dependency
  checks/timeouts, component updates, replica consensus, load-balancer timing,
  authorization and the deployment termination deadline remain host contracts.
* **Default Dynamic Cache Boundary:** `headers_middleware` supplies
  `Cache-Control: no-store` only when the handler has not already selected an
  explicit cache policy. Versioned public/static responses can therefore opt
  into reviewed caching without weakening the default for dynamic data.
* **Double-Submit Form Contract:** `csrf_middleware` installs the exact
  request-scoped `CsrfToken` used by the CSRF cookie on eligible safe requests
  and preserves it after a valid state-changing request. Server-rendered forms
  must echo that value in `_token`; HTMX/JavaScript may instead send it through
  `X-CSRF-Token`. Nested application and `Server` baseline composition is
  request-idempotent: exactly one CSRF layer owns token validation/cookie
  emission, so an explicitly protected router remains valid when the production
  server wraps it. The cookie intentionally remains script-readable and must
  not be confused with an authentication or session cookie.

### 4.2. Server-Side Rendering (`rullst::macros`)
* **Macro:** `html!` expands supported HTML trees into ordinary Rust `String`
  construction at compile time.
* **XSS Protection:** Dynamic display values in the supported `{expr}` syntax
  are HTML-escaped by the generated code.
* **Raw Unescaped HTML:** Explicitly bypassed using the wrapper `rullst::html::RawHtml(String)`.
* **Example:**
  ```rust
  use rullst::html;

  let username = "<script>alert('xss')</script>";
  let rendered = html! {
      <div class="user-badge">
          <span>"User: "{username}</span>
      </div>
  };
  // Automatically escapes to: &lt;script&gt;alert('xss')&lt;/script&gt;
  ```

### 4.3. Durable Queue Timing and Completion History
* SQLite and Redis persist `dispatch_at` for at most 366 days and never claim a
  job before its stored millisecond due time. Execution remains poll-dependent
  and at-least-once.
* Successful SQLite jobs are deleted by default. The explicit
  `Queue::sqlite_with_completed_history` constructor validates a 1–100,000 row
  limit, changes a processing row to `completed`, and prunes excess history in
  the same transaction.
* `Queue::purge_completed_history` removes those opt-in retained successes.
  Rows contain the original payload, so Studio access, data minimization and
  retention policy remain host responsibilities. Redis/custom drivers expose
  inspection or history only when their capability implements it.

---

### 4.4. Private S3-compatible storage (v13, source admitted)

The opt-in `storage-s3` Core/facade feature extends the existing `Storage` and
`TenantStorage` APIs with explicit AWS S3 or Cloudflare R2 credentials. It must
not add cloud dependencies to bare Core or infer credentials from the ambient
environment. The provider constructors alone remain unconfigured until a
validated cloud configuration is attached.

The supported journey is bounded object upload, download, metadata, deletion
and short-lived signed GET URLs. Use the maintained AWS SigV4 signer, explicit
provider endpoint/region rules, HTTPS, no redirects or ambient proxies, bounded
request/body budgets and redacted errors. A separate development configuration
may target a literal loopback endpoint for disposable protocol acceptance; it
must not pass a production-configuration check. Empty or `mock_*` credentials
select a deterministic bounded in-memory fallback, also rejected by that check.
No mock URL may impersonate a signed provider URL.

Tenant wrappers bind all five operations to the authenticated namespace. The
application still authorizes the current account and object before every
operation or grant; namespacing alone is not authorization. Private access URLs
are bearer secrets, remain usable until expiry/provider invalidation, and must
not appear in Debug/errors. They are not single-use or individually revocable.
Direct presigned uploads, bucket/IAM provisioning, image processing and arbitrary
S3 operations are outside this increment. Existing file quarantine/admission
remains an explicit application step before cloud persistence or sharing.

Acceptance requires feature-off/on and MSRV checks, an extracted-package
consumer, a tenant-bound application journey, independent signature/protocol
fixtures, bounded failure/expiry tests and a real disposable S3-compatible
service. Those tests do not establish interoperability with owner accounts,
provider configuration or regional legal compliance. Source/package admission
passed in PR #236; final release admission remains separate.

#### Resumable multipart candidate (source admitted, unpublished)

The separate opt-in `storage-multipart` extends this adapter with server-mediated
initiation, exact-size SHA-256-checked parts, bounded remote reconciliation,
completion and abort. A server-owned `MultipartKey` seals resumable checkpoints
with AES-256-GCM under a versioned purpose and exact endpoint/bucket/region/mock
profile/object binding (including the ephemeral instance in mock mode). Each
operation reopens the checkpoint and checks policy;
tenant wrappers derive that object only from the authenticated namespace. The
application must still authorize the current subject/object and serialize or CAS
checkpoint updates. A checkpoint is sensitive recovery state, never identity,
membership, malware clearance or permission to publish an object.

The initial scope fixes total length, part size (5–64 MiB), at most 256 parts
and a lifetime of one minute through seven days. Part bytes match a caller-supplied
SHA-256 before signing the exact payload with SigV4. ETags are opaque provider
receipts, not content hashes. Checkpoints retain receipts; resume lists provider
parts and reports missing/mismatched receipts for explicit re-upload. Completion
requires every consecutive receipt and rejects an embedded XML error even under
HTTP 200. A provider marker and exact length support explicit reconciliation of
lost completion responses, without guessing from object existence alone.

Only one part is buffered per call; applications bound concurrent calls. Responses
are byte/node bounded, prohibit DTDs and validate exact root/resource identity.
No public ACL, direct browser upload or ambient endpoint is introduced. Existing
ordinary-object download limits still apply to large completed objects. New
objects may replace a same-name object as with `put`; applications must allocate
fresh private/quarantine names and withhold reads until scanning and authorization.

Abort remains available for expired, authentic checkpoints and checks provider
absence; concurrent in-flight writes can require another abort. Retain cleanup
records durably and provision provider lifecycle expiration for abandoned uploads,
including initiation whose response was lost before an upload ID was obtained.
No application database, global sweeper, bucket provisioning or durable revocation
of checkpoints is implied. The offline backend has explicit bounded in-memory
state; native disposable S3, process/service restart, forged checkpoints, tenant
isolation, response failures and extracted consumers are required evidence.

## 🗄️ 5. Active Record ORM & Schema Engine (`rullst-orm`)

### 5.1. Model Definition & CRUD
```rust,no_run
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[orm(encrypted)]
    pub secret_token: Option<String>,
}

async fn use_users() -> Result<(), rullst_orm::Error> {
// Queries (after `Orm::init(...)` and schema migration at startup).
let all_users: Vec<User> = User::all().await?;
let user: Option<User> = User::find(1).await?;

// Mutations
let mut new_user = User { id: 0, name: "Alice".into(), email: "alice@example.com".into(), secret_token: None };
new_user.save().await?; // Auto-executes parameterized INSERT or UPDATE
new_user.delete().await?;
let _ = (all_users, user);
Ok(())
}
```

The `Orm` derive grammar is fail-closed. Model and field attributes are parsed
as structured nested metadata; unknown or duplicate options are compile
errors. Every SQLx model requires a persisted named `id` field. Explicit
table/column/relation identifiers use the 1–64 byte portable ASCII identifier
grammar, and declared hook, scope, policy, relation-model, tenant, soft-delete,
and embedding references are validated before code generation. A relation
field accepts exactly one relation declaration; options that do not apply to
that relation fail compilation. `belongs_to_many` requires `pivot_table` and
defaults omitted owner/related pivot keys from the two model names.

Only `skip`, `default`, `json`, and `json(nullable)` from SQLx field metadata
are compatible with generated ORM persistence in v12. `rename`, `try_from`,
`flatten`, and unknown SQLx options fail compilation instead of letting the
decoded shape drift from generated SQL. Soft-delete sentinel expressions are
bounded compile-time SQL fragments, not parameterized runtime values: they are
capped at 128 bytes and reject statement separators, NUL, and SQL comments,
while portability and semantic review remain the model author's responsibility.

### 5.2. Parameterized Queries & Privacy
* Values accepted by non-raw query APIs use SQLx parameterization. Structural
  identifiers use the bounded ASCII grammar. pgvector helpers bind canonical
  vector/distance strings after rejecting empty/non-finite vectors and invalid
  distances; they do not interpolate those runtime values. Methods explicitly
  suffixed/named `raw` remain caller-owned escape hatches rather than an
  injection-safety claim.
* Generated builders assemble bindings by emitted clause position (CTE, JOIN,
  WHERE/HAVING, ORDER BY), not by the order in which fluent methods were
  called. Nested typed subqueries export that ordered binding sequence.
* Generated magic filters bind supported primitive fields to their Rust type at
  compile time (`String`, `i32`, `f64`, and `bool`), and generated column enums
  make unknown columns unrepresentable on typed paths. String-column builders,
  custom `RullstValue` conversions and raw SQL are explicit runtime-checked or
  caller-owned alternatives, not compile-time schema verification.
* `String` and `Option<String>` fields annotated with `#[orm(encrypted)]` are encrypted before generated ORM writes and decrypted after generated model reads using AES-256-GCM. Randomized ciphertext cannot be filtered, ordered, grouped, or explicitly selected by generated query-builder methods; use a separately reviewed blind index when equality lookup is required. Raw SQL remains an explicit, non-transparent escape hatch.

### 5.3. Generated Relationship Contract

* SQLx models may declare `morph_many`, `morph_one`, and one or more explicit
  typed `morph_to` targets. A polymorphic relation requires
  `morph_name = "..."` (`name` remains a legacy alias).
* `morph_to` fails macro expansion unless the source has a persisted bindable
  `<morph_name>_id` field and a persisted `String` discriminator named
  `<morph_name>_type`. `foreign_key` may override the ID field and
  `related_key` may override the target key.
* The discriminator stores the Rust target model name. Lazy loading returns
  `None` for a different target; eager loading batches each declared target and
  never guesses an undeclared runtime type. Target models used in eager inverse
  loading must implement `Clone`.

### 5.4. Tenant Scope Contract

* A SQLx model declaring `#[orm(tenant_column = "tenant_id")]` must have a
  persisted `String`, `i32`, `f64`, or `bool` tenant field. The derive rejects a
  missing or unsupported field type.
* Generated queries fail closed when called outside `with_tenant(...)` and bind
  the active tenant inside the scope. Generated full/partial updates and
  instance delete/restore paths reject a model from another tenant.
* `Model::unscoped()` is the explicit global escape hatch. Deciding who may use
  it, deriving tenant identity from authenticated state, and database-level RLS
  remain host responsibilities.
* SQLx builders keep offset-based `chunk(...)` for compatibility and expose
  fallible `chunk_by_id(...)`/`chunk_by_id_with_tx(...)` for stable ascending
  keyset traversal over the generated `i32` primary key. This prevents deletes
  of processed rows from shifting later rows behind an offset; it is not a
  database-server cursor or a universal cross-shard snapshot.
* A model delete with marked `cascade_soft_delete` has-one/has-many relations
  runs parent and direct-child mutations in one transaction. An existing
  explicit or task-scoped transaction is reused; otherwise `delete()` opens,
  commits, or rolls back its own transaction. Recursive descendant/cycle
  traversal remains a separate contract.
* Generated `#[orm(auditable)]` instance `save()`/`delete()` operations write
  their bounded audit entry through the same explicit, implicit, or task-scoped
  transaction as the model mutation. Audit write errors fail the mutation and
  roll its savepoint back; direct `log_audit` calls also honor a task-scoped
  transaction. Every recorded mutation requires an `AuditContext` naming a
  validated user, service, or system principal; the record also carries the
  optional correlation identifier and derives its typed tenant key from the
  active `with_tenant(...)` scope. The host remains responsible for deriving
  both contexts from authenticated authority rather than client assertions.
* `create_audit_table` creates the v2 schema and adds its columns to a legacy
  table without presenting legacy rows as v2 evidence. JSON payloads are
  bounded and recursively mask sensitive names for create, update, and delete;
  audit/debug output does not expose principal, tenant, correlation, reason, or
  payload values.
* An auditable model exposes `restore_revision(audit_id, reason)` and its
  caller-owned transaction variant. Only a bounded v2 update patch for the
  exact model, ID, and active tenant is eligible. The current row must still
  match the revision's recorded post-state; PostgreSQL and MySQL/MariaDB also
  lock that row during restoration. A successful restore is a compensating
  audited update referencing the restored audit ID and reason. Legacy,
  create/delete, oversized, stale, malformed, cross-tenant, or redacted-field
  revisions fail closed. Bulk builders still do not synthesize per-row history,
  and durable external export remains an explicit outbox/application contract.

### 5.4.1. Transactional partial-update candidate (source admitted)

* `update_partial().save()` and its new explicit `save_with_tx(...)` variant
  merge selected typed values into the current persisted row within a savepoint.
  PostgreSQL and MySQL lock that row with `FOR UPDATE`; SQLite retains its
  transaction/locking semantics and can reject contended read-to-write upgrades.
  Missing rows and cross-tenant model handles fail before mutation.
* The merged model uses the normal generated save lifecycle, including policy,
  hooks, observers, encrypted fields, atomic audit entries and post-commit
  cache/Scout/observer effects. This is a logical partial change implemented
  through a full-row save, not a selected-column SQL optimization. Existing
  model hooks can transform that candidate under the normal save contract.
* The caller's object is replaced with the fresh merged model only after the
  operation succeeds. A direct save waits for its transaction commit; an
  explicit or task-scoped save reflects the transaction's tentative state, so
  the caller must discard/reload it if the enclosing transaction rolls back.
  A `PostCommit` failure does not undo a durable direct save. An ambiguous
  database commit error requires reconciliation rather than blind replay.
* An empty builder remains a no-op. A failed policy, hook, SQL write or audit
  rolls back the operation's savepoint and discards its pending effects. This
  allows a managed outer transaction to catch that failure and continue.
  Strict post-commit timing requires `Orm::transaction` or the owned direct
  path; a raw SQLx transaction retains the documented observation limitation.
* Unselected persisted values and relation fields in the caller's object are
  refreshed from the database representation. Unsubmitted edits on that object
  do not become an implicit patch. This is not an expected-version comparison:
  callers needing conflict rejection must still supply their domain revision
  contract. Local SQLite/PostgreSQL/MySQL, cancellation, cache/Scout and
  extracted-facade journeys pass. Full hosted workspace/platform/package
  source admission passed in PR #236; final release admission remains separate.

### 5.5. Process-Local Post-Commit Contract

* `Orm::transaction` and direct generated model `save()`/`delete()` operations
  own a post-commit callback scope. `after_commit` callbacks registered within
  it run only after SQLx confirms commit and are discarded on rollback. When no
  managed transaction is active, `after_commit` executes immediately for an
  already committed/autocommit operation.
* Generated observers retain synchronous lifecycle callbacks such as
  `creating`, `created`, and `saved` for mutation validation. The separate
  `committed(ModelCommittedEvent)` callback receives an owned, hidden-field-
  aware snapshot after the managed commit. Generated Redis invalidation/pub-sub
  and Scout projections use this same post-commit boundary.
* Savepoint-scoped generated saves/deletes and revision restores collect their
  callbacks in a nested scope. The callbacks are promoted to the enclosing
  commit boundary only after that savepoint succeeds, so catching a failed
  auditable mutation cannot leak a later `committed` effect.
* Every queued callback is attempted. A failure is returned as `PostCommit`,
  whose contract explicitly means the database mutation is already durable.
  Applications must not retry the database mutation blindly from this error.
* A caller-owned raw SQLx transaction passed to `save_with_tx` or
  `delete_with_tx` does not expose its later commit/rollback decision to the
  ORM. Use `Orm::transaction` for the strict process-local boundary.
* These callbacks do not survive process failure and provide no retry,
  idempotency or cross-node delivery. Use the explicit durable outbox below
  for an irreversible or externally delivered effect; it is not enabled
  automatically by a generated observer.

### 5.6. Durable Transactional Outbox Contract

* `Outbox::enqueue` accepts only a currently managed `Orm::transaction` and
  writes `rullst_outbox` through that same transaction. A domain rollback also
  removes the event. `enqueue_with_tx` provides the equivalent explicit path
  for a caller-owned SQLx transaction. No implicit independent commit is
  permitted.
* `(stream, event_key)` is the database uniqueness boundary. Replaying the same
  key and exact event kind/payload returns the existing `i64` identifier;
  reusing the key with different content fails closed. `stream`, event key,
  event kind and worker identifiers use a bounded ASCII grammar, and serialized
  payloads are limited to one MiB.
* PostgreSQL, MySQL/MariaDB and SQLite share the outbox state machine. A claim
  increments attempts and receives a random token plus a bounded lease. Only
  that token may acknowledge or fail the event; expiration permits another
  worker to reclaim it. Failure schedules a bounded retry or moves the event to
  `dead_letter` at the configured attempt limit, including a worker that dies
  while holding its final lease.
* Delivery is **at least once**, not exactly once. A worker may perform its
  external effect and crash before acknowledgement, so consumers must use the
  stable stream/event key as their own idempotency key. Ordering across retries
  or concurrent workers is not guaranteed.
* `Outbox::install` is an explicit setup/test convenience and never runs at
  startup. `OutboxMigration` puts the same schema under the built-in reviewed
  migration lifecycle. The ORM does not infer tenant authorization from
  `stream`, automatically serialize model observers, dispatch HTTP webhooks,
  purge delivered rows or promise cross-database transactions.

### 5.7. Generated Redis Query Cache Contract

* The optional `redis` feature enables `.remember(seconds)` for generated SQLx
  reads. `Orm::init_redis_with_namespace(url, application_namespace)` is the
  recommended initializer when a Redis database is shared; the compatibility
  `init_redis(url)` initializer uses the literal namespace `default`.
* Versioned SHA-256 cache keys bind the validated application namespace, an
  opaque digest of the active tenant scope when present, table, generated SQL,
  and typed bindings. Raw tenant identifiers are not emitted in keys.
* Generated reads always bypass Redis inside explicit and task-scoped database
  transactions, so cached state cannot replace the transaction's own view.
  `remember(0)` is invalid. Outside transactions, explicitly requesting cache
  without initializing Redis fails closed as a configuration error; transport
  failures and corrupt cached JSON fail open to the authoritative database.
* Cache writes occur only after a successful database read and retain encrypted
  model fields as ciphertext. Generated model `save()`/`delete()` operations
  invalidate the active tenant/table's versioned keys only after commit through
  a bounded Redis `SCAN` plus asynchronous `UNLINK`; rollback preserves existing entries.
  Raw SQL, bulk builders, caller-owned raw transactions and writes from other
  processes cannot be inferred. Callers must retain a defensive TTL and treat
  Redis cluster/failover and durable invalidation delivery as separate
  application contracts.

### 5.8. Polyglot Persistence Boundary

* Optional persistence adapters are disabled by default and selected with
  `mongodb`, `duckdb`, `turso`, `surrealdb`, `qdrant`, or the `polyglot` convenience
  feature. The umbrella crate exposes matching `orm-*` features.
* `DocumentRepository<T>` provides create, find, replace, delete, and
  deterministic bounded listing. Collection names, document IDs, offsets and
  limits are validated before reaching a driver.
* `DocumentInventory<T>` is the separate identifier-preserving extension used
  by recovery tooling. Its stable ascending pages avoid breaking existing
  third-party `DocumentRepository` implementations.
* `export_document_snapshot` performs two matching bounded observations before
  sealing a versioned payload with AES-256-GCM. The authentication data binds
  the key-rotation ID, trusted application namespace and exact collection;
  decoding is capped at 64 MiB and 100,000 documents. Restoration accepts only
  an empty destination or an exact matching subset, never replaces or deletes,
  tolerates only an exact raced duplicate and verifies the complete final
  inventory. Applications must quiesce source/destination writers and durably
  store, rotate and protect keys/snapshots. Required destination schema must be
  provisioned first; driver/schema errors never become an implicit empty
  collection. The API is crash-resumable, not a cross-store transaction or
  managed backup service.
* `MongoDbStore<T>` uses the official MongoDB Rust driver and stores the
  portable `DocumentId` as `_id`; portable models must not define `_id`.
* `DuckDbStore` serializes access to its native connection and delegates every
  database operation to `spawn_blocking`. Dynamic values use prepared
  parameters, and callers must supply a `QueryLimit` before rows are
  materialized. Application-provided SQL text remains a trusted structural
  input.
* `TursoStore` speaks the official Hrana HTTP v3 protocol directly for remote
  edge SQL. It uses positional typed parameters, conditional atomic batches,
  a 30-second request deadline, no redirects, a 16-MiB response bound, bounded
  row materialization, and ordered checksummed migrations. This avoids an
  unnecessary embedded/native SDK dependency while retaining conformance
  against the official libSQL server. Empty or `mock_*` endpoints use a
  one-connection SQLite fallback that exercises real SQL without pretending to
  be a remote replica. HTTPS/`libsql://` is required outside explicitly enabled
  loopback development.
* `SurrealDbStore<T>` uses the documented `/key`, `/sql`, and `/gql` HTTP
  endpoints with namespace/database headers, no redirects, bounded streaming
  responses, HTTPS by default, and redacted authentication configuration.
  `GraphQuery::read_only` accepts one `MATCH` query, rejects mutation tokens
  and caller-supplied limits, then appends a bounded limit.
* This boundary does not turn every backend into SQL Active Record, perform
  cross-database transactions, provide an online-consistent snapshot,
  synchronize records between engines, or prove a third-party deployment. See
  the [Polyglot Persistence guide](polyglot-persistence.md).

### 5.9. Scout Search Projection Contract

* `#[orm(searchable)]` projects generated save/delete operations only after a
  managed relational commit. Search adapter failures remain visible; a failed
  query is not silently treated as an empty result, and `PostCommit` means a
  projection failed after the database mutation became durable.
* `MockSearchEngine` is deterministic and always available. The optional
  `scout-http` feature adds Meilisearch, Elasticsearch and Algolia. Empty or
  `mock_*` credentials select the mock; keyless live constructors accept only
  loopback HTTP, while remote/custom origins require HTTPS without URL
  credentials, redirects, paths, queries or fragments.
* Index names, positive IDs, object payloads, queries, response bytes and hit
  counts are bounded. Meilisearch/Algolia tasks use bounded polling;
  Elasticsearch requests use `refresh=wait_for`. Provider response bodies and
  credentials are not copied into transport errors.
* The repository proves a real Meilisearch lifecycle in a digest-pinned
  container. Elasticsearch and Algolia protocol fixtures prove the documented
  HTTP shape and bounds, not hosted service operation, version-wide
  compatibility, ranking quality or cluster failover.
* The generated hook remains process-local. Guaranteed crash recovery requires
  an application-versioned event in the transactional `Outbox` and an
  idempotent worker; the ORM cannot infer a safe event key or external retry
  policy from an arbitrary model save.

### 5.10. PostgreSQL pgvector Contract

* The optional `pgvector` feature re-exports the SQLx-compatible `Vector` type.
  The supported execution profile combines it with `strict-postgres`; other
  SQLx backends do not pretend to implement PostgreSQL vector operators.
* `where_similar`, L2, cosine and inner-product ordering validate column names,
  reject empty/non-finite vectors and invalid distances, and bind vector and
  distance values. ORDER BY bindings are assembled after WHERE bindings
  regardless of builder call order.
* A digest-pinned PostgreSQL + pgvector container installs the extension, uses
  a typed vector model and proves L2 threshold/cosine ordering queries. The
  application still owns reviewed migrations, vector dimensions, embedding
  model compatibility, HNSW/IVFFlat index selection/tuning, tenant policy,
  context budgets, citations, ingestion/deletion and RAG evaluation.

### 5.11. Qdrant and Redis Specialized Store Contract

* The optional `qdrant` feature exposes a separate `VectorRepository` rather
  than pretending Qdrant is SQL Active Record. Collection names, dimensions,
  vectors, cosine norm, point payloads, query limits and response bytes are
  bounded. The HTTP client rejects redirects and URL credentials, uses short
  connect/request deadlines, requires HTTPS outside loopback, redacts API keys,
  and never copies provider response bodies into errors.
* `QdrantConfig::new` selects a deterministic in-process fallback for empty or
  `mock_*` endpoint/API-key values. `unauthenticated_local` is an explicit
  loopback-only path for self-hosted development. The supported live API is one
  unnamed dense cosine vector per numeric point with create, single-point
  upsert/delete and bounded nearest-neighbor query; named/sparse/multivectors,
  arbitrary filters, collection tuning and distributed topology are outside it.
* The optional `redis` feature exposes `RedisDataStore` for explicitly
  namespaced Hash, Set and Sorted Set operations in addition to the generated
  query cache. Keys, fields, UTF-8 values/members, finite scores and scan/range
  materialization are bounded. Remote endpoints require `rediss://`; URL
  credentials are rejected, ACL credentials are redacted, operations have
  connection/response deadlines, and empty/`mock_*` credentials select a
  deterministic in-process fallback.
* Digest-pinned Qdrant and Redis matrices prove their respective live
  lifecycles, including Redis namespace/structure separation. They do not prove
  hosted-provider availability, backups, cluster failover, tenant
  authorization, eviction policy, ANN quality, or cross-store transactions.

### ORM Driver Selection

* ORM defaults retain SQLite, PostgreSQL and MySQL/MariaDB through the explicit
  `drivers-all` convenience feature. A standalone consumer can disable defaults
  and select `strict-postgres`, `strict-mysql` or `strict-sqlite`; each enables
  only its own SQLx backend. The strict pool's existing precedence when multiple
  strict features are unified remains PostgreSQL, then MySQL, then SQLite.
* Features are additive. Another dependency enabling ORM defaults, a SQLite
  queue, Turso's offline SQLite transport or another SQLx driver can broaden the
  final graph. Core, Studio, Nexus and the facade propagate an explicitly
  selected backend when defaults are disabled. Studio's SQLite queue is now
  explicitly optional; defaults retain `drivers-all` and `queue-sqlite`.
  Fifteen standalone normal/build graphs cover ORM/Core/Studio/Nexus/facade
  across all three SQLx backends. Capital/other optional capabilities may
  intentionally broaden those graphs. Turso enables SQLite for its offline
  contract.
* A standalone consumer check must compile generated model/query/transaction
  code and inspect its normal/build graph for unrelated SQLx driver packages.
  Workspace all-feature or all-target checks cannot prove driver isolation.
  Applications previously disabling defaults without selecting a driver must
  choose a strict backend or explicitly restore `drivers-all`.
* Native enum codecs follow the ORM's selected drivers, not identically named
  features in the consuming application. ORM 12.1 opts its paired macro crate
  into `runtime-driver-codecs` and exports hidden, driver-gated codec helpers.
  The macro crate's default expansion remains compatible with the all-driver
  12.0 runtime; the opt-in helper expansion requires the matching 12.1 runtime.
  Standalone isolation checks include enum encoding/decoding as well as CRUD.

### 5.12. ORM Telemetry Contract

* Generated model/query entrypoints, transaction-aware variants, raw ORM
  queries and generated streams emit `rullst.orm.query` spans with only a
  static model, validated table and bounded operation name. SQL text, bindings,
  model values, DSNs and error strings are not fields of these Rullst-owned
  spans. The explicit debug query logger remains a separate opt-in surface.
* Managed transactions emit begin and lifecycle spans. Their final outcome is
  one of the bounded commit/rollback states; transaction errors are returned to
  the caller rather than copied into telemetry. Generated stream spans are
  entered only while the stream is polled, so a tracing guard is never held
  across suspension.
* Every pool constructed through `Orm::init*` emits SQLx pool-acquire timing at
  info level and promotes acquisitions slower than 500 ms to warnings. Primary
  and replica pools share this configuration. Direct pools constructed by the
  application are outside the contract.
* These standard `tracing` spans/events are exported when the host enables the
  umbrella `telemetry` feature and initializes Core's OpenTelemetry subscriber.
  The host still owns OTLP endpoint security, filters, sampling, retention and
  collector availability. SQLx or application logging configured separately
  may have its own statement-data policy.

---

## 💳 6. Billing, Payments & Fiscal Engine (`rullst-capital`)

`rullst-capital` exposes bounded billing-provider and payout-provider adapters,
plus a bounded Brazilian digital-invoicing preparation pipeline (NFS-e
Nacional). Local cryptographic/schema validity is not tax authorization.

Every reviewed live adapter uses the same fail-closed outbound HTTP boundary:
redirects and ambient proxy environment variables are disabled, connection and
whole-request timeouts are finite, and a successful provider response is read
only up to one MiB before JSON decoding. Failures expose a redacted typed
`ProviderFailure` contract with permanent, transient, and rate-limited classes;
only a bounded numeric `Retry-After` delta is retained. Raw response bodies,
request URLs, credentials, and transport diagnostics are not included in the
public error. Rullst deliberately does not retry billing mutations: callers may
retry a transient or rate-limited result only when that exact operation has a
persisted provider-forwarded idempotency key and a reconciliation policy.
Returned checkout locations are accepted only as bounded, absolute,
credential-free HTTPS URLs. Stripe's documented opaque hosted-URL fragment is
preserved; other adapters reject fragments. Provider/account sandbox acceptance
remains external evidence.

### 6.1. Multi-Gateway Payment Architecture
Billing adapters implement `BillingProvider`; the Wise payout adapter implements
the separate `PayoutProvider` contract. Individual billing operations may still
return `Unsupported` when a provider adapter has no reviewed implementation:
```rust,no_run
use rullst_capital::providers::stripe::StripeProvider;
use rullst_capital::providers::BillingProvider;

async fn create_checkout() -> Result<(), rullst_capital::CapitalError> {
let provider = StripeProvider::new("mock_api_key", "mock_webhook_secret");
let session = provider
    .create_checkout_session(
        "customer@example.com",
        "price_monthly",
        "https://example.com/billing/complete",
    )
    .await?;
let _ = session;
Ok(())
}
```

`#[derive(rullst::Billable)]` is the umbrella convenience for named structs with
an `email: String` field. It preserves generics; optional
`subscription_id: Option<String>` and `tier: Option<String>` fields expose the
corresponding helpers. An all-or-none
`grace_period_starts_at: Option<i64>`/`grace_period_ends_at: Option<i64>` pair
exposes a validated half-open window of at most 366 days. A provider-bound
`SubscriptionHandle<P>` delegates cancellation and pausing; the explicit
`subscription_with` path keeps static dispatch. These values do not infer or
persist ownership, team membership, entitlement, currency, payment methods,
usage or provider scheduling. Shared quota accounting is a separate explicit
boundary described below.

The same derive inherits the bounded `charge_with`/`charge` helpers for an
immediate off-session charge. A charge requires a positive integer amount in
currency minor units (maximum eight digits), a three-letter currency, explicit
provider customer and tokenized payment-method IDs, the model e-mail and an
application-owned idempotency key of at most 255 bytes. `BillingProvider::charge`
defaults to `UnsupportedOperation`; the reviewed live implementation is Stripe
Payment Intents, which forwards the idempotency key, confirms off-session and
fails closed on an amount/currency mismatch or a status other than `succeeded`
or `processing`. Empty/`mock_*` Stripe credentials return a deterministic local
receipt with the distinct non-success `Mock` status for exact retries. Rullst
does not model raw payment credentials, prove that a stored method has a valid
mandate, persist idempotency, grant an entitlement, reconcile webhooks or imply
direct-charge parity across adapters.

Paddle signature verification accepts any matching `h1` candidate under a
4 KiB header and 16-candidate bound, with constant-time comparison for each
decoded HMAC. Exactly one nonempty `ts` timestamp is required; malformed
neighboring signatures cannot hide a valid one, and duplicate timestamps are
rejected. The configured freshness window still applies to the exact raw body.
This authenticates a delivery, not subscription ownership or settlement.

`PaddleCustomerRequest` and `PaddleProvider::create_customer` bind explicit
customer provisioning to an opaque owner and durable attempt. The typed
`PaddleCheckoutRequest`/`create_transaction_checkout` replacement uses an
existing customer, one server-owned recurring price, quantity one, automatic
collection and an approved Paddle.js payment-link page. It verifies customer
ownership before mutation and response identity, metadata, recurring price and
the exact payment page plus transaction-bound `_ptxn` before returning a URL.
The page is a checkout launcher, not an after-payment return URL. The account's
default payment-link configuration and domain approval remain prerequisites.

Paddle's sandbox is selected explicitly. Receipts distinguish offline mocks
from real selected-environment evidence. No arbitrary provider idempotency key
or automatic retry is promised: persist intent before dispatch and reconcile
uncertain outcomes by independently recovered known customer/transaction IDs.
Do not discover or claim ownership by contact email. Signed subscription events
must match owner, attempt, customer and recurring price; initial creation also
matches the persisted transaction ID, and later lifecycle events require its
bound subscription ID. Current subscription reads preserve status and billing
periods. Hosts retain account/environment scope, atomic inbox/domain commits,
revision fencing and entitlement/settlement policy. Cancellation/pause use the
selected API and accept only matching immediate or scheduled changes. Legacy
email/price-only checkout remains unsupported because it cannot express these
bindings. Generated durable billing integration remains Stripe-specific.

`PaddleProvider::create_bound_customer_portal` separately creates an overview
session after reading the active customer and verifying the original owner and
customer-provisioning attempt. Hosts must authenticate and authorize the caller
and load these references plus the customer ID from trusted tenant, provider
account and environment-scoped state. Metadata must remain host-owned; the
provider read and session creation are not atomic. Portal access is customer-wide,
not limited to one subscription. No email lookup, arbitrary return URL, automatic
retry, caching or subscription deep links are included. The legacy generic
email-based portal method remains unsupported with live credentials.

The response must match the customer and a valid session ID. Overview URLs are
bounded to 8 KiB, HTTPS, the exact selected environment's
`customer-portal.paddle.com` or `sandbox-customer-portal.paddle.com` host, a
`/cpl_` plus 26-character lowercase alphanumeric path, and exactly one
`action=overview` and nonempty `token` query pair. Userinfo, nondefault ports,
fragments, whitespace/control characters and ambiguous or additional query
fields fail closed. This is an explicit API profile, not a general URL resolver.
Receipts redact Debug and expose no Serialize/Clone implementation; their owned
URL is zeroized on drop without claiming erasure of parser or caller copies.
Hosts must not store/log/cache the bearer URL and must deliver it only to the
authorized customer with no-store/no-referrer response headers. Provider expiry,
single use and remote revocation are not inferred. Empty/`mock_*` keys preserve
owner binding and return deterministic `example.invalid` receipts that
`require_real` rejects. Local transport fixtures and archive consumers do not
establish live-account interoperability. This API does not enable Paddle in the
generated durable SaaS flow.

`PolarCheckoutRequest` and `PolarProvider::create_product_checkout` implement
POST `/v1/checkouts/` with one explicit product UUID, stable opaque external
customer identity, HTTPS success URL and optional contact email. The response
must bind product, external customer, metadata and redirect before a session is
returned. `with_sandbox` selects Polar's isolated API; it never infers environment
from a token. Client IP is optional and accepted only as a typed address supplied
by the host's reviewed trusted-proxy boundary; raw forwarding headers are never
read by the adapter. No undocumented idempotency guarantee or automatic retry is
introduced. `verify_checkout_subscription` verifies Standard Webhooks and binds
the nested customer's external identity and product to the persisted checkout
request. Hosts retain account/environment scope and atomic event processing.
The old email/price-only trait method cannot supply those bindings and remains
explicitly unsupported for live use; its replacement is the typed product API.

Polar's signed subscription normalizer accepts only its explicit lifecycle
events and states, with bounded subscription/customer/product identities and
consistent current/legacy customer references. RFC3339 billing periods and the
current nested customer contact are preserved; positive integer periods and
unambiguous legacy user/price references remain compatible. Scheduled
cancellation can retain an active state until final revocation. Order/payment
events, ambiguous identities and malformed present periods are rejected.
The legacy normalized event does not retain provider account/mode, scheduled
flags or ordering metadata; hosts must retain those from verified raw events
and reconcile ownership/settlement before changing entitlements.

#### Customer-bound Stripe Subscription Checkout

`StripeCustomerRequest` and `StripeProvider::create_customer` supply the
preceding provider-customer operation. The immutable input binds an opaque
local owner reference, a persisted retry key and optional bounded contact
email. The response must be an undeleted customer with matching metadata,
valid ID/time and the credential's test/live mode when that mode is known.
Mock receipts are separate from provider creation. Requests carry a versioned
digest, explicit idempotency header and the pinned checkout API version.
The host must persist provisioning intent before HTTP, bind the result to the
authorized account/owner before checkout, and reconcile unknown outcomes;
provider idempotency retention is not durable application state. Email is not
used to find or establish ownership of an existing provider customer.

The additive `StripeCheckoutRequest` binds an existing provider customer, one
server-owned recurring price, an opaque local owner reference, explicit HTTPS
success/cancel URLs and an application-persisted idempotency key. The dedicated
Stripe method sends that key and reference with the customer ID; it does not
create or discover a customer by email. A bounded response must match the
customer, owner reference, requested price/quantity, subscription mode and
redirects before a checkout session is returned. Request/receipt debug output
omits identifiers, URLs and keys. Empty/`mock_*` credentials produce an explicit
deterministic mock, never payment evidence.

The application must commit customer/tenant ownership and an immutable attempt
with its request digest before dispatch, retain account/test-live namespaces,
and reconcile unknown outcomes. Provider idempotency has a finite retention
window; repeating an expired key is not a durable deduplication guarantee.
Session creation and return navigation never grant paid access. The legacy
email-based trait method remains source-compatible; new generated Stripe flows
use the dedicated customer-bound contract and durable persistence below.

`StripeSubscriptionLookup` and `StripeProvider::retrieve_subscription` provide
an explicit read for reconciliation. The request binds subscription, persisted
customer/local reference, expected price and test/live mode; the pinned-version
response must match every binding and the bounded single-item state contract.
`StripeSubscriptionSnapshot` preserves the exact provider status and separates
retrieved state from deterministic non-entitled mocks. A retrieved snapshot is
neither an event claim nor invoice-settlement evidence. Hosts must serialize
reconciliation with their database update so a delayed earlier read cannot
overwrite a newer state; fetching before an unrelated transaction does not
establish event order. Account credential custody and durable intent remain
application responsibilities.

#### Provider-Specific Metered Usage

`MeteredBillingProvider` deliberately uses an associated request type rather
than pretending provider identities and retry semantics are interchangeable.
`StripeMeterEvent` targets the current `/v1/billing/meter_events` API with the
default `stripe_customer_id` and `value` payload mapping. It validates positive
integer usage, the meter event name, customer, timestamp window and a forwarded
identifier; the Stripe adapter sends that identifier as both event identity and
HTTP idempotency key. `LemonSqueezyUsageRecord` targets
`/v1/usage-records`, requires the numeric subscription-item relationship and an
explicit `increment` or `set` action that must match the configured aggregation.

Both adapters limit response JSON to one MiB, bind identity/quantity/action
fields before returning `UsageStatus::Accepted`, redact request/receipt
identities from `Debug`, and return deterministic `Mock` receipts for empty or
`mock_*` API keys. Stripe documents only a rolling provider deduplication
window. Lemon Squeezy's reviewed request has no equivalent event-key field, so
its receipt reports `ApplicationOutboxRequired`; the application must claim
`event_key` durably before submission. Provider-account acceptance, durable
outbox storage, retries, reconciliation and entitlement/quota policy remain
application/release evidence. The legacy uniform `BillingProvider::report_usage`
is compatibility-only and fails closed for live Stripe/Lemon Squeezy rather
than guessing the required provider-specific identity.

#### Coupons and Relative Trial Extensions

`CouponCode` accepts at most 256 ASCII identifier bytes and redacts its value
from `Debug`. The Stripe adapter uses `discounts[0][coupon]`, requests expanded
discount evidence and accepts only a response bound to both the subscription
and requested coupon. Lemon Squeezy documents discount codes for checkout, not
post-checkout subscription mutation; it and every adapter without a reviewed
live contract return `UnsupportedOperation` rather than a false success. Empty
or `mock_*` credentials retain the deterministic offline no-op required for
local applications and tests.

`TrialExtension` resolves 1 to 730 whole days against a trusted clock.
`Billable` and `SubscriptionHandle` expose the historical ergonomic
`extend_trial(15)` meaning plus `extend_trial_days_at` for a stable persisted
command clock and `set_trial_end` for explicit reconciliation. Stripe sends
`trial_end`; Lemon Squeezy sends `trial_ends_at` through its JSON:API PATCH.
Both cap provider responses and bind the returned subscription and expiration.
The host must authorize the subscription owner, persist a stable command time
before retry, serialize conflicting changes, reconcile signed webhooks and
evaluate provider-specific billing-cycle effects. Live-account acceptance is
release evidence, not inferred from protocol fixtures.

#### Server-side Plan Entitlements (v13 candidate)

The candidate `rullst-capital::entitlements` module supplies a read-time gate,
separate from usage quotas. `EntitlementScope` binds an authenticated tenant to
an existing `BillingSubject`; `EntitlementPolicy` binds one server-selected
feature to at most 64 exact plan IDs, an explicit live/sandbox mode and a
1–300 second maximum reconciliation age. Only active subscriptions qualify;
trial, past-due, canceled, unknown and mock states deny access. Validity has an
exclusive end, and both future observations and clock rollback during a read
are rejected. No serialized snapshot or browser field is an authorization token.

`EntitlementStore` is a static-dispatch trusted adapter contract. Every gate
call reads it again and checks the returned tenant/subject binding, provenance,
plan, status, expiration and trusted clock before and after the read. Store
failures deny access. Hosts must supply authoritative current state and serialize
reconciliation/revocation; this read gate does not make subsequent domain writes
atomic or revoke an operation already authorized. Custom adapters and database
backup/restore remain explicit application obligations.

The generated SaaS candidate consumes this gate at an authenticated billing
report route. Its single-tenant scope is the configured Stripe account/mode
inside the application's database, never a request header or submitted owner.
The existing durable billing intent and revision fence precede an actual
ownership-bound provider refresh; the state is committed before authorization.
It performs this refresh on every report, with a bounded deadline and no cached
grant. The exact report-plan allowlist is separately configured on the server.
Production requires live state; local sandbox state requires the explicit test
profile, and offline credentials produce deterministic denial. Mutable CMS
subscription projections, checkout redirects and unverified events cannot grant
report access. Other providers need reviewed reconciliation adapters before this
consumer can enable them. Existing profile/rights exports remain independent.

The local implementation and generated HTTP/database contracts exercise current
state, account/owner isolation, revocation races, expiry, provider/store failures,
mock denial and cancellation at the request deadline. Combined hosted admission
is pending; protocol fixtures do not establish live provider-account acceptance.

#### Shared Team and Workspace Quotas

`BillingSubject` identifies one authoritative user, team, workspace or trusted
tenant as the owner of both the subscription and its shared counters.
`Billable::quota_request` derives the limit from that owner's tier policy instead
of accepting it from an HTTP payload. A `QuotaStore` then performs an atomic,
idempotent reservation before the application creates the resource.

`InMemoryQuotaStore` is the deterministic offline/process-local contract. The
opt-in `quota-sql` feature supplies `SqlQuotaStore` for SQLite, PostgreSQL,
MySQL and MariaDB. Its conditional counter update and unique event claim prevent
concurrent members from exceeding the same limit. Exact retries return a replay
grant without consuming or executing again; a key reused with different units
or limit fails closed. `QuotaGate::execute` blocks the callback before an
over-limit creation and compensates an ordinary callback error.

The convenience gate cannot make two unrelated storage systems atomic. A
relational application that needs exact quota/resource atomicity must open a
transaction from `SqlQuotaStore::pool`, call `reserve_with_transaction`, perform
the domain insert through that transaction and commit once. Trusted middleware
must establish membership and active tenant before constructing the subject.
Plan/webhook reconciliation, migrations and custom/non-relational stores remain
explicit application work.

### 6.2. Invoice Rendering

`Invoice::generate_html` remains the source-compatible escaped HTML renderer.
Trusted paths use `validate`/`try_generate_html`: the legacy public `f64` model
accepts only bounded finite positive values with at most two decimal places,
converts them to integer minor units, and requires the exact item sum.

The opt-in `invoice-pdf` feature adds bounded paginated A4 rendering. Its
embedded Helvetica subset supports WinAnsi text; other scripts require a
caller-supplied TTF/OTF of at most eight MiB containing every used glyph. PDF
output is capped at sixteen MiB. `Invoice::bind_succeeded_charge` creates an
immutable `PaidInvoice` only when a final, non-mock receipt exactly matches the
recipient, minor-unit amount and currency. It derives a stable non-secret key
from the invoice and provider evidence for use by an application outbox.

The downstream `rullst-mail/capital-invoice` feature converts that value into a
pipeline-validated HTML message with the PDF attached and exposes one-call
facade, tenant-aware or static-driver delivery. It does not infer a webhook
event, atomically claim the delivery key, guarantee provider acceptance or
promise exactly-once delivery. Applications processing retries or multiple
instances must persist/claim the key and reconcile payment state durably before
sending.

### 6.3. Webhook Signature Verification
* InfinitePay's legacy body-only verifier has no reviewed authentication and
  authoritative payment-lookup contract for the documented checkout callback.
  Real-secret verification and normalization return `UnsupportedOperation`;
  explicit mock-secret fixtures remain offline-only. A local HMAC fixture is
  not evidence that the provider signs that protocol. Enabling live processing
  requires order/merchant/amount binding and provider reconciliation first.
* The additive `StripeProvider::verify_subscription_event` returns an immutable
  `StripeSubscriptionEvent` after the existing signature/freshness check and
  bounded subscription normalization. It retains event ID/type/API version,
  creation time, matching event/subscription test-live mode, optional connected
  account and opaque local owner reference, plus the original provider status.
  A versioned mutation digest excludes contact email, delivery signature and
  unrelated JSON fields; a separate digest binds the exact raw payload.
  Neither digest is encryption or a persisted replay claim. Mock verification
  is explicit in the result and `require_real` rejects it. The host must bind
  the endpoint's provider account, mode, customer and owner, and atomically
  commit the event claim with domain writes. Event creation time is not a
  complete ordering/reconciliation protocol. The legacy `WebhookEvent` and
  `BillingProvider` contracts are unchanged.
* Stripe subscription normalization requires an explicit supported lifecycle
  event and subscription object with bounded subscription/customer/price IDs.
  Its single-price v12 contract rejects multiple or truncated item lists and
  statuses outside Stripe's subscription vocabulary. `incomplete` and
  `incomplete_expired` map to the legacy non-entitled `Unpaid` status; they must
  not be interpreted as proof of an unpaid invoice. Billing-period end is read
  from the single item for Basil payloads, or from the subscription on older
  payloads; conflicting period values are rejected. Email remains optional
  contact data. Event identity, ordering and atomic inbox processing remain
  separate requirements; an active subscription is not settlement evidence.
* Razorpay subscription normalization requires its own subscription/customer/
  plan identities and agreement between the event and entity state. Authentication
  and standalone payment/order events cannot establish an active subscription.
  Email is optional contact data; durable owner binding, event ordering and
  reconciliation remain application responsibilities. Lifecycle activation is
  not a receipt proving settlement of an invoice.
* Lemon Squeezy normalization accepts only explicit subscription lifecycle
  events containing a `subscriptions` object, positive numeric identities and
  a valid provider state. It binds the store when `with_store_id` is configured
  and rejects conflicting test-mode fields. `on_trial` maps to `Trialing`;
  `cancelled` and `expired` map to `Canceled` with a required valid `ends_at`.
  Cancellation retains its grace-period timestamp; host policy decides access.
  Invoice/payment/refund events require separate handling and cannot masquerade
  as subscription snapshots. This legacy event does not retain account/mode or
  causal identity, so durable owner/scope binding and reconciliation remain
  application responsibilities.
* The Axum and opt-in Actix middleware adapters call one canonical bounded
  verifier before dispatch. Built-in provider adapters use provider-appropriate
  cryptographic verification; equality checks for derived signatures are
  constant-time where applicable.
* Timestamped protocols enforce a bounded freshness window. The default replay
  store is bounded and process-local and fails closed instead of evicting an
  unexpired proof when full.
* The opt-in `webhook-sql` store shares bounded payload-digest or semantic-event
  claims across processes on SQLite, PostgreSQL, MySQL, and MariaDB. Its schema
  profile is immutable, claims serialize through one configuration lock, expiry
  uses the database transaction clock, and storage/configuration/capacity
  failures reject the request.
* SQL-backed middleware admission claims a payload before handler dispatch; it
  is replay protection, not an exactly-once delivery protocol. When billing
  correctness requires atomic domain mutation, the application must verify the
  provider payload, select the provider's stable event ID, and call
  `check_and_record_event_key_with_transaction` through the same relational
  transaction as the mutation. Cross-system effects still require an outbox,
  idempotent consumers, and reconciliation.
* The additive `SqlStripeEventInbox` under `webhook-sql` owns the relational
  transaction for a verified subscription event and its caller-supplied SQL
  mutation. An immutable scope binds application namespace, configured Stripe
  account, endpoint kind and test/live mode. Mock or mismatched events fail
  before database access. A per-scope configuration lock serializes admission;
  the stable event ID and versioned mutation digest distinguish an exact retry
  from conflicting content. A committed retry returns the retained outcome
  without invoking the mutation again. Domain errors and cancellation before
  commit roll back both changes; uncertain commit requires replaying the same
  event to discover the retained outcome. Capacity is immutable and bounded;
  records are never automatically evicted. The host owns schema migration,
  retention/reconciliation, account credential custody, authorized customer
  binding, ordering and an outbox for external effects. No SQL transaction can
  undo HTTP or other external effects performed by the callback. This API does
  not migrate the generated handler or grant access from subscription status.
* Generated SQLx and Turso billing persists an opaque authenticated-owner binding,
  immutable customer/checkout intents and session IDs before redirect. Account
  identity is checked through Stripe and test/live namespaces are separate.
  Customer email never discovers or authorizes ownership. Real Stripe credentials
  require an explicit account, HTTPS return URL, webhook secret and price allowlist;
  mixed credentials and other generated live providers remain unavailable.
  Customer and session creation reuse persisted keys for at most 23 hours. Older
  unknown outcomes use bounded read-only recovery; absence never permits another
  mutation. Completed and expired Checkout notifications and subscription lifecycle
  events trigger ownership-bound provider reads. A random database revision is
  committed before reading and compared atomically when committing the resulting
  subscription projection and durable event receipt. Concurrent newer reads fence
  earlier work; failed/uncertain commits remain retryable. An open checkout is
  retrieved and reused, and rejected/repeated requests do not consume a newly
  created-session quota. Existing subscriptions use an ID-bound customer portal.
  Provider-portal price changes must remain in the application's allowed catalog.
  The application owns entitlement/settlement policy, inbox retention and capacity
  operations; a subscription state is not invoice settlement evidence.
  Existing application-owned code requires merging the generated modules and an
  additive migration; updating the package does not rewrite deployed controllers.
* Hosted checkout forms require an explicit provider-specific CSP `form-action`
  origin on the document that submits the form, including its HTTP 303 handoff.
  The SaaS starter selects Stripe and adds only `https://checkout.stripe.com`
  to its generated policy; Core's default remains `form-action 'self'`.
  Changing providers requires reviewing that exact checkout origin, including
  any merchant/custom domain. `make:billing` advises this integration without
  overwriting an existing application's policy. This is a browser policy
  boundary, not provider URL authentication: real flows must independently
  validate HTTPS, exact host/port, absence of credentials and durable session,
  customer, tenant, product and test/live bindings before emitting a redirect.
  A real-browser POST/303 positive and disallowed-origin negative are required;
  HTTP-client status checks alone cannot establish this behavior.
* Before the future generated live flow creates a checkout, resolve the
  authenticated owner's persisted attempt. Resume only a provider-retrieved
  open session with all the bindings above; completed, expired and uncertain
  attempts need distinct handling. A business new-session quota applies only
  to new attempts, with a useful `Retry-After` on rejection; a separate request
  abuse limit may still protect retrieval. Concurrent clicks must converge on
  one durable intent/idempotency key. A disabled submit button is only feedback,
  never the duplicate-payment or ownership boundary.

### 6.4. NFS-e Nacional Specification (`FiscalEngine`)
* 🟢 **`[Implemented / Bounded]` DPS 1.01 Builder:** `NfseDpsV101` models an ordinary domestic-service subset, validates CPF/CNPJ/IBGE/identifier/text limits, keeps BRL values in integer cents and ISS rates in basis points, and emits an unsigned DPS in the official namespace. The legacy floating-point preview remains compatibility-only.
* 🟢 **`[Implemented / Bounded]` Pinned Schema Validation:** Production profile `v1.01-20260209` and restricted profile `v1.01-20260727` carry immutable archive/file SHA-256 values. `NfseDpsSchemaValidator` reads only the expected bounded files and resolves imports from an in-memory catalogue; it never downloads schemas or follows instance hints.
* 🟢 **`[Implemented / Bounded]` Local XMLDSig and mTLS Preparation:** `sign_dps_xml` parses a protected PKCS#12 A1 container, rejects malformed/duplicate/already-signed envelopes and emits an enveloped inclusive-C14N 1.0 RSA-SHA256 signature over the unique `infDPS/@Id`. The matching certificate chain is embedded and tested with independent local verification. The same container can construct a rustls mTLS identity/client with HTTPS-only, no redirects, and bounded timeouts.
* 🟢 **`[Implemented / Bounded]` Offline SEFIN Issuance Codec:** `NfseIssueRequest` accepts only one structurally bound and cryptographically valid embedded DPS XMLDSig, emits deterministic GZip/Base64 inside the exact `dpsXmlGZipB64` JSON object, and parses at most four MiB. HTTP 201 can become `Authorized` only when environment, submitted DPS ID, 50-digit access key, `infNFSe/@Id` and the embedded NFS-e XMLDSig agree; HTTP 400/403/500 become a separate bounded `Rejected` variant. Unknown fields, malformed JSON/XML/Base64/GZip, duplicate/confused IDs, invalid signatures and decompression amplification fail closed. Embedded-signature validity does not establish ICP-Brasil trust or emitter ownership.
* 🟢 **`[Implemented / Bounded]` Local Fiscal Command Journal:** The `nfse` feature exposes a single-active-writer `FiscalCommandJournal` that accepts only a homologation/production command whose selected environment equals the signed `infDPS/tpAmb`. It synchronously records a prepared command before any caller-owned transport and then one bound authorized or rejected terminal result. Exact command/request/result replays do not append; key reuse with different material, invalid transitions, external file growth, quota exhaustion, wrong keys, symlinks, corruption and durability uncertainty fail closed. The append-only v1 file is bounded to 16 MiB and 4,096 events, uses a named 256-bit HMAC key and chains every frame to the prior tag. It stores the caller's opaque command ID, request/result digests, environment, state and bounded times, never the DPS/NFS-e XML, access key, certificate, response body or processing messages. `pending()` recovers minimized unresolved descriptors after restart. A serializable exact-tip checkpoint can detect valid-prefix truncation only when retained independently. The host owns a non-PII command namespace, key custody/rotation, a trusted directory, one active writer, secure storage of the actual request, checkpoint persistence, backup/retention, authority reconciliation and retry policy; this journal does not transmit, retry, prove cross-system exactly-once or establish tax authorization.
* 🟡 **`[Simulado]` Offline Mock Environment:** `NfseEnvironment::Mock` produces deterministic test fixtures for local sandboxing.
* 🔵 **`[Roadmap / External Evidence]` Official SEFIN Homologation & Production:** `Homologation` and `Production` validate credentials and then return `FiscalError::Unsupported` without network I/O. Enabling transmission requires emitter-certificate/ICP-Brasil lifecycle checks, deployment of the local journal plus authoritative request/outbox and reconciliation storage, retained protocol fixtures, real restricted-environment tests with an authorized contributor and municipality, independent review, and successful official homologation.

---

## 🛡️ 7. Enterprise Security, RASP & Vault (`rullst-security`)

### 7.1. Rullst Vault (Authenticated Field Encryption)
* **Algorithm:** AES-256-GCM with authenticated 96-bit random nonces and 128-bit authentication tags.
* **Envelope Format:** `RULLST:v2:<key_id>:<base64_nonce>:<base64_ciphertext_and_tag>`.
* **Key Rotation:** Built-in keyring support (`decrypt_with_keyring`) can read
  prior keys while new writes use the active key. Deployment coordination,
  re-encryption, key custody and retirement remain operator responsibilities.
* **ORM Configuration:** `RULLST_ENCRYPTION_KEY`, `RULLST_ENCRYPTION_KEY_ID`, and `RULLST_ENCRYPTION_KEYRING` select the current and still-readable prior keys. Rullst does not provide key custody or automatic retirement.

### 7.2. Runtime Application Self-Protection (RASP)
* **Bounded Heuristic Inspector:** ASCII case-insensitive signature matching covers selected SQL injection, traversal, SSRF, shell/JNDI patterns across URI, non-secret headers, and supported bounded textual/JSON bodies. Percent decoding and body/JSON inspection allocate; this control does not replace typed parsing, SQL binds, validation, authorization, or SSRF allowlists.
* **Login Guard Tarpit:** `record_login_failure` returns progressive delay
  decisions and `record_login_failure_and_wait` applies them asynchronously;
  both share bounded, temporary in-memory jails keyed by a hashed identity.

### 7.3. MFA and Security Evidence Boundaries
* **TOTP enrollment:** Secrets contain 160 bits derived from the OS RNG,
  verification accepts exactly six ASCII digits with constant-time comparison,
  and enrollment can emit an `otpauth://` URI or bounded SVG QR. Secret custody,
  recovery workflow and durable rate limiting belong to the application.
* **Security CLI:** CycloneDX generation, MSRV/tool diagnostics, unsafe/IDOR
  source heuristics, network observations and compliance evidence are bounded
  checks. They do not certify a deployment, prove absence of vulnerabilities or
  replace provider/CI evidence tied to an immutable SHA.

### 7.4. Bounded JSON Schema Enforcement
* `JsonSchemaPolicy::from_schema` compiles an application-supplied JSON Schema
  2020-12 document once; `from_openapi_component` selects one explicit
  `components.schemas` entry from OpenAPI 3.1. OpenAPI 3.0 is rejected because
  it is not the same schema dialect.
* Construction caps serialized bytes, node count and depth, rejects non-local
  `$ref`/`$dynamicRef`, disables network/filesystem retrieval and selects the
  linear-time regex engine. The route-scoped Axum middleware first enforces the
  existing exact media-type, syntax, duplicate-key, payload-size and depth
  boundary, then returns `422` for schema mismatch without echoing values.
* The policy validates JSON bodies only. Authentication, authorization,
  ownership, business invariants and query/header/form parameters remain
  separate application boundaries.

### 7.5. Deterministic Threat Sentinel and Proof of Work
* `ThreatClassifier` assesses a bounded aggregate window supplied by the host
  against transparent thresholds for credential stuffing, API scraping and
  distributed automation. It does not collect traffic, infer identity, use a
  model or attribute a botnet.
* `ProofOfWorkGate` issues OS-random, HMAC-authenticated challenges bound to one
  canonical application subject. Tokens have bounded difficulty, TTL and
  cardinality; successful verification atomically consumes local state so only
  one concurrent verifier succeeds in the process.
* Classification is evidence, not authorization. The host chooses whether and
  where to challenge, provides an accessible fallback, rate-limits issuance and
  owns trusted proxy/device policy. Replay state is process-local; distributed
  enforcement, durable telemetry and cross-process one-shot consumption require
  an application adapter.

### 7.6. Authenticated Local SIEM Journal
* `AuthenticatedSiemSpool` is the opt-in authenticated counterpart to the
  compatible unsigned `DurableSiemSpool`. It normalizes each local v1 event,
  writes it synchronously under 16 MiB/4,096-record ceilings and authenticates
  sequence, key identifier, predecessor tag, payload length and exact payload
  with a domain-separated HMAC-SHA256 chain.
* `SiemKeyRing` accepts one active write key and at most seven historical
  verification keys. Key identifiers are bounded, key material is consumed
  into zeroizing storage, and secret-bearing `Debug` output is redacted. A
  rotation reopens the spool with the new active key plus every still-needed
  historical key; silently missing or wrong keys fail closed.
* This journal detects forged records, interior deletion/reordering and
  external length changes. Removal of a complete valid tail is indistinguishable
  from an earlier valid file unless the operator retains a trusted external
  checkpoint. Path/key custody, permissions, single-writer enforcement,
  rotation retirement, compaction, retention, backup, transport, retry,
  acknowledgement and dead-letter handling remain operator/application work.

---

## 📡 8. IoT, Firmware Security & Protocol Frames (`rullst-iot`)

### 8.1. Ed25519 OTA Firmware Gate
* **Firmware Verification:** Strict Ed25519 signature validation over a cryptographic manifest `[target, version, rollback_counter, firmware_len, firmware_sha256]`.
* **Anti-Rollback Protection:** Verification rejects any counter lower than or equal to the state loaded into the manager. The recommended `RollbackCounterStore` path additionally performs an exact compare-and-set and may report success only after a strictly increasing value is durably committed across reset. Atomicity, integrity, wear-leveling and power-loss behavior are obligations of the caller's platform adapter and require hardware-specific evidence.
* **Commit Invariant:** In-memory partition selection and store-backed counter commit are blocked until full cryptographic verification succeeds. `verified_target_partition` exposes the inactive bank for platform flash/read-back before commit. The compatibility `commit_verified_update` path is process-local and does not claim persistence, flash or bootloader control.

### 8.2. Embedded Sensor Frames (`#![no_std]`)
* `rullst-iot` core models compile under bare-metal `#![no_std]` targets (STM32, ESP32-C3, Cortex-M).
* `cargo rullst make:iot <DeviceName>` generates and registers a local telemetry
  module, enables the umbrella `iot` feature and refuses unsafe names or
  collisions. It does not install firmware, a HAL, MQTT or CoAP.
* `IotDashboard` renders an escaped HTML snapshot. It does not infer online
  state or provide a live device connection.
* `MqttPublish` encodes one bounded MQTT 5 PUBLISH packet with validated topic,
  minimal Remaining Length, QoS/packet-identifier invariants and an empty
  property section. `CoapRequest` encodes bounded RFC 7252 base requests with
  a token, ordered URI-Path/Content-Format options and a non-empty payload
  marker. Both compile under `no_std`; neither opens a socket or owns protocol
  session state.
* 🔵 **`[Roadmap]` MQTT/CoAP Transport:** Async connections, TLS/DTLS, broker
  negotiation, acknowledgements, retransmission/congestion control, block-wise
  transfer and interoperability matrices remain separate integration work.

---

## 🤖 9. AI Agent & LLM Orchestration (`rullst-ai`)

### 9.1. Guarded AI Client
* `AiClient::auto()` and Nexus share `AutoAiConfig`. Resolution uses one
  snapshot in OpenAI/custom, Anthropic, Gemini, DeepSeek, Groq, Ollama order.
  Empty environment values are absent; explicit mocks remain offline. Groq
  requires `GROQ_API_KEY` plus `GROQ_MODEL`; a custom `OPENAI_BASE_URL` requires
  `OPENAI_API_KEY` plus `OPENAI_MODEL` and uses the HTTPS chat-only compatible
  adapter. Configuration indicators do not attest network/account health.
* Provider-agnostic interface for **Google Gemini, OpenAI, Anthropic Claude,
  DeepSeek, Ollama, and explicit OpenAI-compatible endpoints**. The compatible
  adapter is chat-only by default; applications declare optional request shapes
  for one exact model. Loopback may be unauthenticated, cloud requires
  HTTPS/Bearer, and unrelated protocols implement the public `AiProvider`
  boundary rather than passing through arbitrary HTTP.
* **Prompt Injection Firewall:** Real-time token heuristics intercepting prompt exfiltration, instruction overrides (`DAN mode`), and delimiter injection attacks.
* **Automated PII Masking:** Scrubs sensitive data (CPF/CNPJ, credit cards, emails) prior to outbound LLM dispatch.

### 9.2. Bounded Streaming and Cancellation
* `StreamingAiClient<P>` preserves static dispatch, reapplies the mandatory
  input guardrails and enforces at most 4,096 non-empty chunks, 64 KiB per
  chunk and 2 MiB aggregate output independently of the provider.
* An OpenAI-compatible configuration may explicitly declare SSE streaming. The
  transport requires `text/event-stream`, bounded raw bytes, supported chat
  deltas and `[DONE]`; malformed, truncated or oversized streams fail closed.
* `AiCancellation` races the initial request and every streamed body read. It
  drops local transport work but does not prove upstream cancellation or stop
  provider billing. Non-compatible protocols and ordinary non-streaming calls
  retain deadline/drop semantics.

### 9.3. Bounded Tenant-Aware RAG
* `RagPipeline::answer` requires a trusted Core `TenantContext` and composes
  guarded embedding, a static-dispatch application `RagRetriever`, bounded
  context selection, guarded generation, source metadata, and one required
  terminal `RagAuditSink` event.
* Retrieved documents carry the trusted tenant tag. The pipeline rejects
  mismatches, over-return, injection heuristics, empty context, non-finite
  embeddings, and unavailable mandatory audit evidence rather than silently
  generating an ungrounded response.
* Context limits count Unicode scalar values per document and in total. The
  audit event omits raw question, context, embeddings, provider bodies, and
  answer; its SHA-256 query digest is correlation metadata, not encryption.
* `InMemoryRagRetriever` and `InMemoryRagAuditTrail` are bounded process-local
  development/test implementations. `DurableRagAuditTrail` and
  `DurableToolAuditTrail` add bounded, synchronously persisted, versioned local
  evidence with restart validation and fail-closed corruption/quota handling.
  They are single-process writers; their SHA-256 frames detect corruption but
  do not authenticate events. Production hosts own authoritative tenant and
  ownership predicates, durable/external vector adapters, ingestion/deletion,
  model/vector compatibility, output policy, directory permissions,
  rotation/retention/backup, external audit delivery, tuning, evaluation and
  recovery.

### 9.4. Bounded Conversational Memory
* `StatefulChat<M>` uses static dispatch over `ChatMemory`, requires a trusted
  `TenantContext` and validated `ConversationId`, loads only the configured even
  number of recent messages, applies the guarded `AiClient`, and persists the
  user/assistant exchange only after generation succeeds.
* `InMemoryChatMemory` is deterministic, tenant-partitioned, cardinality-bound,
  and intended for tests/local use. The opt-in `sql-memory` adapter supports
  SQLite, PostgreSQL, MySQL, and MariaDB through a dedicated SQLx Any pool.
* The SQL adapter advances an even conversation revision and inserts both
  messages in the same transaction. A compare-and-swap predicate rejects stale
  cross-process writers; Rullst deliberately does not retry the provider call.
  History reads bind the tenant/conversation and never include rows newer than
  the revision observed by that read.
* Message text is not encrypted by this adapter. Authenticated conversation
  ownership within a tenant, retention/erasure, provider audit, backups,
  migration governance, and user-facing conflict retry remain host policy. The
  generated Turso/custom-model scaffold is a separate application-owned path.

### 9.5. Authenticated Audit Export
* `AuditDeliveryClient` exports an application-minimized serializable event in
  a versioned JSON envelope of at most 16 KiB. Cloud endpoints require HTTPS;
  local HTTP(S) requires a literal loopback IP. Redirects and ambient proxies
  are disabled.
* HMAC-SHA256 covers a domain separator, key ID, Unix-millisecond timestamp and
  the exact body. A caller-supplied event ID remains unchanged across one to
  five attempts. Only transport/deadline errors, HTTP 429 and HTTP 5xx are
  retryable, and a closed acknowledgement of at most 8 KiB must bind that ID.
  `AiCancellation` races request, body and retry waits. Empty or `mock_*` keys
  select deterministic offline behavior.
* A timeout may follow remote acceptance. The receiver therefore owns
  signature and freshness verification, event-ID deduplication, authorization,
  persistence, retention, key distribution/rotation and operational
  availability. The client does not minimize arbitrary event values or provide
  a durable outbox/SIEM service.

### 9.6. Bounded Adaptive Evaluation
* `AdaptiveAiEvaluator<P>` keeps static provider dispatch and reapplies the
  mandatory prompt guardrail on every strategy-generated prompt. A scenario
  is capped at 32 turns, 16 KiB per prompt and 2 MiB per response, with an
  independent per-turn deadline and `AiCancellation` raced against each call.
* `AiEvaluationStrategy` temporarily receives the bounded response or a
  low-cardinality guardrail/provider/deadline outcome and explicitly chooses
  pass, fail, inconclusive or a next prompt. The synchronous strategy is
  application code and must not persist/log model text without policy.
* The version-1 JSON report records caller-supplied suite and exact
  model/configuration subject labels, provider name, terminal code and only
  per-turn byte counts/outcomes. It retains no prompt, response or provider
  error. Subject labels are assertions, not automatic model discovery.
* Deterministic repository fixtures prove bounds, feedback, redaction,
  cancellation and status semantics. Operators still version domain corpora,
  run them against every exact live model/configuration and review results; a
  pass is not universal safety, groundedness or jailbreak-resistance evidence.

---

## 📊 10. Control Center & Admin Interfaces (`rullst-studio` & `rullst-nexus`)

### 10.1. Rullst Studio (`http://127.0.0.1:5555`)
* Local-first developer dashboard with a server-rendered dark interface; browser
  assets and final page policy remain deployment concerns.
* Process observations sourced from `RadarSnapshot::collect()` and explicitly
  supplied local collectors; unsupported values remain unavailable.
* Generated applications start the standalone Studio only in debug builds and
  bind it to loopback. Its local capability verifies the direct loopback peer,
  accepts only a local `Host` authority, requires same-origin `Origin` on unsafe
  methods, and rejects missing origins on mutations. This is a local
  DNS-rebinding/CSRF boundary, not production authentication.
* Queue, revenue, security and telemetry pages report only values supplied by
  their configured process-local source. Unsupported driver operations and
  disconnected integrations remain errors or `Unavailable`. The standalone
  migration surface provides CLI guidance and returns `501` from legacy
  mutation handlers because no migration/seeder registry is installed.
* The database browser accepts a deliberately narrow ASCII SQL-identifier
  boundary. Reads are bounded; writes require the crate-private proof inserted
  by the verified local middleware, database-inspected table/column/complete-PK
  metadata, a 64 KiB request limit, primitive typed binds and exactly one
  affected row. Primary keys/backend-specific values are read-only, while
  delete requires `DELETE <table>`. SQLite, PostgreSQL, MySQL and MariaDB run
  separate mutation contracts. This is not application authorization, tenant
  scoping, audit, rollback or shared-production administration. The ER diagram
  inspects the same relational backends with bound lookup values and strict
  normalized Mermaid identifiers. Swagger requires an application-supplied
  `OpenApi`.
* Request SSE records method, URI, status, and latency without bodies or headers.
  Environment values are redacted by default and the typed config projection
  never renders connection URLs, filesystem paths, cookies, tokens, or
  credentials. A successful Studio database-flag mutation invalidates warm
  `DbFeatureDriver` caches in the same process; direct writers and other
  processes remain subject to TTL unless the host distributes invalidation.
  SQLite removes completed jobs by default. An explicit 1–100,000-row history
  policy retains and atomically prunes real completion records for Studio, with
  a separate purge; retained payload access and lifecycle belong to the host.
  Redis/custom queue inspection remains capability-specific.
* `Studio::with_cache` is an explicit metadata-only diagnostic capability. The
  memory and Redis cache drivers return at most 200 sorted entries containing
  logical key, UTF-8 value byte length and remaining TTL; custom drivers return
  `InspectionUnsupported` by default. Studio displays at most 100 keyed opaque
  identifiers and never cache values or exact logical keys. Individual
  invalidation requires its unforgeable verified-local marker and a fresh
  process-bound HMAC token. The page has no bulk flush operation.
* Distributed trace producers use separately mounted, push-only routers; each
  endpoint binds one exact producer name to one key and multiple producers use
  separate endpoints/keys over the same shared store. A v1
  batch is at most 128 KiB and 128 spans, carries W3C-compatible IDs, a bounded
  source/operation/kind/timing/status set, and accepts no attributes, SQL,
  bindings, headers, bodies or error strings. HMAC-SHA256 authenticates the
  exact body and source/timestamp/nonce headers; a 60-second window and bounded
  atomic nonce cache reject replay, while the shared in-process store is
  capacity-bound and idempotent by trace/span ID. The local viewer reports a
  fixed 100 ms slow-query signal and a three-equal-label possible-N+1
  heuristic. This is not an OTLP collector, durable backend, secret manager,
  remote viewer/login, or proof that a repeated operation is defective. TLS,
  network policy, key distribution/rotation, clock synchronization, producer
  label redaction, retention and availability belong to the deployment.
* Exposing Studio beyond the developer machine requires an explicit
  authenticated network boundary owned by the application; no environment
  variable silently converts the local server into a production admin surface.

### 10.2. Rullst Nexus (`/nexus`)
* Auto-generated CMS with dynamic CRUD operations and AI Admin Assistant.
* **Security Default:** Fail-closed by design; requires explicit authentication middleware and RBAC role validation (`admin`) on all mutating endpoints.
* Generated applications may use `NexusAuthPolicy::local_development_or_basic_from_env()`: debug builds accept only a peer address verified as loopback through `ConnectInfo`, while release builds require validated Basic Auth credentials from the environment. Missing peer metadata is denied, and an environment mode flag cannot enable unauthenticated release access.
* A model may explicitly declare one text `tenant` column. Nexus then obtains the scope only from a trusted Core `TenantContext`, injects it on create, includes it in every built-in read/mutation/batch predicate, and denies a missing context. Models without that metadata remain global administrator models. Authentication and tenant-membership resolution remain host contracts.
* `with_required_audit` requires the fixed `rullst_nexus_audits` schema and appends one minimized committed-mutation row in the same transaction. Audit unavailability rolls the data change back. This is not append-only, tamper-evident, denied-attempt, retention, backup, replication or external-SIEM evidence; those properties remain host responsibilities.

---

## 🛡️ 11. Architectural Guidelines for Backward Compatibility

1. **`#[non_exhaustive]` on Public Structs:** All configuration structs and enums must use `#[non_exhaustive]` to ensure minor versions can add fields without breaking downstream code.
2. **Deprecation Policy (`#[deprecated]`):** Public APIs will never be removed without at least one minor release cycle marked with `#[deprecated]`.
3. **Ergonomic String Constructors:** Public constructors accept `impl Into<String>` to support both `&str` literals and owned `String` parameters without boilerplate.
4. **Zero-Panic Invariant:** Production paths must never call `panic!()`, `unwrap()`, or `expect()`; domain errors must return typed `Result<T, AppError>`.

---

### 11.1. Generated Project Context (v13 candidate)

`generate:ai-context` retains its existing helper signature but replaces raw
source/configuration embedding with the `rullst.project-context.v1` inventory.
It writes a readable `.llms.txt` and machine-readable `.rullst/context-map.json`,
and creates a project `AGENTS.md` only when no user instructions already exist.
New project generation installs the same instructions and inventory. Existing
`AGENTS.md` files are never rewritten by regeneration or automatic scaffold hooks.

The map contains bounded project/dependency/feature names, validated dependency
version requirements, configuration key
names, source paths and roles, file sizes and one deterministic input fingerprint.
It contains no source bodies, configuration values, dependency URLs or absolute
workstation paths. Only bounded `Cargo.toml`, optional `Rullst.toml` and
`.env.example`, and regular `.rs` files beneath `src` are inspected. Secrets,
databases, VCS metadata, build output, dependency directories, hidden descendants
and symlinks are excluded or rejected explicitly. Directory depth, entries,
individual files, aggregate reads and output size all have hard limits; errors
must not echo configuration values. Generated instructions identify the map as
an inventory, not evidence that a feature, deployment or test has passed.
The dotenv inventory accepts single-line declarations and rejects multiline
values before they can be mistaken for key names; it performs no interpolation.
The selected root's `src` is explicit: external workspace members are not scanned.

`--check` recomputes and compares the generated inventory without writing files;
stale, missing or altered output fails. Normal regeneration uses preflighted
atomic replacement/rollback, refuses links and unrecognized output, and retains
the legacy generated `.llms.txt` migration boundary explicitly. The fingerprint
detects changed Rust source and inventoried metadata, excluding configuration
values and dependency URLs; it is not a signature or an authorization claim. The
workspace directory is trusted against concurrent adversarial filesystem edits.

Local unit and real CLI/new-project contracts passed for the bounded inventory.
Hosted and installed-archive source acceptance passed in PR #221, as recorded
in the [delivery plan](v13-delivery-plan.md); final release admission remains separate.

### 11.2. Schema-First API Profile (v13 candidate)

`generate:api --schema <file> --output <directory>` is an opt-in generator with
one explicit OpenAPI 3.1 JSON source. It must reject unsupported keywords and
shapes, external/cyclic references, duplicate keys, ambiguous names/routes and
unbounded input before writing anything. The existing route-scanning generators
remain discovery aids; their placeholder schemas do not enter this typed profile.

The initial profile admits closed named objects, bounded strings/arrays,
booleans, JavaScript-safe integers, required nullable scalar/array fields and
optional non-nullable fields. Optional nullable fields, arbitrary maps,
polymorphism, floating-point/64-bit numeric wire values, formats and recursive
schemas remain unsupported. References are local named object components.
Parameters are explicit path strings and scalar query values; request and
response bodies are named JSON objects with explicit status codes. Bearer
authentication is described explicitly, while identity, ownership, CSRF and
authorization remain host application responsibilities.

Generated Rust DTOs use Serde, and explicit operation codecs reuse Security's
bounded duplicate-key inspection and offline JSON Schema policies. They validate
input before deserialization and output before serialization. Generated
TypeScript uses strict types, runtime validation and typed status/body unions;
its HTTP client bounds response bytes/time, encodes parameters, refuses redirects
and never includes tokens or rejected payloads in errors. No route is silently
mounted and no schema declaration constitutes authorization.

Generation preserves unrelated files, preflights recognized outputs and exposes
a read-only freshness check. Hard budgets apply to source/output bytes, schemas,
operations, properties, parameters and reference depth. The trusted project
directory is not an adversarial concurrent filesystem boundary. Acceptance
requires compiled generated Rust and strict TypeScript, plus a real HTTP consumer
journey covering Unicode, missing versus null, bounds, typed errors and denied
cross-owner access. Local generation, compiled Rust/strict TypeScript and HTTP
consumer contracts passed, including read-only APIs, exact safe-integer bounds,
nullable nested arrays, duplicate queries/JSON keys and cancellation. The initial
implementation passed hosted and installed-archive source acceptance in PR #221;
final release admission remains separate. See the
[supported profile](typed-api.md) for exact limits and application wiring.

## 🔄 12. Assisted Framework Upgrade Contract

`cargo rullst upgrade` is the canonical application-upgrade boundary. It is an
assistant, not a claim that compilation proves production compatibility.

* 🟢 **`[Implemented / Bounded]` Planning:** `--dry-run` enumerates only Cargo
  workspace manifests, preserves TOML comments/order, understands normal,
  inline-table, workspace, target-specific and renamed Rullst dependencies, and
  reports path/git dependencies that have no version instead of guessing.
  `--dry-run --json` emits the versioned `rullst.upgrade-plan.v1` envelope.
* 🟢 **`[Implemented / Bounded]` Versioned Rules:** source findings are selected
  from a versioned rule catalog using detected source majors and the exact
  target major. Every future major release must extend that catalog, migration
  documentation, negative tests and process-level fixtures for its supported
  upgrade paths.
* 🟢 **`[Implemented / Bounded]` Transaction:** the default target is the exact
  installed `cargo-rullst` version; `--to` accepts only the same major train as
  that CLI. Before writes, the command snapshots workspace manifests, the root
  lockfile and Rust sources under `target/rullst-upgrades`. It applies only
  dependency edits and compiler-provided `cargo fix` changes, then requires
  `cargo check --workspace --all-targets --locked` to pass. Managed version
  requirements are exact `=VERSION` pins, so an explicitly selected release
  cannot silently resolve a later patch/minor release. `cargo fix` resolves
  the candidate lockfile; the final check must use that same resolution.
  A failed gate restores the
  snapshot by default; `--keep-on-failure` is explicit, and `--restore` can
  recover a persisted, path-validated snapshot after an interruption.
  Process fixtures independently select the v5, v6 and v11 rule sets, prove
  restoration across multiple workspace members, preserve a failed edit
  only when explicitly requested, restore that persisted review state, and
  reject symlinked Rust sources before starting the transaction.
  **12.1.0 recovery hardening:** automatic and persisted restores validate
  the complete bounded index and every snapshot/target before staging all file
  replacements. Limits are 8 MiB/index, 100,000 entries, 64 MiB/file and
  512 MiB/restore. Symlinks/reparse points, malformed or duplicate entries and
  non-regular files are rejected. Per-file replacement does not truncate a
  hardlinked destination. A later apply error may leave earlier files restored;
  it reports progress and retains the backup. Stop other writers first: this is
  neither an all-files atomic commit nor protection from hostile concurrent
  filesystem changes. Platform acceptance is scoped to the
  [published release evidence](v12.md#1210-published-maintenance-release).
* 🟠 **`[Manual Application Boundary]`** the command never installs a CLI,
  changes secrets, executes database migrations, invents authorization or
  tenant policy, exposes Nexus/Studio, validates providers, or declares an
  application production-ready. Database restore/migration/rollback, the full
  test suite, authorization negatives and deployment smoke tests remain
  mandatory human-owned gates.

  **12.1.0 advisory discovery:** the
  CLI checks for notices only on interactive dashboard startup, respects
  explicit offline/CI/notification-disable flags, and retains at most one
  validated result in process memory. It no longer reads or writes the legacy
  shared temporary cache. Discovery uses a fixed HTTPS registry endpoint,
  denies redirects, caps the response at 256 KiB and applies one four-second
  network/body deadline. Notices select a newer, non-yanked stable version in
  the installed major, not an arbitrary registry maximum. They authorize no
  installation or project changes. The explicit `cargo rullst update check`
  command now reports an exact eligible CLI release, its declared MSRV and the
  current OS/architecture. `--to` pins selection; another major requires
  `--allow-major`, and a prerelease separately requires `--prerelease`.
  `--json` emits `rullst.update-discovery.v1` with every execution/write/artifact
  authority false. It does not certify compiler/platform compatibility.
  Explicit discovery now reuses a six-hour advisory cache on Unix platforms.
  The caller-owned cache base and its ancestors are checked before using a
  private `0700` directory; regular single-link `0600` files, bounded reads,
  no-follow opens, a non-blocking writer lock and staged atomic replacement
  protect the cache. Cached catalogs are parsed and selected again, never
  accepted as artifact/installation authority. `--offline` never requests the
  network or writes; missing, invalid, expired or future-dated caches fail.
  `--refresh` skips cached reads; `--no-cache` disables persistence entirely.
  Ordinary interactive notices remain process-local, not filesystem writers.
  The Windows implementation uses `%LOCALAPPDATA%/rullst-update-v1`, a protected
  DACL created atomically for the caller, SYSTEM and Administrators, handle-based
  owner/access checks, no reparse points or multi-link files, bounded reads and
  non-blocking staged replacement. Local-drive ancestors must exclude untrusted
  replacement/control rights; UNC paths and alternate data streams are rejected.
  ACLs are never repaired implicitly. Native Windows cache contracts passed at
  the [documented maintenance checkpoint](v12.md#1210-delivery-checkpoint-unreleased).
  A hostile same-user/root/administrator process
  and authenticated release verification are outside this advisory cache's
  contract. Verified CLI installation and the expanded project acceptance
  transaction passed their declared platform and release gates in the
  [12.1.0 publication](v12.md#1210-published-maintenance-release).

  **Native CLI artifact preparation:** the candidate pipeline builds both CLI
  entry points on the four targets in `.github/cli-artifact-targets.json` and
  runs each executable's version check on its native host. A bounded
  `rullst.cli-artifacts.v1` inventory binds file names, sizes and SHA-256 digests
  to version, target, build runner, repository and source commit. Ordinary CI
  inventories have no release tag and cannot be promoted to release artifacts.
  These checks establish file integrity, not publisher authenticity. Trusted
  tag-workflow provenance, client verification, staged installation, recovery
  and application acceptance remain separate required boundaries; no build or
  discovery command installs these files automatically.
  The tag-only release pipeline now calls the same native builder after exact
  protected-main admission. A separate job with signing authority checks the
  downloaded checksums and attests executables, manifests and inventory files
  without checking out source or executing downloaded binaries. GitHub release
  assets include those files only after crate publication and attestation pass.
  Client-side verification must pin the publisher, tag workflow, source tag and
  commit and then compare the expected platform/version/file digest; metadata
  and matching checksums by themselves remain insufficient authority. This
  pipeline change is unaccepted until its native and release evidence passes.

  **Explicit local artifact verification:** `cargo rullst update verify`
  takes a caller-selected directory and exact `--to` version. It accepts only
  the native supported target, a bounded release inventory and both standalone
  binaries with matching sizes and SHA-256 digests. It authenticates a private
  snapshot of the manifest through the caller-installed GitHub CLI, pinning
  github.com, Rullst/Rullst, `.github/workflows/release.yml`, the exact source
  tag/commit, GitHub's OIDC issuer and hosted runners. Failure, absence or timeout
  of that verifier is a rejection; there is no checksum-only fallback. This
  explicit operation can access attestation services and create a temporary
  private manifest, but never executes candidate binaries or changes installed
  files. Offline mode rejects before I/O. Its report describes only the bytes
  just read, is not a reusable installation token, and does not establish
  current registry eligibility or protect against hostile same-user writers.
  Installation must independently revalidate the selected release and bytes.

  **Authenticated download (12.1.0):**
  `update stage --to EXACT_VERSION` fetches a fresh non-yanked registry
  selection and uses only the fixed official
  release URL with at most two HTTPS redirects through GitHub/release-assets hosts,
  authenticates the bounded manifest before requesting executable bytes, and
  stages at most two 128 MiB binaries in fresh private caller-owned storage.
  Exact sizes and hashes must match the authenticated manifest. Ordinary failures
  discard the stage; forced termination can leave an incomplete private directory
  that later stages never reuse. Success records source/version/target and grants no execution,
  installation or project authority. Offline mode rejects before I/O; install
  must independently revalidate eligibility, provenance and bytes.

  **Managed CLI installation (12.1.0):**
  `update install review` authenticates a fresh eligible local candidate and
  previews a new/empty or receipt-owned private destination, a root/source-bound digest,
  proposed version smoke checks and the pinned Cargo source fallback. It does
  not create the installation directory, execute binaries or install files.
  An existing destination must contain exactly the two private single-link
  executables and a bounded `rullst.cli-installation.v1` receipt binding the
  canonical root to the original attested manifest bytes. Their hashes and the
  prior manifest's provenance are revalidated; unknown/package-manager entries
  and divergent files reject. Selection uses that installed version, so a stale
  CLI cannot authorize its downgrade. The review digest binds the exact prior
  receipt and rechecks local state after network verification.
  `update install apply --approved-review SHA256` uses an explicit installation
  root separate from the advisory cache and existing package-manager roots.
  First installation accepts only a new/empty private caller-owned directory;
  later updates require the updater's strict receipt and exact installed hashes.
  Never take over unknown binaries or Cargo/Homebrew/system-manager records;
  show the pinned manager/source-install command when that owner must update it.
  A preview authenticates an exact eligible native candidate and binds its
  source/files, destination and prior installed state to a review digest. Apply
  requires the matching review digest, rechecks fresh registry/provenance/bytes, holds a
  destination-local lock and copies candidates to private staging on that
  filesystem before execution or replacement. Only the declared `--version`
  probes run, with a 15-second deadline and 4 KiB per output stream per binary.
  Both must report the selected version without stderr. Authenticated prior bytes
  and a bounded intent are saved before replacing either entry point. Old entries
  are moved aside instead of truncating executing images; per-file operations
  are not an atomic two-binary swap and do not promise power-loss durability.
  Windows in-use failures report the root and approval digest for recovery.
  `update install recover --approved-review SHA256` accepts only the selected
  operation's before/after states, rejects foreign edits and reauthenticates
  both manifests before restoring the exact predecessor. Interrupted first
  installation removes only the recorded new entries. Repeated recovery is
  idempotent; an unrelated older version is never a recovery target.
  The private destination-local sibling stores a root-bound owner marker, lock,
  selected operation and at most eight operation directories, independently of
  advisory cache settings. New operations prune only verified terminal older
  evidence, retaining the selected recovery. Unknown files and incomplete
  unselected operations require manual review. An in-use historical executable
  can require closing the older CLI before pruning. PATH/shell configuration, source
  compilation, project migration and deployment remain separate consent scopes.
  Native concurrency, interruption, disk faults, executable-lock and complete
  user-journey acceptance remain release gates.

  **Guided update composition:** `update guided --to EXACT_VERSION` is an
  opt-in interactive composition of the same checked commands. An explicit
  `cli`, `project` or `both` scope reuses the selected version, directories and
  review digests. Show each complete review before a default-no approval;
  distinguish authenticated download, CLI probe/replacement, trusted project
  execution/network access and original-file application. No blanket approval
  or noninteractive implicit yes is accepted. Declining stops before the next
  operation, preserving any already completed step and its recovery evidence.
  The current CLI's versioned migration rules remain authoritative: installing
  another major does not teach this running process that major's migrations.
  Existing structured commands remain the automation interface. Report elapsed
  time per executed stage, excluding user input; do not promise instant upgrades.

  **Isolated project preparation (working source):** the opt-in
  `update project prepare` command snapshots tracked and non-ignored untracked
  files from the selected Git working directory into private caller-owned
  storage, preserving uncommitted source contents and deletions. Links,
  special files, unsupported paths and oversized inputs fail before migration.
  The copy is bounded to 100,000 entries, 64 MiB per file and 512 MiB total.
  Cargo metadata and dependency planning run only inside that copy, with
  Cargo network access and Rustup auto-installation disabled. The existing exact-version manifest editor and
  versioned source rules supply its review report; preparation executes no
  build scripts, procedural macros or application tests and grants no apply
  authority. Ignored files, including typical secret files and build outputs,
  are not copied, except the root `Cargo.lock` (legacy generators ignored that
  reproducibility input). Version requirements and existing locked Rullst
  packages must not imply a downgrade; unsupported or ambiguous requirements
  require manual review. Source reports stop above 10,000 findings. This is a
  source snapshot, not a filesystem sandbox: later
  verification must explicitly authorize trusted project execution. Bounded
  reviewed application/recovery have separate explicit consent and acceptance gates.

  **Candidate verification (12.1.0):**
  `update project verify` reloads the private preparation, validates its
  baseline/current source/candidate against bounded records and recomputed
  migration plans, rejects stale inputs, and takes an exclusive operation lock.
  `--dry-run` shows the selected commands without running builds/tests. Execution
  requires `--allow-project-code` and uses another fresh private copy. Cargo
  resolves the lockfile; every locked managed Rullst package must match the
  exact target before checking all workspace targets and running workspace
  tests with `--locked` and the selected feature policy. Default features are
  the default policy, with explicit all/custom/no-default selection. Cargo is
  offline unless separately authorized; an offline environment cannot be
  overridden. Rustup auto-installation remains disabled.
  Build outputs live outside the verified source tree. Each tool has a bounded
  deadline (900 seconds by default, at most 3,600) and at most 8 MiB per output
  stream. Execution reuses the supervised child/process-group cleanup and
  responds to cancellation. This is best-effort process-tree cleanup, not
  containment of hostile descendants. Private logs include Cargo/rustc version
  probes, commands and their output digests. Configured compiler wrappers or
  alternate toolchains still require operator review; probes are not toolchain
  attestations. Original/prepared/baseline changes, unresolved migration
  findings, failed commands and unexpected source writes reject acceptance.
  Only the candidate root lockfile may change during verification.
  `rullst.project-verification.v1` records successful commands, feature policy
  and final file digests, grants no application/deployment authority, and is not
  a reusable apply token. Native acceptance and complete application/recovery
  remain release gates. Tests execute trusted project code with the caller's
  environment: the copy is not a sandbox, and external effects cannot be
  reversed through source-file recovery.

  **Explicit candidate review (working source):** `update project review`
  revalidates the private preparation, verified file inventory and bounded
  successful command logs under both operation locks, then shows the complete
  manifest/lockfile diff and a SHA-256 digest binding that review. Git external
  diff/text-conversion helpers and paging are disabled; the diff is bounded to
  8 MiB and no build/test or original-file edit occurs. The review digest grants
  no application authority. Native acceptance remains required.

  **Reviewed application/recovery (12.1.0):**
  `update project apply --verified PATH --approved-review SHA256` requires the
  exact review digest and fresh source validation under both preparation locks
  and a canonical-source lock in the configured private cache. Only the reviewed workspace
  manifests and root lockfile are eligible. Stage all replacements before
  changing originals, preserve original permissions and replace directory
  entries without truncating hardlinks. Unix mode/owner/group and Windows
  owner/group/DACL/integrity label enter the review digest, with a 1 MiB aggregate
  serialized access-policy limit. Unix extended ACLs/xattrs and special
  mode bits require manual updates; Windows read-only/special attributes, alternate
  streams, resource/central-access policies and
  access policies that cannot be recreated exactly fail before source writes.
  Darwin extended ACLs are inspected through a narrowly scoped OS FFI module,
  since they are separate from xattr names. It owns and frees the returned ACL
  and never changes it; a native ACL regression and the exact unsafe-source
  allowlist govern this exception. The API follows Apple's
  [ACL entry contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/acl_get_entry.3.html).
  Staging installs the access policy before writing candidate contents. Keep a
  durable bounded intent record
  and before/after digests for interruption recovery. Recovery must refuse
  divergent user edits and may restore only files from that intent; absent
  original root lockfiles may be removed. Per-file replacement is not a single
  atomic workspace commit. `update project recover` with the same approval
  accepts only the recorded before/after states and supports repeated recovery.
  Locks coordinate this CLI under the same cache configuration, not editors,
  other caches or filesystem aliases. Stop other writers first; hostile concurrent
  renames, external test effects and databases remain outside file recovery.
  Forced process termination during staging can leave disposable sibling temp
  files; the private before/verified trees and intent must be retained. Timestamp,
  Windows audit-policy preservation and power-loss fault
  acceptance remain outside this implementation. Published acceptance covers
  only the declared platform and fault scenarios. Local process tests cover a
  killed per-file commit with a persisted intent and subsequent engine recovery,
  plus real CLI staging terminated by Linux's file-size limit without changing
  originals. They do not establish power-loss durability.

---

## 📱 13. Omni Packaging Contract

`cargo rullst make:omni` generates an application-owned Tauri packaging shell;
it is not a native-runtime abstraction or store-publication service.

The canonical product is the Rullst web application. Server-side domain rules,
authentication, authorization, persistence, realtime policy and security
controls remain authoritative and must work without trusting the platform
shell. Omni is **web-first, platform-enhanced**: it may add narrowly scoped
native capabilities, but it must not fork the business/security model or move
authoritative secrets into JavaScript or an untrusted client.

* 🟢 **`[Implemented / Bounded]` Deterministic Scaffold:** `--platform` accepts
  desktop, Android and iOS selections without a prompt. Mobile selections
  require `--backend-url`; HTTPS is required except for explicitly bounded
  loopback/emulator development hosts, and embedded credentials are rejected.
  Product name and application version default to validated Cargo package
  metadata. `--product-name`, `--app-version` and `--identifier` provide
  deterministic overrides. Android/iOS require an application-owned lowercase
  reverse-DNS identifier and reject framework/reserved example placeholders;
  desktop-only development may use a clearly documented `com.example` value.
* 🟢 **`[Implemented / Bounded]` Reproducible Tooling:** the generated manifest
  pins the Tauri CLI and Rust dependencies, emits a restrictive local CSP and
  real source-derived platform icons, and treats npm, icon generation or
  explicitly requested mobile initialization failures as command failures.
  Explicit iOS initialization requires macOS/Xcode.
  **12.1.0:** new shells embed the existing Rullst logo as their
  default square icon source and regenerate icons after all mobile init steps.
  Existing shells are never regenerated in place. New Android shells bind the
  release signing configuration to application-owned keystore/alias/password
  environment inputs and fail release preparation when inputs are missing;
  debug signing remains development-only. `omni android --release` checks
  required inputs before invoking Tauri, without changing the public v12
  command enum. No shared signing key, store publication or physical-device
  evidence is implied. Existing/custom-flavor shells need reviewed migration;
  see the [signing guide](tutorials/49-omni-android-signing.md).
* 🟢 **`[Implemented / Bounded v13 candidate]` Verified Android release output:** the
  CLI binds a successful build to one fresh release APK under the generated
  Android output directory. An explicit relative APK selection may disambiguate
  variants; absent, unchanged, ambiguous, linked or oversized outputs fail.
  Reproducible bytes may be accepted only when the output's filesystem timestamp
  changed during this build; a cached unchanged artifact is not fresh evidence.
  Compare artifact time with a temporary filesystem timestamp anchor created
  before the build, so filesystem clock granularity cannot invalidate a fresh
  fast build merely by lagging the wall clock. The before/after artifact identity
  comparison remains mandatory.
  An application-owned DER certificate and a trusted absolute SDK
  `apksigner.jar` path are required in addition to signing inputs. Invoke the
  jar through Java from an absolute trusted PATH entry, with bounded output,
  deadlines and process cleanup. The verifier receives no keystore or password
  environment inputs. It requires cryptographic verification of a private bounded APK
  snapshot with SDK warnings treated as errors, report exactly one supported signer and match the expected
  certificate SHA-256. Verify that the original bytes still match before
  reporting the APK digest. Tool failures expose fixed diagnostics, not captured
  logs or signing secrets. Local protocol fixtures are not real APK acceptance;
  hosted Android CI built and verified a generated signed APK through this
  CLI in PR #220, whose complete source campaign passed. Preserve that check
  in final release verification. Multiple signers/key rotation, arbitrary output layouts, AAB/store/device
  acceptance and protection from hostile build tools or same-user writers remain
  separate contracts.
* 🟢 **`[Implemented / Bounded]` Remote-content Boundary:** the generated local
  bootstrap exposes no Tauri IPC API to the remote application. A native
  navigation callback permits only Tauri's packaged origin and the exact
  scheme/host/effective-port tuple of the configured backend; cross-origin
  links and OAuth must use a separately reviewed system-browser/deep-link flow.
  The bootstrap provides an accessible initial offline/retry state, but this is
  not offline application data or synchronization.
* 🟢 **`[Implemented / Bounded]` Desktop Lifecycle:** the one-command local
  `http://localhost:3000` development profile owns its child process, refuses a
  pre-existing port rather than attaching to an unknown process, stops on early
  child exit or timeout, and terminates only the child it spawned. HTTPS and
  other configured origins are treated as externally operated backends.
* 🟢 **`[Implemented / Bounded]` Shared Wire Contract:**
  `rullst::client_contract` provides the strict `rullst.client` v1 JSON marker,
  positive version negotiation, private typed request/success/failure
  envelopes, bounded log-safe correlation and idempotency tokens, server time,
  message-free dotted failure codes and a codec capped at 2 MiB. Unknown outer
  fields, unsupported versions and oversized bodies fail closed. The same code
  compiles natively and for `wasm32-unknown-unknown`. The envelope carries no
  role, tenant or authorization claim; the server must derive those from its
  authenticated context and persist idempotency atomically.
* 🟢 **`[Implemented / Feature-gated Foundation]` Offline State Contract:** the
  native `offline-sync` feature supplies one account-bound, versioned state
  with bounded cached records, FIFO idempotent mutations, server revisions and
  cursors, atomic response application, explicit conflict isolation, full
  resync, quotas, cache recovery and logical erasure. `OfflineSnapshotCipher`
  authenticates and encrypts the closed snapshot with randomized AES-256-GCM,
  binds it to the exact account and rotation-key id, revalidates every bound
  after decryption, redacts state payloads from aggregate `Debug`, and zeroizes
  its owned key/plaintext buffers where possible. It never treats client time,
  cached identity/role/tenant, score or local revision as authority.
  A static-dispatch coordinator bounds foreground push/pull requests, mandates
  per-request timeout, stops on retryable no-progress and rejects a continuing
  page whose cursor did not advance. Keychain/Keystore, atomic platform
  persistence, browser storage, concrete authenticated HTTP, retry/background
  orchestration, concrete future-schema migrations, physical-device
  recovery/erasure evidence and application conflict UX remain explicit
  platform/application work; therefore the generated shell alone is not an
  offline-first application.
* 🟢 **`[Implemented / Hosted Compile Evidence]` Compile Evidence:** on commit
  `755fbd61933bed04369e0eb5de50b11275db5e3d`, path-aware workflows created
  disposable hosts and passed fresh desktop shell checks on Linux, macOS and
  Windows, an Android aarch64 debug APK build, and an iOS simulator build.
  These are compile gates for that SHA, not physical-device or store evidence.
* 🟠 **`[Application / Platform Boundary]`** bundle identity, signing and
  provisioning, privacy manifest and usage declarations, native capabilities,
  production endpoint/auth policy, physical-device testing, TestFlight,
  Play testing, metadata and store review belong to the generated application.
  Offline sync, push, biometrics, OS secure storage, deep links and signed
  updates are not implied by the web shell and require opt-in capability scopes
  plus platform tests. Simulator/APK compilation must never be described as
  store acceptance or universal iPhone/Android
  compatibility.
