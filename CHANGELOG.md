# Changelog 📝

All notable changes to the **Rullst Framework** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [12.0.0] - Unreleased 🚀

> **Unreleased status:** entries below are a development inventory, not release,
> certification, benchmark, or test evidence. The current capability contract is
> `docs/src/spec.md`; CI artifacts tied to the eventual tag are authoritative for
> tests and provenance. Absolute wording in older development notes must not be
> interpreted as a guarantee. In particular, live NFS-e, MQTT transport/HSM/PQC, Alipay
> RSA2, Connect message-broker adapters, S3/R2 storage, and database replication
> remain fail-closed or roadmap capabilities as documented by the SST.
>
> **Correction convention:** an extraordinary historical claim is preserved in
> its original release section. An adjacent **v12 audited scope** note states
> what can be asserted now and, when useful, which part remains a worthwhile
> ambition. This makes corrections visible instead of silently rewriting the
> project's history.

### Release summary by user impact

#### Upgrade and compatibility

- Rullst v12 coordinates 16 publishable packages in one release train. Upgrade
  direct `rullst-*` dependencies together and review the
  [migration guides](docs/src/migration-v12.md),
  [feature matrix](docs/src/feature-matrix.md), and
  [compatibility policy](docs/src/compatibility-policy.md).
- Added the first-class `rullst-messaging` crate and umbrella `messaging`
  feature. Its bounded broker contract provides a versioned envelope,
  topic-scoped idempotency, consumer-group fan-out, expiring single-use ACK
  leases, retry, dead-letter inspection, explicit purge, deterministic time,
  redacted diagnostics, and concurrent contract tests. The opt-in
  `messaging-sqlite` profile adds a fixed-schema durable local adapter whose
  publications, claims, ACK/retry/DLQ transitions and purge use serialized
  SQLite transactions; restart, multi-instance contention, configuration drift
  and corrupt-row repair have executable regressions. It remains at least once,
  stores payloads unencrypted and does not claim Kafka, RabbitMQ, Redis Streams,
  NATS/JetStream, SQS/SNS, Google Pub/Sub, or Pulsar interoperability.
- Hardened the shared mail attachment contract: messages now fail closed above
  32 attachments, 20 MiB per item or 25 MiB aggregate; unsafe basenames,
  ambiguous MIME values, duplicate/unreferenced inline CIDs and inline assets
  without HTML are rejected. SMTP now emits the proper mixed/alternative/related
  MIME tree with attachments and inline Content-ID parts, bringing the named
  REST, native SES and SMTP transports onto the same bounded owned-byte model.
  Encoding may copy data, provider limits can be lower, and opaque file bytes
  are not malware- or DLP-inspected.
- Added a compact canonical capability-status view and a SHA-bound CI quality
  scorecard artifact for every main push/PR. The score evaluates engineering
  evidence and explicitly excludes feature completeness and certification.
- Raised the v12 RC quality gate to A (90) for every publishable crate except
  the approved IoT floor of B (80). The audited plan records thirteen crates
  below A and a 67-point aggregate gap; policy values may move only with real
  implementation and evidence, never to make the release table look greener.
- Added a `no_std` OTA rollback-counter adapter to `rullst-iot`. The manager can
  load durable state and require an exact, strictly increasing compare-and-set
  before changing local state; public tests cover restart/replay, transient
  retry, corrupt load and stale-writer conflict. The trait does not implement or
  certify flash/HSM persistence, bootloader atomicity or power-loss recovery.
- Replaced the staged 80% framework-coverage threshold with one blocking 90%
  minimum for framework libraries and changed lines. The README again exposes
  dynamic Codecov and OpenSSF values, the declared Rust 1.96.0 MSRV, and the
  tag-only release-provenance workflow while explicitly avoiding a project-wide
  SLSA Level 3 certification claim.
- Added category-aware OAuth token revocation. `Provider::revoke_token` and
  `revoke_refresh_token` reject malformed/oversized values before transport;
  bounded protocol fixtures cover Google, GitHub, Discord, Apple, Auth0 and
  Cognito with their provider-specific authentication and token-type rules.
  `HttpRequest` and `HttpResponse` debug output now redacts credentials and
  payloads, revocation errors omit provider bodies, arbitrary invalid HTTP
  methods fail closed, and mock credentials stay network-free. Other
  providers still report unsupported, and remote revocation does not clear the
  host application's sessions or durable token state.
- `cargo rullst upgrade` is now a transactional, version-aware assistant. It
  offers human or versioned JSON dry runs, updates the exact Cargo workspace
  release train while preserving TOML formatting, recognizes renamed
  dependencies, scans known v5 source risks, snapshots manifests/lock/Rust
  sources, applies compiler fixes, validates with Cargo, and rolls back on
  failure. Path/git-only dependencies and application-owned data/security work
  remain explicit review items; the command never declares production
  readiness.
- The declared v12 MSRV is Rust 1.96.0. The default umbrella dependency enables
  ORM and the SQLite queue; Nexus, Studio, AI, auth, security, and other domain
  crates remain opt-in features.
- Rullst Omni now generates an application-identified, web-first Tauri shell.
  Product/version metadata inherit the host package or accept validated CLI
  overrides; mobile generation requires an application-owned reverse-DNS
  identifier. The packaged bootstrap has an exact-origin CSP, exposes no remote
  Tauri IPC and installs a native navigation policy that rejects cross-origin
  destinations. Managed localhost desktop startup now refuses a pre-used port,
  early backend exit and timeout instead of attaching to an unknown process or
  launching anyway. New path-aware workflows cover desktop crate checks on
  Linux/macOS/Windows and an Android debug APK alongside the existing iOS
  simulator compile gate; all three passed on the implementation SHA. The new
  portable `rullst.client` v1 boundary adds bounded positive-version
  negotiation, typed request/success/failure JSON, correlation/idempotency
  tokens, server time and stable message-free failure codes while excluding
  client authority. The new opt-in native `offline-sync` feature adds bounded
  account state, FIFO idempotent proposals, server revision/cursor application,
  explicit conflicts/full resync/recovery/logical erasure and randomized
  account-bound AES-256-GCM snapshots with revalidated quotas and closed schema.
  A static-dispatch coordinator adds bounded push/pull requests, mandatory
  per-request timeout, cursor-stall detection and typed run reports over an
  application-owned authenticated transport. It does not automatically mount
  persistence or HTTP/background networking: Keychain/Keystore, atomic platform
  storage, browser offline state, retry/backoff/background policy, future-schema
  migrations, physical devices, signing and store review remain explicit
  evidence boundaries.
- The generated Academy/LMS lesson boundary now distinguishes video and audio,
  accepts only HTTPS or absolute same-origin media sources, requires WebVTT
  captions for video, requires bounded transcript/language metadata for every
  lesson and keeps playback opt-in. Its materialized SQLite test verifies
  escaping, CSP nonce identity and fail-closed invalid-source/metadata cases.
  Rullst still does not claim media hosting/transcoding, caption quality,
  speech recognition, browser/device conformance or a complete learning
  product.
- Academy's generic activity contract no longer exposes a validator that can be
  handed client-authored points. `evaluate_activity` accepts only an evaluator's
  submission and uses static dispatch to build a result after owner, kind,
  attempt/ruleset, object-shaped state, time, score bounds and canonical
  SHA-256 evidence checks. The built-in `SingleChoiceEvaluator` computes
  correct/incorrect points, while the bounded `MatchingEvaluator` validates a
  complete permutation of at most eight server-owned pairs, scores it without
  trusting the client and canonicalizes order for exact replay. The
  complete-starter persistence and HTTP verticals are described immediately
  below; listening/game evaluator kinds remain open.
- The complete Academy starter now closes that first activity path with an
  owner-only `POST /activities/{id}/attempts` boundary. Its JSON accepts only an
  idempotency key and selected option; server state supplies identity, answer,
  policy, points, digest and time. The same transaction persists a bounded
  read-only activity attempt, `ScoreEvent` v2, leaderboard and outbox after
  revalidating the exact evaluator configuration under its policy lock. Identical
  retries are no-ops, while a changed option under the same key fails closed;
  attempt/event deduplication is server-scoped by learner and activity so one
  learner cannot reserve another learner's client key.
  Listening/game evaluators and quiz-path unification remain open.
- Academy now also generates owner-only
  `POST /activities/{id}/attempts/matching`. It accepts only bounded pair IDs and
  an attempt key, rejects missing/unknown/duplicate IDs, derives the answer map
  and score from the locked activity policy, and shares the same durable
  attempt/score/leaderboard/outbox transaction. Materialized tests cover order
  independence, partial server scoring, malformed input, exact retry and a
  changed pairing under the same key.
- Academy now generates owner-only `POST /activities/{id}/attempts/typed` with
  closed server-side accepted answers, a 512-byte/control-character boundary,
  trim plus optional Unicode lowercase comparison and exact policy binding. The
  durable replay key is a policy-bound SHA-256 digest, not the raw learner text;
  normalized-equivalent retries are no-ops and changed text under the same key
  conflicts. NFC/accent/fuzzy-language normalization remains application work.
- Academy activities can now opt into durable deterministic spaced review with
  the versioned `rullst-box-v1` policy. A newly applied authoritative score
  updates the learner/activity schedule inside the score transaction, while an
  exact replay cannot advance it. Owner-only `GET /reviews/due` derives learner
  and time server-side and rechecks school membership, course scope and active
  enrollment before returning a bounded queue. This inspectable algorithm is a
  foundation, not FSRS/SM-2 compatibility, efficacy evidence, speech learning
  or AI-personalized pedagogy; real PostgreSQL/MySQL contention remains open.
- Score automation now treats a globally valid achievement threshold above one
  activity's maximum as a valid non-match instead of poisoning that activity's
  outbox event. The generated regression covers the 70/80 boundary.
- `rullst-security::sha256_hex` now provides canonical lowercase SHA-256 for
  integrity identifiers without forcing an extra direct dependency into every
  LMS profile; its docs explicitly prohibit password/MAC misuse.
- `rullst-capital` now provides a bounded National NFS-e 1.01 preparation
  pipeline: checksum-pinned official production/restricted artifact manifests,
  strict ordinary-service DPS construction with integer money/rates, closed-
  catalogue validation of checksum-pinned official XSD sources (including one
  exact documented production regex compatibility normalization), protected
  PKCS#12 handling, enveloped inclusive-C14N/RSA-SHA256 XMLDSig with independent
  local verification, and bounded rustls mTLS client construction. The opt-in
  offline protocol codec now emits deterministic GZip/Base64 in the exact
  `dpsXmlGZipB64` request object and strictly separates bounded HTTP 201
  authorization from 400/403/500 rejection. It binds environment, DPS ID,
  50-digit access key, `infNFSe/@Id` and the embedded XMLDSig, rejecting unknown,
  malformed, mismatched, tampered and decompression-amplified input. Durable
  idempotency/audit, ICP-Brasil/emitter trust, real A1 restricted-environment
  evidence, independent review and SEFIN homologation remain open; live modes
  still perform no network I/O.
- Capital's signed-webhook boundary now exposes both Axum and feature-gated
  Actix Web middleware over one canonical verifier. Both paths cap raw bodies
  at two megabytes, verify before dispatch, restore the exact payload, attach a
  normalized event and reject local replay. Explicit provider-bound state
  removes the need for global configuration in isolated applications; durable
  cross-instance provider-event idempotency remains application-owned.
- `cargo rullst make:mail-invoice [Name]` and `make:mail-dunning [Name]`
  complete the bounded transactional-mail scaffold set. The fiscal template
  consumes typed Capital provenance and always labels `OfflineMock` as
  `[PREVIEW — NOT AUTHORIZED]`; the dunning template exposes explicit
  D+1/D+3/D+7 stages without inferring billing state, scheduling delivery, or
  mutating access. Both generated build paths run the mandatory mail pre-flight,
  and the materialized generator contract covers all seven variants, hostile
  markup/links, feature registration, collisions, Clippy, and runtime behavior.
- `cargo rullst make:billing --model` now generates backend-specific SQLx or
  Turso-primary models and reversible migrations, enables the exact facade
  features once, registers every module and refuses existing outputs. Its
  materialized two-backend contract runs Clippy, migrations and normalized
  webhook persistence, including a negative proving cross-owner subscription
  reuse is denied before the conflicting customer can be bound. Stripe and
  LemonSqueezy are the bounded scaffold choices; route mounting, plan policy,
  live provider validation and distributed reconciliation remain host-owned.
- Capital now exposes `SubscriptionHandle<P>` for validated, redacted,
  provider-bound `cancel()` and `pause()` operations. The explicit-provider path
  uses static dispatch; the global provider remains a compatibility path.
  `GracePeriod` adds a fallible half-open window capped at 366 days, and
  `#[derive(Billable)]` supports a complete optional start/end field pair while
  rejecting an incomplete pair. Persistence, authorization, trusted time,
  entitlement enforcement and provider scheduling remain application-owned.
- `TenantMailResolver` now accepts the trusted Core `TenantContext` directly
  for registration and delivery, validates every registry key, and returns a
  typed failure instead of treating an unavailable registry lock as a missing
  tenant. Its regression contract proves that two membership-derived contexts
  select separate drivers. Credential persistence, encryption, rotation, and
  distribution between processes remain application/deployment concerns.
- Mail failover is now disposition-aware. Provider HTTP responses, transport
  failures, rate limits with bounded delta `Retry-After`, SMTP transient replies,
  validation and configuration errors have typed classes; only transient or
  rate-limited primary failures reach a fallback or open the process-local
  circuit. Provider error bodies are bounded/redacted, telemetry uses structured
  low-cardinality fields, and circuit-state lock failures are typed instead of
  silently failing open. Distributed breaker coordination and alert operations
  remain deployment concerns.
- Mail scheduling is now durable through the built-in SQLite and Redis queues.
  `Queue::dispatch_at` and `Mail::enqueue` preserve a bounded UTC due time;
  workers cannot claim it early, consume the scheduling field before transport,
  and the Redis contract runs against the digest-pinned live service in CI and
  release verification. Direct Resend/SendGrid delivery retains provider-native
  scheduling, while real SMTP, Postmark, Log and SES-proxy paths fail closed for
  future direct delivery; offline fixtures may retain the metadata for assertions.
  Execution remains poll-dependent and at-least-once,
  not an exactly-once or provider-acceptance guarantee.
- SQLite queues now offer explicit bounded completed-job retention through
  `Queue::sqlite_with_completed_history`. Successes are still deleted by
  default; opt-in completion and pruning are atomic, retained payloads are
  visible to the real Studio snapshot, and a separate purge removes that
  history. Redis/custom inspection and host access/retention policy remain
  explicit boundaries.
- Studio's SQLx data browser now supports bounded primitive row updates and
  confirmed single-row deletion across SQLite, PostgreSQL, MySQL and MariaDB.
  The database supplies allowlisted table/column/complete-primary-key metadata,
  every value is typed and bound, unsupported codecs remain read-only, bodies
  are capped and affected-row count must equal one. Mutation handlers require
  the crate-private proof installed by the debug-loopback/same-origin access
  middleware, so importing the raw browser router cannot expose writes.
  Application tenant/RBAC, durable audit, rollback and shared production access
  remain outside Studio's local developer contract.
- Security's Schema Guard now compiles a bounded JSON Schema 2020-12 document
  or one explicit OpenAPI 3.1 component into reusable route-scoped Axum
  middleware. It preserves the exact media-type/syntax/duplicate-key/body/depth
  checks, rejects external references, disables network and filesystem schema
  retrieval, uses linear-time regexes and returns a value-free `422` on shape
  mismatch. Authentication, ownership, domain validation and non-JSON
  parameters remain separate application controls.
- Security now includes a deterministic aggregate `ThreatClassifier` and an
  opt-in proof-of-work gate. Challenges are OS-random, HMAC-authenticated,
  subject-bound, expiring, capacity-limited and atomically one-shot within one
  process. This does not claim AI/botnet attribution, traffic collection,
  autonomous blocking, distributed replay protection or DDoS prevention.
- `rullst-ai` now exposes static-dispatch `ChatMemory` and `StatefulChat`
  contracts. The bounded tenant-partitioned in-memory implementation supports
  deterministic offline work; opt-in `sql-memory` atomically persists each
  user/assistant exchange on SQLite, PostgreSQL, MySQL, or MariaDB. An even
  monotonic revision and transactional compare-and-swap reject stale
  cross-process writers without automatically repeating a provider call. Raw
  message encryption/retention, ownership within a tenant, provider audit,
  backups, migrations and conflict UX remain application concerns.
- Added an opt-in Polyglot persistence boundary to `rullst-orm`: typed document
  CRUD for MongoDB and SurrealDB, parameterized bounded OLAP queries for
  DuckDB, parameterized Turso/libSQL edge SQL with transactions and checksummed
  migrations, and bounded read-only ISO GQL for SurrealDB. The SQL Active
  Record API remains unchanged. Remote adapters support explicit deterministic
  offline modes; SurrealDB uses its public HTTP protocol rather than adding the
  BSL-licensed SDK. Umbrella `orm-mongodb`, `orm-duckdb`, `orm-turso`,
  `orm-surrealdb`, and `orm-polyglot` features mirror the lower-level flags.
- Added bounded specialized datastore contracts to `rullst-orm`. `qdrant`
  provides validated dense-cosine collection/upsert/delete/query operations,
  deterministic fallback, authenticated protocol fixtures and a digest-pinned
  live lifecycle; `cargo rullst new --qdrant` scaffolds its features and safe
  environment boundary. `redis` now additionally provides namespaced Hash, Set
  and Sorted Set operations with bounded inputs/reads, TLS-required remote
  configuration, redacted ACL credentials, deterministic fallback and a pinned
  live lifecycle. These APIs do not claim arbitrary Qdrant search, Redis
  Lists/Streams, cluster/failover, tenant authorization or cross-store
  transactions.
- Added structured ORM telemetry for generated/raw queries, generated streams,
  managed transaction outcomes and Rullst-owned pool acquisitions. Rullst span
  fields omit SQL, bindings, model values, DSNs and error strings; the existing
  opt-in Core OpenTelemetry layer exports them when the host initializes it and
  remains responsible for filters, sampling and collector policy.
- Added a lockfile-pinned Criterion comparison for Rullst ORM, Diesel and
  SeaORM. Each gets one typed SQLite connection, an equivalent indexed schema,
  100 rows and the same SQLite policy; the suite measures find, filtered read,
  count, list-ten and insert/delete and publishes all results under one CI
  commit/runner. The first local smoke contradicted the historical “negligible
  overhead versus Diesel” wording, so no superiority claim is made.
- SQLite tests generated by `#[rullst_orm::test]` now serialize their sandbox
  transaction within each test process, preventing nondeterministic schema-lock
  failures while retaining parallel execution for server databases.
- The opt-in Axum `rullst-connect::mock_idp` is now a bounded signed OIDC
  fixture: validated HTTP-loopback issuer/callback, exact registered client,
  one-shot expiring authorization codes, S256 PKCE, nonce-bound EdDSA ID tokens,
  JWKS discovery and issued-bearer userinfo. Its deterministic key and
  credentials are public test material; it is neither a production IdP nor an
  OIDC conformance claim.
- Nexus semantic forms now fail closed against their registered metadata.
  Startup bounds and validates model/field identifiers, primary keys, labels,
  relations and enum options; mutation forms cap pair/value sizes, reject
  unknown or protected fields and duplicate non-Boolean inputs, normalize
  checkbox values, and validate enum, JSON, number, date, e-mail and HTTP(S)
  URL values before executing bound SQL. Boolean inference remains automatic;
  enum variants and multiline intent remain explicit metadata because Rust
  field types alone do not expose those semantics to the model derive.
- `rullst-connect::ReqwestClient` now has first-class explicit HTTP(S)
  corporate-proxy constructors with separately supplied Basic credentials,
  bounded endpoint syntax, HTTPS-required remote authentication, no ambient
  system-proxy fallback and a local protocol-level routing/auth contract.
  PAC/WPAD, SOCKS, proxy mTLS and production network certification are not
  claimed.
- `rullst-connect` now offers an opt-in Axum/tower-sessions authorization
  transaction. `begin_oauth_session` stores state + PKCE for ten minutes,
  `begin_oidc_session` also stores nonce, and `AuthSession` removes and
  immediately saves the sole active challenge before constant-time state
  validation, then exposes
  the exact typed exchange parameters. Expiry, mismatch, missing state, replay,
  replacement and debug redaction are regression-tested. Durable session
  storage, cookie/TLS configuration, account linking/recovery and live-provider
  conformance remain application and deployment boundaries. The generic
  tower-sessions store contract is not a distributed compare-and-delete;
  simultaneous already-loaded callbacks still require idempotent account/login
  effects or an application-owned atomic challenge store. Managed redirects
  also reject non-HTTPS/non-loopback destinations, URL credentials, fragments,
  and conflicting/duplicated state, nonce or PKCE parameters.
- Studio feature-flag toggles now invalidate every already-warm
  `DbFeatureDriver` cache in the same process immediately after the database
  update succeeds. The signal is a constant-size process epoch rather than an
  unbounded flag registry. Other processes and direct database writers still
  converge through their configured TTL unless the application distributes an
  invalidation signal.
- Replaced Turso's remote `libsql` SDK dependency with a direct, bounded Hrana
  HTTP v3 transport over the workspace Rustls client. Typed parameters, remote
  CRUD, checked migrations and conditional atomic batches retain live libSQL
  conformance, including rollback evidence. This removes the obsolete
  Hyper/Rustls chain responsible for the five RustSec failures without adding
  audit exceptions.
- Auditable SQLx models now require a validated user/service/system
  `AuditContext`, derive active-tenant metadata, retain optional correlation,
  and persist recursively redacted bounded create/update/delete records in the
  model savepoint. The v2 audit schema upgrades legacy tables while retaining
  legacy version identity. Eligible update revisions expose compensating
  restore with reason/source linkage, exact model/ID/tenant binding, stale-state
  checks, PostgreSQL/MySQL row locking, and fail-closed refusal for redacted,
  legacy, create/delete, malformed, or oversized revisions. Audit failure still
  rolls back the model mutation even when an outer caller catches it. Bulk
  per-row history, disaster recovery, trusted host identity derivation, and
  durable external export remain outside this contract.
- Hardened generated `.remember(seconds)` Redis query caching with versioned
  SHA-256 keys bound to an explicit application namespace, opaque tenant scope,
  table, SQL and typed bindings. Explicit and task-scoped transactions bypass
  cache, zero TTL is rejected, missing Redis configuration fails closed, and
  Redis transport/corrupt-entry failures fall back to the database. Generated
  model saves/deletes now invalidate active-tenant/table cache keys through a
  bounded scan after commit; rollback preserves the cache. CI and release gates exercise
  hits, TTL, corrupt-entry recovery, invalidation and both transaction APIs
  against a pinned live Redis service. Raw/bulk writes and Redis
  cluster/failover remain outside this contract.
- Added a bounded post-commit contract to `rullst-orm`: `after_commit` and the
  generated `committed` observer defer process-local effects until a direct
  generated save/delete or `Orm::transaction` has committed. Redis pub/sub and
  Scout projections use the same boundary, rollback discards callbacks, all
  queued callbacks are attempted, and `PostCommit` distinguishes effect failure
  after durable persistence. These generated callbacks remain process-local,
  and caller-owned raw SQLx transactions cannot expose their eventual commit.
  Nested generated savepoints now promote callbacks only after their own
  mutation succeeds, so a caught audit/savepoint failure cannot emit a later
  committed event when the outer transaction continues.
- Added an explicit database-backed transactional outbox to `rullst-orm`.
  Enqueue participates in the domain transaction, deduplicates exact payloads
  by stream/event key, rejects conflicting key reuse, and exposes bounded
  lease/token claiming, retry and dead-letter transitions. SQLite plus live
  PostgreSQL, MySQL and MariaDB contracts cover atomic commit/rollback and
  delivery state. Delivery remains at least once: consumers own idempotency,
  production migrations and the external dispatcher.
- Added feature-gated Scout adapters for Meilisearch, Elasticsearch and
  Algolia. They share bounded index/query/document/response rules, explicit
  offline fallbacks, fail-closed one-time configuration and visible search
  errors. Meilisearch runs against a digest-pinned live container; Elastic and
  Algolia protocol fixtures verify request/response contracts without claiming
  hosted-provider or cluster conformance.
- Added an opt-in typed `pgvector` contract. Vector and distance inputs now use
  SQL bindings, ORDER BY bindings retain SQL-position order regardless of
  builder call order, and a digest-pinned PostgreSQL + pgvector matrix covers
  extension setup, typed inserts, L2 thresholds and cosine ordering. Full RAG
  policy, provider/model evaluation and production index tuning remain outside
  this bounded query contract.
- Query-builder execution now assembles CTE, JOIN, WHERE/HAVING and ORDER BY
  bindings in emitted SQL-clause order rather than method-call order. An SQLite
  regression calls those builders in reverse order and exercises `get`,
  `count`, and `paginate`.
- Added a bounded Turso-primary ORM profile for blank/API applications.
  `#[derive(Orm)] #[orm(backend = "turso")]` generates typed CRUD, equality
  filters, ordering, pagination and count methods. `TursoOrm` initializes the
  configured local or remote store; generated projects support persistent
  offline development plus checked `db:migrate`, `db:status`, and
  `db:rollback`. `make:model --migration` and `make:migration` retain the
  selected libSQL backend. SQLx-specific non-blank blueprints, relation/hook
  parity, schema auto-diff, seed generation, and transparent replication remain
  outside this bounded profile.
- The project wizard now distinguishes the primary relational ORM database
  (SQLite, PostgreSQL, MySQL, or separately contract-tested MariaDB) from
  optional Turso, MongoDB, DuckDB, SurrealDB, and Qdrant capabilities. Deterministic
  `--database`, `--turso`, `--mongodb`, `--duckdb`, `--surrealdb`, and `--qdrant` flags
  generate the corresponding feature and environment configuration. This
  supersedes the earlier v12 sidecar-only Turso scaffold claim; transparent
  replication is not implied.
- Clarified SQLite, PostgreSQL, MySQL, and MariaDB as SQLx Active Record
  primaries and Turso/libSQL as a separate typed primary for the bounded
  blank/API profile. Turso is relational, but the framework does not imply
  universal SQLx feature parity or transparent replica synchronization.

#### Safer application and administration boundaries

- The manual OWASP ZAP workflow now materializes fresh blank REST API and
  complete LMS applications through the release CLI, builds and migrates them
  in production mode, and treats every ZAP warning/failure as blocking while
  retaining informational findings. The deliberately CDN-backed blog showcase
  is reported separately as informational. This work also made Core dynamic
  responses default to `Cache-Control: no-store` unless a handler provides an
  explicit policy, and stopped the blank/API database status from returning
  internal SQL diagnostics; details remain server-side logs. The scan exposed
  missing CSRF fields in the blog showcase, which now renders the exact
  request-scoped token and rejects missing or mismatched submissions. The same
  audited contract now protects the state-changing Blank and ERP generated
  forms, including their HTMX requests, in development as well as production.
- Fixed the DAST build selector so `cargo-rullst` and the release blog binary
  are built as explicit targets and asserted executable before any server is
  started. A package-wide `--bin cargo-rullst` filter previously skipped the
  blog executable while still returning a successful Cargo build.
- Pass each DAST rule file explicitly to `zap-baseline.py`. The pinned baseline
  action forwards its `rules_file_name` input only when at least one rule is
  `IGNORE`; Rullst intentionally uses auditable `INFO` explanations instead,
  so the former input silently left those configs inactive. Unlisted warnings
  remain blocking for the generated REST and LMS surfaces.
- Corrected the Capital Actix guide URL to the deployed mdBook path and excluded
  intentional localhost tutorial endpoints from the external-link probe; local
  repository links remain covered by the blocking offline fragment check.
- Added native AWS SES v2 delivery behind `rullst-mail/aws-ses` and umbrella
  `mail-aws-ses`. `AwsSesDriver` delegates regional SigV4 and transport to the
  official AWS SDK, accepts static/temporary credentials, caller-owned rotating
  providers or a complete SDK config, serializes HTML/text, attachments/CID and
  RFC 8058 headers, rejects SES field/40 MiB encoded limits before network,
  preserves bounded rate-limit metadata and requires a non-empty SES
  `MessageId` on success. A loopback contract verifies the signed
  `ses/aws4_request`, session token, payload and typed/redacted rejection
  without contacting AWS. The facade uses
  native mode only when both standard AWS credential variables are present;
  the old constructor remains a deterministic mock or explicit trusted bearer
  proxy and can never send an unsigned AWS request. Account identity/domain
  verification, sandbox exit, IAM policy, quotas, reputation, suppression,
  live acceptance and inbox delivery remain external evidence.
- `rullst-connect::UniversalProfile` now provides the credential-free normalized
  OAuth identity projection. `ConnectUser` serialization no longer emits access
  or refresh tokens; applications must store credentials through an explicit
  encrypted lifecycle.
- `rullst-connect::AutoRefreshingSession<P>` now turns provider `expires_in`
  into a bounded process-local refresh boundary. Token state is user-bound and
  redacted, provider calls cannot overlap, waiting callers reuse a successful
  refresh, rotated credentials replace prior state only after validation, and
  failure preserves the old generation. Encrypted persistence, cross-process
  leasing, retry/backoff, local-session logout/reconciliation, reauthentication
  and revocation outside the named adapter set remain application policy.
- `#[derive(rullst::Billable)]` is now exported by the umbrella facade,
  preserves generic model parameters, and emits a focused compile error when a
  named e-mail field is absent. Invoice HTML escapes all application-supplied
  text fields before rendering.
- `Billable::charge_with` and the global-provider `charge` helper now provide a
  bounded immediate-charge contract instead of the unsafe historical
  `charge(amount)` shorthand. Requests use integer minor units and require
  currency, provider customer/payment-method IDs and an application idempotency
  key. Stripe Payment Intents forwards that key, confirms off-session, binds the
  response to amount/currency and accepts only successful/processing states;
  exact mock retries are deterministic with a distinct non-success `Mock` status
  and sensitive identifiers are redacted. Other adapters fail explicitly until
  their direct-charge path is reviewed.
- Added opt-in bounded paid-invoice delivery. `invoice-pdf` validates legacy
  invoice money into exact minor units and renders paginated A4 PDF with an
  embedded WinAnsi font or checked caller font. `PaidInvoice` requires final
  `Succeeded` evidence matching e-mail, amount and currency; the downstream
  `rullst-mail/capital-invoice` bridge attaches HTML/PDF, applies pre-flight and
  sends through the facade or a static driver. Its stable key supports a
  caller-owned durable outbox; webhook orchestration and exactly-once delivery
  are not claimed.
- Replaced live use of the ambiguous legacy metered-usage method with the
  static-dispatch `MeteredBillingProvider` boundary. `StripeMeterEvent` now
  emits the current Meter Events form contract and binds customer, meter name,
  value, timestamp and identifier; `LemonSqueezyUsageRecord` emits the current
  JSON:API subscription-item relationship and binds quantity/action. Responses
  are capped at one MiB, identities are redacted and offline mocks are
  deterministic/non-live. Stripe exposes rolling provider deduplication, while
  Lemon explicitly requires a caller-owned durable outbox key; live account
  acceptance, reconciliation and entitlement policy are not claimed.
- Completed the bounded coupon/trial contract. `CouponCode` validates and
  redacts identifiers; Stripe now sends `discounts[0][coupon]`, expands the
  resulting discount and binds both subscription and coupon. Lemon Squeezy and
  unreviewed adapters fail explicitly for post-checkout live coupon mutation.
  `Billable`/`SubscriptionHandle::extend_trial(15)` now means 15 bounded days;
  the explicit-clock variant gives stable retries, while Stripe form and Lemon
  JSON:API fixtures bind the returned trial expiration. Authorization,
  command serialization, webhook reconciliation, provider billing effects and
  real-account acceptance remain host/release work.
- Added shared Team/Workspace resource quotas. `BillingSubject::from_tenant`
  binds accounting to trusted tenant state and `Billable::quota_request`
  derives the limit from the subscription owner. `QuotaGate` reserves before
  executing, suppresses exact replay and compensates ordinary operation
  failures. The opt-in `quota-sql` store uses an idempotent claim plus atomic
  conditional counter on SQLite, PostgreSQL, MySQL and MariaDB, and exposes a
  caller-owned transaction path so the quota and domain insert can commit or
  roll back together. Local and live-container concurrency tests prove the
  shared limit. SQLite fixtures use normalized repository-relative URLs so the
  same file-backed concurrency contract runs on Windows. Membership, tier/webhook
  reconciliation, migrations, abandoned standalone reservation policy and
  Turso/NoSQL stores remain host boundaries.
- `cargo rullst make:iot` now validates device identifiers, refuses traversal
  and collisions, enables the umbrella `iot` feature, registers generated
  modules, and is checked by compiling a materialized application. IoT HTML
  cards escape untrusted labels and report `SNAPSHOT` instead of inventing an
  online state; anomaly evaluation fails closed for non-finite inputs and
  Digital Twin serialization exposes a fallible API with a safe JSON fallback.
  These changes do not add hardware, network, firmware or realtime drivers.
- `rullst-iot` now provides bounded `no_std` MQTT 5 PUBLISH and RFC 7252 CoAP
  base-request encoders with typed failures, official-format vectors, public API
  and deterministic robustness tests. MQTT topic/QoS/packet-ID/length rules and
  CoAP token/option/payload ordering fail closed under local 1 MiB/1152-byte
  ceilings. This is packet construction only: connections, TLS/DTLS, broker
  negotiation, acknowledgements, retries, LwM2M and interoperability remain
  outside the implemented scope.
- `cargo rullst make:mail` now generates all five exposed variants through the
  public facade, enables `mailer`, validates identifiers, refuses traversal and
  collisions, registers modules and escapes dynamic HTML. A materialized
  application passes strict Clippy and adversarial rendering. `MailFactory`
  fixtures use the same escaping primitive. The former unsigned AWS SES path
  fails closed; native SES v2 is now a separate opt-in path signed by the
  official AWS SDK, while deterministic offline mode and the explicit
  HTTPS/loopback bearer proxy remain available. Tracking tokens are documented
  as authenticated but not encrypted, and provider-specific
  scheduling/attachment limits stay explicit.
- Nexus fails closed unless an access policy is selected. Its local convenience
  policy is limited to debug builds and a verified loopback peer; generated
  release applications require valid credentials. `#[derive(Nexus)]` now has
  compile-checked model/field metadata for labels, icons, primary keys and
  explicit semantic widgets. Registered views escape model metadata, and
  bounded parameterized batch actions support deletion plus deactivation only
  for writable Boolean `is_active`/`active` fields.
- Generated projects use explicit CORS allowlists, exact CSRF webhook
  exemptions, ownership checks on parameterized data routes, and bounded WAF
  body inspection. Existing generated applications must apply the
  [CORS migration advisory](docs/src/cors-scaffold-security-advisory.md)
  manually.
- Session, webhook, audit-chain, OTA, DLP, RASP, Login Jail, and secret-handling
  paths received fail-closed validation and negative-path tests. The canonical
  [security claims ledger](docs/src/v12-security-claims.md) links each announced
  behavior to code, tests, and a known limit. These controls are defense in
  depth, not certification or universal attack prevention.
- `rullst-security` now rejects weak fingerprint keys, invalid client networks,
  unsafe deception paths, malformed/duplicate-key JSON and nested unsafe LLM
  prompts. Log redaction handles repeated Bearer and assignment values. MFA uses
  OS randomness for 160-bit TOTP secrets and can render an actual bounded SVG
  QR; Login Jail offers an async API that applies its progressive delay.
- Security CLI evidence was made fail-closed: an explicitly requested Geiger or
  SBOM check can no longer silently fail, the doctor parses the installed Rust
  version against MSRV and verifies autofix results, the managed hook runs
  all-feature Clippy plus bounded unsafe/IDOR checks, and the CycloneDX 1.5 SBOM
  uses parsed Cargo metadata, a valid UUID serial and unique component refs.
- Audited all 45 unique historical ORM `[x]` claims and removed a duplicated
  roadmap body. The current ledger records 26 bounded integral contracts or
  reproducible evidence items and 19 partial foundations; 100% means
  classified, not fully implemented, and the comparative benchmark refuted the
  old performance conclusion instead of confirming it.
- SQLx models declaring `tenant_column` now fail closed outside
  `with_tenant(...)`, validate the tenant field/type during macro expansion,
  inject scoped reads and reject cross-tenant full/partial updates and instance
  mutations. Reviewed global maintenance paths use explicit `unscoped()`.
- Hardened ORM structural/value boundaries: join builders reject invalid tables,
  columns and operators (including constrained joins); pgvector helpers reject
  empty/non-finite vectors and invalid distances; pagination/chunking reject
  zero sizes. Partial updates can no longer bypass model policies.
- Added fallible SQLx `chunk_by_id` and `chunk_by_id_with_tx` keyset traversal
  for generated `i32` primary keys. A SQLite regression deletes processed rows
  while proving that later IDs are not skipped by offset drift.
- Made implicit model deletes with marked direct soft-delete cascades atomic.
  Generated code reuses active transactions or creates its own transaction;
  a forced child-trigger failure proves that the parent mutation rolls back.
- Bound generated magic filters for `String`, `i32`, `f64` and `bool` fields to
  their persisted Rust value types. Dynamic column/raw query APIs remain
  explicit runtime-checked escape hatches.
- Added explicit typed inverse `morph_to` relations with macro-validated
  persisted ID/discriminator fields, lazy target discrimination, constrained
  loading and batched eager loading. `morph_name` now matches the documented
  syntax and morph foreign-key overrides are honored.
- SQLx migration tracking is written after each successful migration, so a later
  failure cannot make earlier DDL appear pending; PostgreSQL rollback uses native
  placeholders. Transaction regressions now assert database rollback state
  rather than merely checking that an error value was returned.

#### AI contracts

- Every built-in provider exposes machine-readable capabilities for text, chat,
  embeddings, vision, JSON/schema, streaming, tools, deadlines, retries, and
  cancellation. Unsupported capabilities remain explicit in the
  [provider matrix](docs/src/ai-provider-capabilities.md).
- Guardrail results now use `passed_heuristics`; the former `is_safe` name is a
  deprecated compatibility alias. Passing a filter or schema does not authorize
  tools or make model output trustworthy.
- Local `ToolRegistry` dispatch now requires an exact allowlist, authenticated
  principal authorization supplied by the application, closed bounded JSON, a
  call budget, and a mandatory audit sink. Destructive/financial approvals are
  one-use and bound to the exact payload; durable production auditing and domain
  authorization remain application responsibilities.
- Added a bounded tenant-aware `RagPipeline` that performs guarded embedding,
  static-dispatch retrieval, Unicode-safe context budgeting, guarded generation,
  source reporting, and mandatory secret-minimized audit in one operation. A
  tenant-partitioned process-local cosine retriever supports offline contracts;
  authoritative datastore authorization, durable external adapters, ingestion,
  output policy, and live-model evaluation remain application boundaries.

#### Local development and observability

- Added an English [Why Rullst?](docs/src/why-Rullst.md) guide that summarizes
  the framework's implemented technical differentiators, suitable use cases,
  explicit limits, and evaluation path without presenting roadmap work as a
  stable capability.

- Dependabot now targets the active `main` line for both Cargo and GitHub Actions;
  the frozen `v5` branch receives no routine dependency maintenance. The v12
  dependency baseline incorporates the open Cargo update set, including the
  `jsonwebtoken`, `aes-gcm`, `p256`, `tokio-tungstenite`, `syn`, `tera`,
  `base64`, and `validator` major trains; the lockfile no longer contains
  yanked packages. The remaining RSA advisory has no fixed upstream release
  and stays bounded by the documented exception policy.
- `rullst-auth` now declares the `rand_core/getrandom` capability used by
  Argon2 salt generation directly, so isolated generated applications no longer
  depend on accidental workspace feature unification.
- SemVer CI resolves the exact latest non-yanked crates.io baseline for every
  supported, already-published library target. Packages awaiting their first
  publication and proc-macro/binary API surfaces unsupported by the checker are
  reported explicitly instead of being presented as verified.
- The README keeps a compact v12 preview quick start and links to the complete
  cross-platform Zero-to-Hero tutorial, including typed startup error
  propagation and an explicit warning against deploying from mutable `main`.
- Repository governance now uses protected `main` as the active v12 integration
  and future release source, short-lived reviewed branches for normal work, and
  a frozen `v5` branch for the exact legacy source baseline. This branch move
  does not publish or certify v12.
- Coverage uploads now use GitHub OIDC instead of a long-lived Codecov upload
  secret; LCOV generation and upload failures remain blocking.
- README performance language now distinguishes compile-time expansion from
  runtime dispatch, identifies benchmark scope and historical comparison data,
  and replaces the unsupported ecosystem scorecard with a source-linked
  architectural positioning guide.
- Added working `#[orm(encrypted)]` persistence for `String` and
  `Option<String>` fields. Generated writes use authenticated AES-256-GCM
  `RULLST:v2` envelopes, generated reads decrypt before hooks, Redis query
  caches retain ciphertext, keyrings support rotation, and unsafe query-builder
  operations on randomized fields fail explicitly. Added end-to-end rotation,
  tamper, nullable-field, pluck, and query-boundary coverage.
- Aligned `FieldEncryptor` writes with the specified `RULLST:v2` envelope while
  retaining migration reads for the development-era `ENC:v2` prefix. Corrected
  Vault documentation to describe zeroization as bounded memory hygiene rather
  than heap-dump prevention.
- Hardened `make:models-from-db`: metadata lookups now bind table names for all
  three supported drivers, table module identifiers are normalized, SQL
  identifiers are allowlisted, and collisions or columns requiring unsupported
  ORM remapping fail before files are written.
- Studio Radar reports real process CPU on Windows after a sampling interval,
  refreshes KPIs from `/api/radar`, and keeps unavailable probes visibly
  unavailable.
- Audited all seven historical Studio roadmap claims. The data browser now
  rejects non-ASCII dynamic identifiers, reports query failures instead of
  rendering fake empty success and casts inspected values portably through
  SQLx Any. ER metadata lookups are parameterized for SQLite, PostgreSQL,
  MySQL/MariaDB; Mermaid identifiers are normalized under strict rendering.
  Request-log markup is escaped, completed queue records are counted only when
  a backend retains them, and the environment page adds a secret-free typed
  configuration projection. SQLite now provides an explicit bounded retained
  history while keeping deletion as the default. The later bounded primitive
  write contract supersedes this audit checkpoint's read-only browser state;
  automatic OpenAPI inference, secret-bearing request capture and Redis queue
  inspection remain explicitly outside the current contract.
- Studio's debug-only local capability now verifies the direct loopback peer and
  local `Host`, and requires same-origin `Origin` on unsafe methods. This closes
  the supported local browser boundary against direct remote access, DNS
  rebinding and cross-origin mutation without claiming production
  authentication.
- Studio no longer infers an active security guard, AI-provider reachability,
  revenue, worker state or migration success from environment variables, event
  names or empty registries. Unsupported queue operations propagate typed
  errors; migration compatibility handlers return `501`; application revenue
  snapshots and webhook inspection remain explicitly supplied and process-local.
- Removed the legacy Studio `/tools/*` aliases; supported pages use the clean
  `/studio/*` route contract.
- Queue drivers now fail explicitly when Studio-style list/retry/purge
  operations are unsupported. `purge_failed_jobs` is the canonical API; the
  misleading legacy `purge_completed_jobs` facade alias remains deprecated for
  source migration.
- Scalar uses a pinned reference asset, escapes the configured OpenAPI URL,
  returns `503` when the configured specification is missing or malformed, and
  keeps the no-script fallback status-only. The Wasm build helper now fails
  closed on missing tools, merges `cdylib` into an existing `[lib]` table,
  resolves the artifact from parsed package/library metadata and writes a
  separate hydrator that awaits generated binding initialization.
- Foundry configuration is parsed as section-aware TOML and validated through
  typed errors. The SSH flow requires reviewed, preinstalled `curl`, systemd and
  Caddy executables instead of installing packages or piping a network script
  into a remote shell. Application data, secrets and binaries are isolated under
  `/opt/rullst/<app>`; Caddy failures are blocking, plain-HTTP proxying uses port
  80, replaced files receive `.previous` copies, and a successful local
  `/health` probe is reported as a candidate deployment rather than proof of
  public DNS/TLS reachability. The current pipeline replaces the global
  Caddyfile, requires root or passwordless sudo, and leaves rollback to the
  operator.
- The blog showcase's security controls execute instrumented local primitives.
  Prompt inspections are counted once, unsigned events are not labeled HMAC
  verified, and detector results are not presented as production guarantees.
- Local security telemetry now has a frozen `LiveSecurityEvent` v1 envelope, a
  packaged JSON Schema, bounded normalized fields, and CEF extension escaping.
  It is explicitly process-local and does not claim durable SIEM delivery.
- Newly generated `.env`, Kubernetes and Foundry configurations now use
  `RULLST_ENV`; generated billing follows the same canonical-first precedence.
  Existing `APP_ENV` configurations remain supported as a legacy fallback.
- Generated JWT middleware now follows the workspace's `jsonwebtoken` 11
  baseline, and the Tera profile follows the CLI's `tera` 2.2 baseline, so
  fresh offline scaffold checks do not request stale dependency trains.
  Upgrade reports and storage object keys also use portable forward slashes on
  Windows.
- Generated blueprints and representative release builds are exercised by
  compile tests. The exact RC still requires multi-OS CI, packaged crates-only
  reproduction, and release-tag evidence before publication.
- The packaged-distribution gate now installs `cargo-rullst` offline from the
  extracted release train, including crates not yet present in the registry,
  before generating and checking all six supported blueprints.

### Detailed technical inventory (preserved)

The entries below retain the development-level inventory for traceability. The
summary above is the curated user-facing release view; this inventory must not
be read as proof that an external integration, certification, benchmark, or
release gate has completed.

- Made Nexus fail closed without an explicit access policy and added server-side
  role, field, batch, TLS-boundary, constant-time credential, and rate-limit
  controls.
- Replaced simulated Vault encryption with versioned authenticated AES-256-GCM.
- Added Ed25519 OTA manifest verification and isolated IoT simulators behind an
  explicit experimental feature; hardware and transport integrations remain
  roadmap work.
- Contained NFS-e to unmistakable offline fixtures and typed `Unsupported` errors
  for homologation/production until official integration is independently
  validated.
- Hardened webhook secrets, signatures, freshness, replay protection, request
  reconstruction, exact CSRF exemptions, trusted peer/tenant identity, and
  production startup configuration.
- Corrected Studio telemetry to display `Unavailable` for unconnected probes and
  aligned README, audit, compliance, example, and roadmap language with actual
  implementation boundaries.
- Added delta-based Windows process CPU sampling and two-second Radar KPI refresh
  through the local JSON endpoint; unsupported probes still remain visibly
  unavailable.
- Corrected prompt-inspection telemetry to count each prompt once and stopped
  Nexus from presenting unsigned local events or an unconnected audit source as
  HMAC verified.
- Replaced the blog security sandbox's display-only RASP, Login Jail, and
  honeypot actions with instrumented local primitives and tests; corrected the
  page copy so a detector decision is not presented as a production HTTP block,
  provider call, universal side-channel guarantee, or scanner grade.
- Published the v12 compatibility policy covering Rust 1.96.0 MSRV changes,
  SemVer, deprecation, prereleases, and the supported release window.
- Published the v12 Cargo feature matrix for the umbrella package and every
  independently released crate, including defaults and compatibility boundaries.
- Added evidence-scoped v5, v6, and v11-era migration guides and rebuilt
  `cargo rullst upgrade` around the versioned `rullst-upgrade-rules-v1` catalog
  and `rullst.upgrade-plan.v1` JSON schema. The CLI scopes edits through Cargo
  metadata, preserves TOML comments/order, updates normal/renamed/workspace and
  target-specific release-train dependencies, reports unversioned path/git
  entries, stores review artifacts under `target/rullst-upgrades`, restores
  automatically after failed Cargo gates, and exposes a path-validated
  `--restore` recovery command. Process-level fixtures prove dry-run, JSON,
  successful apply and rollback; databases, secrets, authorization and runtime
  compatibility remain outside the automatic boundary.
- Added machine-readable AI provider capabilities plus a public provider matrix
  covering JSON/schema, vision, embeddings, streaming, tools, deadlines,
  retries, and cancellation without promoting unsupported paths.
- Replaced the misleading `GuardrailReport::is_safe` recommendation with
  `passed_heuristics` and rewrote the AI integration guide around the guarded
  v12 client, explicit capability/error handling, and untrusted-output limits.
- Published a [CORS scaffold migration advisory](docs/src/cors-scaffold-security-advisory.md)
  for applications generated before the fail-closed origin allowlist. Updating
  the CLI alone does not rewrite middleware already copied into an application.
- Added one-click local Nexus access to generated applications through a
  debug-build-only, peer-verified loopback policy. Release builds require valid
  Basic Auth credentials from the environment, and generated Studio servers run
  only in debug builds on `127.0.0.1:5555`.
- Allowed SemVer prerelease tags such as `v12.0.0-rc.1` through the same
  package-before-publish release validation used by stable tags.

- **Enterprise Resilience, Memory Safety & Cryptographic Hardening (`rullst-core`, `rullst-security`, `rullst-capital`, `rullst-auth`, `rullst-mail`, `rullst-connect`, `rullst-orm`)**:
  - **Signed Firmware Gate & IoT Claim Containment (`rullst-iot`)**: Replaced permissive OTA signature stubs with strict Ed25519 verification over a target/version/counter/length/SHA-256 manifest, enforced commit-after-verification and monotonic anti-rollback state, and moved deterministic MQTT/HSM/PQC fixtures behind the explicit `experimental-simulators` feature with `Simulated*` names. Firmware flashing, bootloader control, a concrete hardware-backed persistent counter, real MQTT transport, HSM backends, and ML-KEM remain unimplemented.
  - **Graceful Shutdown & Zero-Downtime Deploys (`rullst-core::server::builder`)**: Implemented cross-platform termination signal handling (`SIGINT`, `SIGTERM`, `Ctrl+C`) via `shutdown_signal()` and `.with_graceful_shutdown()`, cleanly draining in-flight requests before process shutdown.
  - **Async Cancellation & Drop Safety (`rullst-core::resilience`)**: Implemented RAII `ActiveRequestGuard` for the backpressure middleware, ensuring `active_requests` counters are never leaked when client futures are dropped or timed out.
  - **Zero-Allocation RASP Request Inspector (`rullst-security::rasp`)**: Replaced per-header heap string allocations with zero-allocation ASCII case-insensitive pattern matching (`contains_ignore_ascii_case`) and static attack pattern tables.
  - **Bounded DLP Secret Masking (`rullst-security::dlp`)**: Converted AWS-key and database-DSN redaction to iterative scanning for supported textual responses, with explicit content-type, encoding, streaming, and size boundaries.
  - **In-Memory Rate Limiting & Login Guard Leak Prevention (`rullst-security::rate_limit`, `rullst-security::login_guard`)**: Added periodic asynchronous background janitors pruning expired IP sliding windows and failed login records to prevent memory growth under IP spoofing.
  - **HTTP Connection Pool Reuse (`rullst-capital::providers`)**: Centralized `reqwest::Client` singleton across all 11 billing and payout providers (`stripe`, `mercadopago`, `paddle`, `lemonsqueezy`, etc.), enabling HTTP Keep-Alive and eliminating socket storms.
  - **DoS & OOM Payload Buffering Shields (`rullst-capital::webhook`, `rullst-core::server_middleware`)**: Enforced strict 2 MB payload limit on payment webhooks returning `StatusCode::PAYLOAD_TOO_LARGE` and 10 MB limit on HMR script injection.
  - **Cipher Key Schedule Caching (`rullst-auth::auth`)**: Added `OnceLock` caching for derived `Aes256Gcm` cipher instances, eliminating repeated SHA-256 key hashing on every session encryption and decryption.
  - **Zero-Copy Outbound Email Link Scanner & Anti-CRLF Guards (`rullst-mail::security`)**: Eliminated full-body string cloning in `extract_urls` and added `is_crlf_safe` header validation preventing SMTP header injection attacks.
  - **Constant-Time PKCE Verification (`rullst-connect::pkce`)**: Added `verify_pkce_challenge` using `subtle::ConstantTimeEq` to prevent side-channel timing attacks in OAuth2 / OIDC code exchange flows.
  - **Resilient Database Pool Defaults (`rullst-orm::pool`)**: Added default `acquire_timeout` (10s), `idle_timeout` (300s), and `max_lifetime` (1800s) on global ORM connection pool initializers.
  - **O(1) Circular Telemetry Spans (`rullst-core::telemetry_spans`)**: Migrated `SpanCollector` circular buffer to `VecDeque` with `pop_front`, eliminating $O(N)$ memory moves on trace recordings.

- **Engineering Governance, Tokio Concurrency Shielding & SemVer Integrity**: Hardened framework runtime predictability, Git history discipline, and build pipeline integrity:
  - **Git Governance & Conventional Commits Policy (`.githooks/commit-msg`, `CONTRIBUTING.md`, `AGENTS.md`)**: Enforced strict Conventional Commits validation (`<type>(<scope>): <description>`) via automated Git hook, banning verbose AI-generated marketing essays and maintaining clean, audit-friendly commit histories.
  - **CLI Pre-Commit Hook Suite (`cargo rullst hook:install`)**: Upgraded `cargo rullst hook:install` to automatically configure both `.git/hooks/pre-commit` (fmt, Clippy with zero-warnings, IDOR scan) and `.git/hooks/commit-msg`.
  - **Tokio Runtime Concurrency Shielding (`rullst-auth`)**: Added non-blocking asynchronous Argon2id password hashing and verification primitives (`hash_password_async`, `verify_password_async`) utilizing `tokio::task::spawn_blocking` to prevent CPU-intensive cryptographic operations from stalling Tokio event loop worker threads.
  - **Strict SemVer CI Verification (`.github/workflows/semver.yml`)**: Removed arbitrary crate exclusions from `cargo-semver-checks`, ensuring all public library crates (`rullst-core`, `rullst-orm`, `rullst-auth`, `rullst-security`, `rullst-connect`, etc.) are continuously checked against breaking API changes.
  - **Metapackage Build Bloat Mitigation (`rullst/Cargo.toml`)**: Cleaned unconditional `sqlx` dependency from `rullst`, making SQL compilation completely optional and significantly reducing compile times for lightweight REST APIs.
  - **Axum First-Class Escape Hatches & Macro Diagnostics (`rullst-core`, `rullst-orm-macros`, `rullst-macros` - Milestone 32)**: Implemented bidirectional `From`/`Into` conversions between `rullst::Router` and `axum::Router`, `Deref`/`DerefMut` passthrough, `.as_axum()` / `.as_axum_mut()`, custom Tower fallback handlers (`.fallback()`, `.fallback_service()`), and upgraded proc-macro error diagnostics with `syn::Error::new_spanned` providing precision compiler spans and actionable suggestion hints.
  - **Trybuild Proc-Macro Compiler UI Test Suite (`rullst-macros`, `rullst-orm-macros`)**: Added automated `trybuild` compiler UI test harness validating exact compile-time error spans and user-facing suggestion messages on malformed HTML tags and invalid struct/model attributes.
  - **W3C `Server-Timing` Header Middleware (`rullst-core::server`)**: Implemented `server_timing_middleware` injecting standard W3C `Server-Timing` headers into HTTP responses for instant sub-millisecond route latency inspection directly in browser DevTools.
  - **Multi-Tenant Isolation Guard (`rullst-core::security::tenant_guard`)**: Added membership-validated tenant selection. Headers/subdomains/parameters are untrusted selectors and are accepted only against a `TenantMembership` inserted by authentication middleware.
  - **Rate Limiter Capability Boundary (`rullst-security::rate_limit`)**: Added the in-memory limiter and made unimplemented distributed backends return an explicit unsupported/configuration error instead of behaving as a no-op.
  - **CLI Auto-Repair (`cargo rullst doctor --fix`)**: Added `--fix` flag to `cargo rullst doctor` allowing automatic remediation of missing toolchain linters and dependencies.
  - **CI Continuous Benchmark Automation (`.github/workflows/bench.yml`)**: Added automated pull request benchmarks tracking sub-millisecond routing and HTML macro rendering throughput.

- **Enterprise Security Sentinels, Toolchain Doctor & Concurrency Suite (`cargo-rullst` & `rullst-security`)**: Expanded developer tooling and runtime security verification across the 6 Pillars of Verification:
  - **CycloneDX 1.5 JSON SBOM Exporter (`cargo rullst audit --sbom`)**: Generates dependency inventory artifacts with versions and package identifiers. An SBOM supports review but does not confer SOC 2, ISO 27001, or FedRAMP certification.
  - **Local Network Surface Scanner (`cargo rullst audit --network`)**: High-speed port and interface binding scanner (inspired by *RustScan*) inspecting local listeners (ports 3000, 5555, 8080, 5432, 3306, 6379, 1883, 9092) and preventing insecure `0.0.0.0` bindings.
  - **DevSecOps Git Pre-Commit Hook Installer (`cargo rullst hook:install`)**: Automated `.git/hooks/pre-commit` installer enforcing `cargo fmt -- --check`, strict Clippy (`-D warnings`), and static IDOR route audit on every commit.
  - **System & Security Toolchain Doctor (`cargo rullst doctor`)**: Unified diagnostics scanner verifying Rust MSRV (>= 1.96.0), `rustfmt`, `clippy`, `cargo-llvm-cov`, `cargo-audit`, `cargo-geiger`, `cargo-deny`, `cargo-mutants`, `kani-verifier`, and Docker Engine with instant auto-fix suggestions.
  - **High-Contention Concurrency Test Suite (`rullst-security/tests/concurrency_tests.rs`)**: Multi-threaded stress tests exercise `LoginGuard`, rate limiters, `AuditChain`, and honeypot state within bounded scenarios; they do not prove absence of every race.
  - **Transport Inventory**: Documented Rustls-backed paths and deployment responsibilities. The repository makes no formal “100% Pure-Rustls” certification claim.
- **Next-Gen CI/CD Automation & Verification Pipelines (`.github/workflows/`)**: Expanded the continuous testing infrastructure to 32 dedicated GitHub Action workflows:
  - **WebAssembly & Edge Matrix (`wasm-matrix.yml`)**: Continuous multi-target verification across `wasm32-unknown-unknown` and `wasm32-wasip1` for client Wasm Islands (`rullst-island`).
  - **Continuous Fuzzing Seed Corpus Synchronization (`corpus-sync.yml`)**: Weekly automated `cargo fuzz cmin` compacting and persisting seed discovery corpora across all 33 fuzzing targets.
  - **AI Security Sentinel & Automated PR Reviewer (`ai-sentinel-pr.yml`)**: Automated pull request security analysis running IDOR static route checks, compliance reviews, and CycloneDX SBOM validation.
  - **Unused Dependencies & Feature Flags Scanner (`udeps.yml`)**: Weekly `cargo-udeps` compilation AST analysis pruning dead optional features and bloated dependencies.
  - **Cryptographic Release Attestation (`release.yml`)**: Integrated GitHub build-provenance attestations for release artifacts. The changelog does not claim a SLSA level without an independently evaluated build platform and release.

- **Pre-Flight Deliverability, Disposable Email Filtering & Zero-Cookie Tracking (`rullst-mail`)**: Expanded `rullst-mail` with production-grade deliverability, privacy-preserving tracking, and transactional test fixtures:
  - **`DisposableEmailFilter` & Pre-Flight Deliverability (`rullst-mail::validator`)**: High-speed in-memory syntax validation (`validate_email_syntax`, `validate_email_deliverability`) and disposable domain filter blocking 150+ temporary email providers (`mailinator.com`, `tempmail.com`, `guerrillamail.com`, etc.) directly on `Message` (`.validate_deliverability()`, `.is_disposable()`).
  - **Zero-Cookie Privacy Tracking Engine (`rullst-mail::tracking`)**: Cryptographic HMAC-SHA256 open/click tracking token generator and verifier (`TrackingEngine::generate_open_token`, `verify_open_token`, `generate_click_token`, `verify_click_token`), 43-byte static transparent GIF slice (`PIXEL_1X1_GIF`), open pixel injection, and fluent link rewriting with IP privacy preservation (`.with_open_tracking()`, `.with_click_tracking()`).
  - **Transactional Test Fixtures & Mail Factory (`rullst-mail::factory`)**: Pre-built transactional email blueprints on `MailFactory` (`fake_welcome`, `fake_password_reset`, `fake_otp`, `fake_invoice`, `fake_security_alert`) for local dev, load testing, and fixture generation.
  - **v12 audited scope:** “deliverability” here means bounded syntax plus a
    static disposable-domain list, not DNS/mailbox/inbox verification. Tracking
    uses no cookie and authenticates its payload, but does not encrypt the
    base64-readable email/target URL or establish privacy-law compliance.
- **Sub-Module Modularization & Architectural Refactoring (`rullst-orm` & `rullst-core::security`)**: Continued framework-wide modularization decomposing monolithic files into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **v12 audited interpretation of the pass-rate claims below:** The recorded
    test counts and “100% test pass rate” phrases describe development runs at
    the time of each refactor. They are not coverage percentages or evidence for
    the future v12 tag; only CI artifacts tied to an exact commit can establish
    that run's result. The refactors remain useful independently of the claim.
  - **`rullst-orm` Pool & Value Sub-Modules**: Decomposed `rullst-orm/src/lib.rs` (816 lines) into `src/pool.rs` (385 lines), `src/value.rs` (150 lines), and `src/tests.rs` (165 lines), reducing root `lib.rs` to ~115 lines of clean re-exports with 100% test pass rate (91 unit tests + 20 integration tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-core::security`**: Modularized monolithic `security.rs` (689 lines) into `src/security/` (`mod.rs`, `csrf.rs`, `headers.rs`, `waf.rs`, `pii.rs`, `tests.rs`, `kani_proofs.rs`), isolating CSRF tokens, OWASP secure headers, WAF injection scanners, PII masking algorithms, and formal Kani verification proofs into focused files under 210 lines each with 100% test pass rate (145 unit tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.

- **Sub-Module Modularization & Architectural Refactoring (`rullst-core` & `rullst-studio`)**: Continued framework-wide modularization decomposing monolithic files into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-core::feature`**: Modularized `feature.rs` (891 lines) into `src/feature/` (`mod.rs`, `resolvers.rs`, `driver.rs`, `memory.rs`, `env.rs`, `toml.rs`, `db.rs`, `manager.rs`, `tests.rs`), isolating deterministic hash-bucket resolvers, the `FeatureDriver` trait, four independent flag drivers (Memory, Env, TOML, Database TTL-cache), the composable `FeatureManager` pipeline, and the static facade (`init`, `manager`, `enabled`, `enabled_for`, `variant`) into focused files under 230 lines each with 100% test pass rate *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-studio::data_browser::handlers`**: Modularized `handlers.rs` (830 lines) into `src/data_browser/handlers/` (`mod.rs`, `dashboard.rs`, `table.rs`, `migrations.rs`, `ai.rs`, `security.rs`, `telemetry.rs`), isolating the Studio Control Center index page, paginated table browser with multi-driver schema inspection, migration manager delegate, AI playground delegate, Visual Threat Radar security dashboard with live SOC incident feed and AI provider detection, and telemetry/revenue/trace delegates into focused files under 310 lines each. 100% API compatibility maintained via `pub use` re-exports; `cargo check --workspace` passes clean with 0 errors.

- **Sub-Module Modularization & Architectural Refactoring (`rullst-connect` - Providers)**: Decomposed the two largest monolithic OAuth2 authentication providers into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-connect::providers::apple`**: Modularized `apple.rs` (907 lines) into `src/providers/apple/` (`mod.rs`, `types.rs`, `provider.rs`, `traits.rs`, `tests.rs`), isolating `.p8` ES256 client secret generation, Apple OIDC JWKS caching/verification with nonce checks, and mock test suites into sub-modules under 600 lines with 100% test pass rate *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-connect::providers::google`**: Modularized `google.rs` (866 lines) into `src/providers/google/` (`mod.rs`, `types.rs`, `provider.rs`, `traits.rs`, `tests.rs`), isolating Google OAuth2 client builders, RS256 ID Token signature verification via Google JWKS, token revocation endpoints, and mock test suites into sub-modules under 580 lines with 100% test pass rate *(historical run claim; exact commit/CI artifact not retained in this entry)*.
- **Sub-Module Modularization & Architectural Refactoring (`rullst-core`)**: Continued framework-wide modularization decomposing monolithic files into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-core::error_console`**: Modularized `error_console.rs` (956 lines) into `src/error_console/` (`mod.rs`, `parser.rs`, `middleware.rs`, `api.rs`, `renderer.rs`, `tests.rs`), isolating stack trace parsing, source context snippet extraction with path-traversal guards, panic unwind catch middleware, AI explain/autofix endpoints, and dark-glassmorphic HTML console views into sub-modules under 480 lines each with 100% test pass rate *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-core::artisan`**: Modularized `artisan.rs` (935 lines) into `src/artisan/` (`mod.rs`, `runner.rs`, `studio_server.rs`, `studio_views.rs`, `tests.rs`), isolating CLI command argument translation, database initialization/dispatching, Studio control center HTTP server on port 5555, and dark-theme dashboard HTML view templates into sub-modules under 480 lines each with 100% test pass rate *(historical run claim; exact commit/CI artifact not retained in this entry)*.
- **Sub-Module Modularization & Architectural Refactoring (`rullst-connect` & `rullst-nexus`)**: Continued framework-wide modularization decomposing monolithic files into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-connect::provider`**: Modularized `provider.rs` (1,049 lines) into `src/provider/` (`mod.rs`, `traits.rs`, `types.rs`, `token_ops.rs`, `jwks.rs`, `tests.rs`), isolating OAuth2 asynchronous provider traits, DTO structs, parameter builders, token exchange helpers, JWKS memory caching, and mock test suites into sub-modules under 130 lines of production code each with 100% test pass rate (130 unit tests + 9 integration tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-nexus::crud`**: Modularized `crud.rs` (1,046 lines) into `src/nexus/crud/` (`mod.rs`, `query.rs`, `views.rs`, `handlers.rs`, `proofs.rs`), isolating SQL query builders, identifier sanitization, HTML table and form view renderers, Axum CRUD route handlers, and formal Kani verification harnesses into sub-modules under 430 lines each with 100% test pass rate (7 unit tests + 4 integration tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
- **Sub-Module Modularization & Architectural Refactoring (`rullst-orm-macros` & `rullst-core`)**: Continued framework-wide modularization decomposing monolithic files into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-orm-macros::models`**: Modularized `models.rs` (1,089 lines) into `src/models/` (`mod.rs`, `column_enum.rs`, `json_ops.rs`, `crud_ops.rs`, `query_ops.rs`, `update_builder.rs`, `redis_ops.rs`, `ai_ops.rs`), isolating model code generation, `<Model>Column` enum definitions, JSON serializers, CRUD operations, query builders, Redis Hashes, and AI/RAG context schemas into focused files under 310 lines each with 100% test pass rate (20 unit tests, macro expansion, and trybuild UI suites) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-core::server`**: Modularized `server.rs` (1,053 lines) into `src/server/` (`mod.rs`, `builder.rs`, `hotswap.rs`, `dylib_loader.rs`, `server_middleware.rs`, `tests.rs`), isolating fluent server bootstrapping, atomic hot-swappable Tower services, dynamic library loaders with UUID v4 temporary files, HMR WebSocket injectors, and Zstandard asset compression into sub-modules under 250 lines each with 100% test pass rate (145 unit tests and integration tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
- **Sub-Module Modularization & Architectural Refactoring (`rullst-orm` & `rullst-auth`)**: Continued the framework-wide architectural refactoring decomposing monolithic files into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-orm::schema`**: Modularized `schema.rs` (1,135 lines) into `src/schema/` (`mod.rs`, `validation.rs`, `column.rs`, `blueprint.rs`, `schema_builder.rs`, `migration.rs`, `join.rs`, `tests.rs`), isolating DDL blueprint generation, multi-database column abstractions, identifier validation, and artisan migrations into focused files under 340 lines each with 100% test pass rate (91 unit tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
  - **`rullst-auth::auth::passkey`**: Modularized `passkey.rs` (1,090 lines) into `src/auth/passkey/` (`mod.rs`, `config.rs`, `types.rs`, `cbor.rs`, `service.rs`, `tests.rs`), isolating zero-dependency CBOR encoding/parsing, W3C WebAuthn ceremony registrations, and pure-Rust `ring` ECDSA P-256 cryptographic assertions into sub-modules under 360 lines each with 100% test pass rate (28 unit tests) *(historical run claim; exact commit/CI artifact not retained in this entry)*.
- **Sub-Module Modularization & Architectural Refactoring (`rullst-connect`)**: Decomposed monolithic files in `rullst-connect` into clean, decoupled sub-modules conforming to Rullst's strict standard (< 500 lines target per file):
  - **`rullst-connect::client`**: Modularized `client.rs` (1,244 lines) into `src/client/` (`traits.rs`, `request_builder.rs`, `reqwest_client.rs`, `tests.rs`, `mod.rs`) reducing production files to under 225 lines each while maintaining 100% API compatibility and full Wiremock test coverage.
  - **`rullst-connect::providers::oidc`**: Modularized `oidc.rs` (1,166 lines) into `src/providers/oidc/` (`discovery.rs`, `token.rs`, `tests.rs`, `mod.rs`) isolating OpenID Connect discovery, cryptographic JWKS signature verification, and mock IDP tests into focused sub-modules under 205 lines each.
- **Transactional Email vs Marketing Automation Platform Comparison Matrix (`docs/src/crates/mail.md` & `rullst-mail/README.md`)**: Added an objective, comprehensive comparative matrix and architectural decision guide comparing `rullst-mail` (native, sub-millisecond, zero-markup Rust delivery engine with DLP secrets scanning and anti-phishing) against commercial marketing platforms (RD Station, Mailchimp, ActiveCampaign).
- **Monorepo Documentation & Link Integrity Audit**: Verified and normalized all repository links, CI workflow triggers, badges, and cross-references across all crate `README.md` files, `SUPPORT.md`, `CONTRIBUTING.md`, root documentation, and `docs/src/` to official, absolute GitHub paths (`https://github.com/Rullst/Rullst/...`), eliminating broken URLs and cross-crate 404 links.
- **Resilient Multi-Driver Email Engine & Multi-Tenancy Resolver (`rullst-mail`)**: Expanded `rullst-mail` with production-grade delivery infrastructure and B2B SaaS tenancy:
  - **Zero-Copy Attachments & Inline CID Asset Embedding Engine (`Attachment`)**: High-throughput file attachments and inline media embedding (`.attach_file()`, `.attach_bytes()`, `.attach_cid()`) with automatic MIME detection and Base64 payloads across Resend, SendGrid, and Postmark.
  - **Precision Scheduled Delivery (`.send_at()`, `.send_in()`)**: Native UTC timestamp and relative duration scheduling passed through Tokio queue workers and downstream REST provider scheduling parameters.
  - **Outbound Phishing & Homograph URL Interceptor (`rullst-mail::security`)**: Pre-flight security scanner (`scan_content_security`, `.validate_security()`) detecting dangerous URI schemes (`javascript:`, `data:text/html`) and mixed-script Unicode IDN homograph domain spoofing (e.g. Cyrillic `а` in `pаypal.com`).
  - **Multi-Driver Circuit Breaker & Automatic Failover (`FailoverDriver`)**: Primary driver dispatch with automatic cascading failover across secondary backends, atomic failure counters (`AtomicUsize`), threshold triggering, cooldown circuit breakers, and telemetry warnings (`tracing::warn!`).
  - **Dynamic Multi-Tenancy Resolver (`TenantMailResolver`)**: Dynamic per-tenant email driver routing engine (`register`, `send_for_tenant`) enabling dedicated API keys, SMTP credentials, and custom domains per organization.
  - **Native REST Delivery Drivers for Postmark & AWS SES v2**: Native `PostmarkDriver` and `AwsSesDriver` implementations with zero C-bindings, full RFC 8058 One-Click List-Unsubscribe compliance, and runtime environment resolution.
  - **v12 audited scope:** attachments own/copy bytes and transports encode
    them as required; a shared pre-flight caps count/bytes and validates safe
    filenames, MIME and unique referenced CID metadata. Resend, SendGrid,
    Postmark, native SES and SMTP consume the bounded model, with SMTP emitting
    nested MIME and the REST paths Base64-encoding. Provider limits may be
    tighter and opaque bytes are not malware/DLP inspected. Scheduling fields
    are implemented only where the provider consumes them; tenant selection now has a direct, explicit
    authenticated `TenantContext` bridge and remains in-process. Failover now
    distinguishes permanent, transient and rate-limited outcomes rather than
    forwarding every error. The
    Postmark path is live. The former direct SES bearer request was invalid
    because AWS requires SigV4 and remains fail-closed; the new `aws-ses`
    transport delegates SigV4 to the official SDK and has a local protocol
    contract, without claiming live-account acceptance or inbox delivery.
- **Sovereign Modular Frontend Engines Matrix (`cargo-rullst` & `rullst-core::frontend`)**: Replaced third-party framework scaffolding wrappers with **5 clean, standalone, native frontend engines** in `cargo-rullst` wizard (`wizard.rs`) and `rullst-core`:
  - **1. Zero-Bundle HTMX + Tailwind SSR**: Declarative compile-time `rullst::html!` macro with 0 KB JavaScript bundle overhead.
  - **2. LiveView Server-Driven UI (`rullst::live`)**: Real-time state synchronization over persistent Tokio WebSockets (Phoenix & Dioxus pattern).
  - **3. Reactive Wasm Islands (`rullst::island` / `#[client_component]`)**: High-performance client WebAssembly micro-frontends (Leptos & Yew pattern).
  - **4. Zero-Build Semantic CSS (Pico.css v2)**: Modern classless semantic HTML5 styling with automatic OS dark/light mode detection, **0 Node.js / 0 NPM build dependencies** and instant styling for native HTML tags (`/pico-demo`).
  - **5. File-Based Classic Templates (Jinja2 / Tera)**: External `.html` templates located in `templates/` with full layout inheritance (Django, Rails & Loco.rs pattern — `/templates-demo`).
- **Comprehensive Framework Comparison Matrix (`README.md` & `docs/src/architecture-decisions.md`)**: Detailed breakdown contrasting Rullst's sovereign multi-engine architecture against Leptos (WASM/Signals), Dioxus (Virtual DOM), Loco.rs (Askama/Tera templates), and Topcoat Tokio (Transpiled micro-JS).
- **Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`)**: Response-time equalization helpers with jitter and synthetic password work reduce observable user-enumeration differences. Applications must still use equivalent query/control flow and measure the deployed endpoint.
- **LLM Security Firewall & Prompt Injection Shield v2 (`rullst-security::ai_firewall`)**: Zero-latency multi-vector prompt defense engine and middleware (`LlmFirewall`, `ai_firewall_middleware`) scrutinizing inputs across direct jailbreaks (`Ignore previous instructions`, `DAN mode`), system prompt exfiltration, tokenizer delimiter collisions (`<|im_start|>`, `[INST]`), Markdown image exfiltration beacons, and invisible zero-width unicode character poisoning.
  - **v12 audited scope:** This is a bounded heuristic text filter; it adds CPU
    work, can have false positives and false negatives, and does not guarantee
    prevention of prompt injection or exfiltration. The worthwhile remaining
    work is a versioned adversarial eval suite, provider-specific measurements,
    and authorization outside the model for every sensitive tool.
- **High-Assurance Security Architecture Guide (`docs/src/security-architecture.md`)**: Comprehensive architectural guide detailing Rullst's Defense-in-Depth model, complete OWASP Top 10 & API Security Top 10 matrix mapping, and core security subsystem deep-dives.
- **"The Sovereign SaaS Blog & Publisher" Reference Showcase (`examples/blog`)**: Expanded the development integration showcase. External services remain deterministic offline fixtures, fiscal output is unsigned and unauthorized, and the example is not a production template.
  - **All 3 Front-End Paradigms**: Zero-Bundle HTMX SSR (`/`), LiveView Server-Driven UI with Tokio WebSockets (`/live-feed`, `/_live`), and Reactive Wasm Island (`/editor`, `/wasm-counter`).
  - **Hybrid ORM & Intent-Based Modeling**: Active Record multi-tenant auto-scoping via task-local storage (`apply_tenant_scope`) and Data Mapper / Repository pattern with parameterized SQLx domain aggregations (`/posts/repository`).
  - **Capital SaaS Monetization & Fiscal Preview**: Interactive pricing tiers with `Billable::check_quota` enforcement and an unsigned, unauthorized offline DPS fixture (`/pricing`). Live NFS-e and XMLDSig remain fail-closed roadmap work.
  - **Security & RASP Sandbox**: Live interactive threat inspection for SQL Injection, Path Traversal, Login Jail tarpit simulator, DLP secret masking, and Honeypot crawler traps (`/security-demo`, `/wp-admin`).
  - **AI RAG & Vector Semantic Search**: Article semantic similarity search using local Cosine Similarity matching over vector embeddings, protected by built-in Prompt Injection filters (`/ai-assistant`).
  - **Strategic Email Engine Architecture Roadmap (`rullst-mail/ROADMAP.md`)**: Formulated a comprehensive, 6-phase masterclass roadmap elevating `rullst-mail` to the sovereign standard in Rust:
  - **AI Smart Dunning & Redaction**: Automated, empathetic sales recovery sequences for failed invoices connecting `rullst-mail`, `rullst-ai`, and `rullst-capital`.
  - **Zero-Bundle CSS Inliner (MJML-Free Engine)**: High-speed Rust AST CSS parser converting styles and Tailwind classes into inline attributes with universal email client normalizers (Outlook MSO & VML buttons).
  - **"Mail Radar" in Rullst Studio (`/studio/mail`)**: Live visual HTML template previews with hot-reloading, dead-letter queue inspection, and built-in in-memory `MailTrap` for testing.
  - **Outbound DLP Secret Scanner**: Deep email inspection masking AWS tokens, database credentials, and private keys prior to transport.
  - **Dynamic Multi-Tenancy & Fiscal Blueprints**: Tenant-specific domain and SMTP resolver (`TenantMailResolver`) with built-in NFS-e DPS receipts and RFC 8058 One-Click unsubscribe compliance.
- **Zero-Crash Fuzzing Hardening & UTF-8 Character Boundary Safety**:
  - `rullst-studio` & `rullst-nexus`: Hardened `sanitize_identifier` to safely count and limit UTF-8 bytes (`chars()` iteration up to 64 bytes), completely eliminating deadly crash signals from multi-byte unicode inputs (`fuzz_studio_db`).
  - `rullst-security::log_redactor`: Hardened `redact_secrets` with strict `is_char_boundary` validation and dynamic span adjustments, preventing character slicing panics on arbitrary malformed secret tokens (`fuzz_log_redactor`).
  - `rullst-orm`: Added defensive input length guards (`2048` bytes) in `fuzz_parser`, preventing memory exhaustion over multi-million iteration fuzzing runs.
  - **v12 audited scope:** These changes address identified byte-boundary and
    input-size cases. A successful bounded fuzz run cannot prove zero crashes or
    absence of every memory/CPU exhaustion path; retain the fixes and report
    future fuzz evidence with target, corpus, duration and commit.
- **Enterprise Code Coverage (> 80%) & Monorepo Test Suite Expansion**:
  - Added integration suites across Studio, Nexus, Capital, Mail, IoT, and Connect. Pass/fail status belongs to the exact CI run and is not frozen into this changelog.
- **Supply-Chain & CI Workflow Hardening**:
  - `.github/workflows/scorecards.yml`: Set `publish_results: false` to resolve Fulcio/Sigstore 403 Forbidden token errors while preserving SARIF upload to GitHub Code Scanning.
  - `Cargo.lock` & Advisories: Upgraded `h2` to `v0.4.16` and synchronized `RUSTSEC-2026-0258` exemptions in `osv-scanner.toml`, `deny.toml`, `.github/workflows/audit.yml`, and `.github/workflows/security-audit.yml`.
  - Added `workflow_dispatch` (manual execution) and push/PR triggers across security audit and sanitizer workflows.
- **CI & Supply-Chain Security Hardening**: Added Portuguese and fiscal standard terms to `.typos.toml`, registered `RUSTSEC-2026-0253` exemptions in `osv-scanner.toml` and `deny.toml`, and resolved embedded toolchain target dependencies in `no_std-build.yml` and `iot-integration.yml`.
- **Multi-Provider Payment Infrastructure Expansion (`rullst-capital::providers`)**: Decomposed `rullst-capital` into a decoupled, modular architecture under `src/providers/` (< 250 lines per module) supporting 11 payment and disbursement providers with constant-time HMAC-SHA256 signature verification (`subtle::ConstantTimeEq`), mock checkout fallbacks, and unified `WebhookEvent` mapping:
  - **`AlipayProvider` (China & APAC Cross-Border)**: Native Alipay (支付宝) and Alipay+ digital wallet integrations supporting fast cashier checkouts, RSA2/HMAC-SHA256 signature verification, and cross-border customs/VAT compliance for over 1.3 billion users.
    - **v12 audited scope:** A provider-shaped adapter and deterministic mock
      path exist, but Alipay RSA2 live verification and customs/VAT compliance
      are not implemented or homologated; unsupported live verification fails
      closed. This remains worthwhile only with official protocol fixtures,
      sandbox interoperability tests and specialist review.
  - **`InfinitePayProvider` (Brazil)**: Seamless Pix processing with **0.00% fee**, instant settlement (D+0/D+1), and domestic credit card rates (~0.75%-1.44%) with transparent installment interest pass-through.
  - **`PolarProvider` (Developer-First)**: Merchant of Record (MoR) for monetizing open-source repositories, GitHub backers, software licenses, and micro-SaaS subscriptions.
  - **`PaddleProvider` (Enterprise MoR)**: Global B2B SaaS quote-to-cash transactions with automated EU VAT and sales tax compliance.
  - **`RazorpayProvider` (India & Southeast Asia)**: Recurring UPI Autopay, Indian credit cards, net banking, and subscription orders.
  - **`MercadoPagoProvider` (LATAM)**: Regional subscriptions and payment preferences across Brazil, Argentina, Mexico, Chile, and Colombia.
  - **`CoinbaseCommerceProvider` (Web3 & Crypto)**: Borderless cryptocurrency charges (Bitcoin, Ethereum, Solana, USDC/USDT) with automated on-chain webhook verification.
  - **`PicPayProvider` (Brazil)**: Brazilian digital wallet and QR-code payments.
  - **`WiseProvider` (Global Payouts)**: High-speed, low-fee international B2B payouts and contractor disbursements across 40+ currencies.
- **Payment Gateways & Financial Infrastructure Guide (`docs/src/payment-gateways-guide.md`)**: Comprehensive architectural guide analyzing Direct Merchant vs Merchant of Record (MoR) vs Domestic Low-Fee vs Web3 Crypto vs International Payouts, complete fee comparison matrices, and step-by-step Rust configuration code.
- **Monorepo Examples & Reference Apps Guide (`docs/src/examples.md`)**: Dedicated official book chapter detailing the role of the `examples/` directory in Rullst, contrasting monorepo showcases (`rullst-blog-example`) with interactive CLI scaffolding blueprints (`cargo rullst new ... --blueprint blog`).
- **`examples/blog` Documentation (`examples/blog/README.md`)**: Complete architectural overview, local execution guide (`cargo run`), interactive route catalog (`/`, `/live-counter`, `/wasm-counter`), multi-tenant testing via `X-Tenant-ID` headers, and CI/CD integration details.
- **Bounded NFS-e Preparation (`rullst-capital::fiscal`)**: Retained the unmistakably unauthorized deterministic mock and added the pinned DPS/XSD/XMLDSig/protocol-codec/mTLS preparation pipeline described in the release summary. Homologation and production continue returning `Unsupported` until the remaining certificate-trust, durability and external homologation evidence is complete.
- **Secure Headers Suite (`rullst-security::headers`)**: Unified middleware for HSTS, dynamic CSP nonces, Permissions-Policy, COOP, COEP, and CORP. Scanner grades depend on the deployed page and infrastructure; no A+ result is guaranteed.
- **Anti-Bruteforce Tarpit & Login Jail Engine (`rullst-security::login_guard`)**: Bounded in-memory delay/jail engine; `record_login_failure_and_wait` applies the progressive async delay directly while the compatibility API returns the decision to the caller.
- **RASP Deep Request & Header Inspector (`rullst-security::rasp`)**: Enhanced runtime application self-protection with `inspect_text` and `inspect_headers` intercepting JNDI/Log4j, RCE, and advanced SQL injection vectors before handler dispatch.
- **HTTP Response DLP Interceptor (`rullst-security::dlp`)**: Data Loss Prevention middleware (`DlpResponseLayer`, `mask_response_payload`) intercepting outgoing HTTP response streams to neutralize accidental leakage of private keys, AWS credentials, and database passwords.
- **CLI IDOR / BOLA Static Audit Scanner (`cargo rullst audit --idor`)**: Static AST analyzer in `cargo-rullst` scanning parameterized routes (`/:id`, `/{id}`, `/users/:user_id`) to verify that ownership validation (`RbacGuard::authorize_owner_or_role`) is enforced.
- **Multi-Factor Authentication Engine (`rullst-security::mfa` & `cargo rullst make:mfa`)**: OS-random 160-bit secrets, RFC 6238 TOTP generation/constant-time verification, `otpauth://` enrollment, bounded SVG QR rendering and the `make:mfa` scaffold.
- **Dynamic Threat Deception Traps (`rullst-security::deception`)**: Dynamic decoy route registry (`register_deception_trap`, `deception_trap_middleware`) baiting automated scanners (`/api/v1/admin/debug`, `/graphql/v1`) and triggering instant WAF IP bans.
- **Cross-Site WebSocket Hijacking Guard (`rullst-security::cswsh`)**: WebSocket upgrade handshake validator (`cswsh_guard_middleware`) verifying Origin and Host headers to prevent unauthorized cross-origin WebSocket streams.
- **Sliding-Window Rate Limiter (`rullst-security::rate_limit`)**: In-memory sliding-window IP rate limiter (`rate_limit_middleware`, `is_rate_limited`) protecting sensitive login, password reset, and API endpoints from brute-force attacks.
- **SIEM Evidence Boundary (`rullst-security::siem`)**: `format_cef_event` safely escapes CEF fields and `dispatch_siem_alert` records process-local events. External Datadog, Splunk, Elastic, Slack and Syslog transports are not implemented.
- **Log & Secret Redaction Engine (`rullst-security::log_redactor`)**: Bounded best-effort sanitizer (`redact_secrets`) for recognized Authorization Bearer tokens, passwords, AWS access keys, and API-secret patterns. It is defense-in-depth, not a zero-leak guarantee.
- **Subresource Integrity (SRI) Signer (`rullst-security::sri`)**: SHA-384 hashes and escaped script/link tags can be generated from bytes or explicitly selected bounded local asset files.
- **Zero-Trust Client Fingerprinting (`rullst-security::zero_trust`)**: HMAC binding rejects weak keys/invalid IPs and normalizes IPv4/IPv6 subnets plus caller-supplied observations. TLS/JA3/JA4 collection and host session invalidation are not automatic.
- **Strict API Payload & JSON Bomb Guard (`rullst-security::schema_guard`)**: Middleware enforces exact JSON media types, syntax, recursive duplicate-key rejection, payload size and nesting depth; it is not OpenAPI/JSON Schema conformance.
- **Security Evidence Exporter (`cargo rullst audit --compliance`)**: Exports a Markdown record of executed, failed and unassessed checks without claiming OWASP, SOC 2, HIPAA or ISO certification.
  - **v12 audited scope:** The exporter creates a self-assessment/evidence
    worksheet. It neither evaluates every control nor grants OWASP, SOC 2 or ISO
    certification; external scope, operational evidence and an authorized
    assessor remain necessary.
- **Threat Radar UI Expansion (Studio & Nexus)**: Integrated counters emitted by installed security controls. Audit integrity and unavailable probes are no longer reported as healthy without a connected verifier/source.
- **Scoped Concurrency & Memory Sanitizers (`sanitizers.yml`)**: Added Nightly ThreadSanitizer (`TSan`) and AddressSanitizer (`ASan`) jobs for their declared package/target matrix. Results apply only to the exercised paths and CI run.
- **Cross-Platform Multi-OS CI Matrix (`ci.yml`)**: Expanded test suite to validate Ubuntu, macOS (Apple Silicon ARM64), and Windows MSVC runners.
- **Automated End-to-End Smoke Verification (`e2e-smoke.yml`)**: Automated production binary boot of `examples/blog`, live SSR HTML status 200 checks, security header validation, CSRF metadata exemption, and form submission database persistence.
- **Scoped Kani Model Checking (`kani.yml`)**: Model checks cover the explicitly defined harnesses. They do not constitute mathematical verification of every path in every crate.
- **30+ Production `libFuzzer` Targets & Google OSS-Fuzz Readiness (`fuzzing.yml` & `oss-fuzz/`)**: Added dedicated fuzz targets across all packages and complete Google OSS-Fuzz integration package (`project.yaml`, `Dockerfile`, `build.sh`) for 24/7 cloud cluster fuzzing.
  - **v12 audited scope:** Repository targets and OSS-Fuzz packaging exist. That
    does not by itself prove enrollment, uninterrupted 24/7 execution, adequate
    corpus quality or coverage of every parser. CI and OSS-Fuzz evidence must be
    linked to substantiate each of those narrower claims.
- **Miri Strict Provenance & Memory Safety Matrix (`miri.yml`)**: Expanded Miri interpreter coverage to 13 packages with randomized layouts.

- **Tool-Calling Automático para Agentes de IA (`rullst-ai::tools`)**: Added `ToolRegistry` and `AiTool` trait for exporting OpenAI, Anthropic, and Ollama compatible JSON Function Calling schemas and executing dynamic tools (Milestone 19).
- **Agentic DevOps & Infrastructure Tuning (`rullst-core::devops`)**: Introduced `DevOpsAgent` inspecting Tokio runtime tick latency and RAM memory from `rullst::radar` to calculate autonomous SQLx connection pool and thread scaling recommendations (Milestone 22).
- **Auto-Healing Database Migrations (`rullst-orm::auto_healing`)**: Added `SchemaErrorInterceptor` diagnosing SQLx missing table/column errors during development and generating corrective SQL migration scripts (Milestone 23).
- **One-Click PaaS Deploy (`cargo rullst deploy`)**: Added 1-click cloud deployment CLI supporting Fly.io, Railway, Render, and VPS Docker Compose + Caddy reverse proxy with automatic SSL certificates.
- **LiveView-Style Server-Driven UI (`rullst::live` & `cargo rullst make:live`)**: Added real-time WebSocket state synchronization engine and `make:live` component scaffolding for zero-JavaScript reactive interfaces.
- **Zero-Cost Compile-Time Dependency Injection (`rullst::di`)**: Introduced static dispatch DI container (`Container`), `Injectable` trait, and Axum `Inject<T>` extractor.
- **Microsserviços gRPC (`cargo rullst make:grpc`)**: Added `make:grpc` CLI generator scaffolding Protobuf `.proto` definitions and `tonic` gRPC service implementations.
- **Rullst Radar — Kernel-Level Telemetry (`rullst::radar` & `/studio/tools/radar`)**: Added real-time process RSS memory tracking, Tokio runtime tick latency measurement, and native Prometheus text-format exporter (`GET /metrics`).
- **Kubernetes-Native Infrastructure (`cargo rullst make:k8s` & Health Probes)**: Added `make:k8s` CLI generator for Cloud-Native manifests (`deployment.yaml`, `service.yaml`, `configmap.yaml`, `hpa.yaml`, `ingress.yaml`, `all-in-one.yaml`) and built-in Liveness (`GET /health`) and Readiness (`GET /ready`) HTTP probes.
- **Interactive Scalar API Docs (`cargo rullst make:scalar` & `/docs`)**: Added `make:scalar` CLI generator and zero-config Scalar OpenAPI reference page served at `/docs` with CDN loading and static offline fallback.
- **Rullst Capital Revenue Dashboard (`/studio/tools/revenue`)**: Added real-time MRR/ARR analytics metrics (`RevenueMetrics`), active subscriber stats, churn rate calculator, and live Stripe/LemonSqueezy Webhook Audit Inspector.
- **Interactive CLI Wizard Options (`cargo rullst new`)**: Added interactive selection prompts for ORM Architecture (Active Record, Data Mapper / Repository, Hybrid) and Frontend Engine (Zero-Bundle HTMX, Leptos SSR, Dioxus SSR).
- **3 New CI/CD Workflows:**
  - **`no_std-build.yml`:** Automated bare-metal compilation check for `rullst-iot` on 3 targets (STM32 Cortex-M4/M7, Cortex-M0, ESP32-C3 RISC-V).
  - **`iot-integration.yml`:** IoT host tests plus Cortex-M `no_std` cross-compilation; no hardware or QEMU execution claim.
  - **`pqc-compliance.yml`:** Scheduled signed-OTA and simulator-boundary checks; the workflow does not assert ML-KEM, HSM, or post-quantum compliance.
- **IoT Verification Badges (README.md):** Documents `no_std` builds, signed OTA tests, and simulator containment without asserting PQC or HSM compliance.
- **`rullst-iot` planned industrial integrations (Roadmap; not implemented or certified):**
  - **OPC-UA Protocol Driver (`rullst_iot::opcua`):** Industry 4.0 OPC-UA driver for SCADA/MES/ERP communication (ISA-95, IEC 62541).
  - **MQTT Sparkplug B Profile (`rullst_iot::sparkplug`):** IIoT-standard Sparkplug B over MQTT for Unified Namespace interoperability.
  - **IEC 61508 / IEC 62443 Safety Mode (`rullst_iot::safety`):** SIL 2/3 safety-critical deterministic execution with watchdog timer and memory protection.
- **Rullst Vault (`FieldEncryptor`)**: Added `Zeroize`-on-drop memory hygiene through `VaultSecret<T>` and versioned AES-256-GCM field encryption in `rullst-security`; no ChaCha20-Poly1305 implementation is claimed.
- **Intent-Based Modeling (`rullst-orm`)**: Added `IntentAnalyzer` in `rullst-orm` to auto-generate `CREATE INDEX` migrations directly from plain-text Rust doc comments (`/// @index(...)`).
- **IoT data and frame helpers (`rullst-iot`)**: Introduced a `no_std`-compatible helper crate; network transports and hardware drivers are not part of this release:
  - **GPIO state and I2C frame helpers (`rullst_iot::gpio`, `rullst_iot::i2c`)**: In-memory GPIO state plus I2C transaction byte construction; no register access claim.
  - **Modbus Frame Helper (`rullst_iot::modbus`)**: Modbus request frame construction with **CRC-16**; no serial/TCP transport claim.
  - **BLE GATT Data Structures (`rullst_iot::ble`)**: Service and characteristic models; no BLE server or radio integration claim.
  - **Anomaly Evaluator (`rullst_iot::anomaly`)**: Deterministic statistical threshold evaluation for `no_std` applications.
  - **Dashboard Renderer (`rullst_iot::ui`)**: HTMX sensor-card string rendering; no measured footprint claim.
  - **Mesh Topology Model (`rullst_iot::mesh`):** In-memory RSSI relay selection; no P2P transport or self-healing network claim.
  - **Signed OTA Manifest Gate (`rullst_iot::ota`):** Strict Ed25519 verification and monotonic rollback checks before inactive-partition selection; flashing, a concrete durable-counter backend, and bootloader integration are not implemented.
  - **Explicit HSM Fixtures (`rullst_iot::hsm`):** Deterministic `SimulatedHsmDevice` fixtures behind `experimental-simulators`; no hardware binding, protected key, or signature claim.
  - **Explicit PQC Fixtures (`rullst_iot::pqc`):** Deterministic `SimulatedPqcFixture` values behind `experimental-simulators`; no ML-KEM/Kyber or quantum-safety claim.
  - **Power Policy Helper (`rullst_iot::power`):** Calculates a recommended mode from supplied voltage values; it does not control sleep or power hardware.
  - **Digital Twin State Model (`rullst_iot::twin`):** In-memory telemetry snapshots and JSON serialization; no bidirectional network sync claim.
- **CLI IoT Generator (`cargo rullst make:iot <DeviceName>`)**: Added `make:iot` subcommand in `cargo-rullst` to scaffold IoT edge device modules in seconds.
- **Database Replication Boundary (`rullst::db::replica`)**: Configuration types remain available, but unimplemented synchronization returns `Unsupported` instead of logging simulated success.
- **RASP - Runtime Application Self-Protection (`rullst-security`)**: Introduced bounded heuristic inspection for common SQLi, traversal, SSRF, and RCE indicators; it is not a complete parser or authorization layer.
- **Distributed Tracing Visualizer (`rullst::studio::traces`)**: Added microsecond telemetry span collector (`SpanCollector`) and flamegraph visualizer interface in Rullst Studio (`http://localhost:5555/studio/tools/traces`).
- **Dylib Hot Reloading ABI Integrity Guard**: Integrated SHA-256 fingerprint verification during dynamic library hot-swapping in `rullst-core` dev mode.
- **Framework Escape Hatches (`cargo rullst eject`)**: Added `cargo rullst eject [--force] [--output <path>]` as a migration starting point for standard Axum/Tokio code; generated output requires review and verification.
- **Hybrid ORM Repository Pattern (`rullst-orm`)**: Added `Repository<T>` trait and `GenericRepository<T>` in `rullst-orm` to support Data Mapper / Repository pattern alongside Active Record models.
- **Hybrid Frontend SSR Adapters (`rullst-core/src/frontend.rs`)**: Introduced `LeptosAdapter` and `DioxusAdapter` for seamless rich-client SSR integration alongside the default 0KB HTMX bundle mode.
- **Native Real-Time Engine (`rullst::realtime`)**: Introduced `Channel`, `BroadcastManager` (pub/sub over `tokio::sync::broadcast`), and `PresenceTracker` in `rullst-core` for declarative WebSockets and SSE.
- **`rullst-security` Monorepo Crate**: Introduced dedicated security suite crate containing:
  - **`rullst-honey`**: Deception security engine deploying synthetic honeypot routes (`/.env`, `/admin.php`, `/.git/config`) and invisible form fields to fingerprint and ban malicious bots in memory (`DashMap`) and WAF.
  - **`rullst-sanitizer`**: XSS/SVG HTML sanitization engine powered by `ammonia`, plus `CspSecurityLayer` middleware generating dynamic per-request CSP nonces and Clickjacking headers (`X-Frame-Options: DENY`).
  - **`rullst-rbac`**: Declarative Role-Based Access Control (`UserContext`, `RbacGuard`) natively preventing BOLA / IDOR attacks via `authorize_owner_or_role`.
  - **`rullst-audit-log`**: Canonically encoded HMAC chain with sequence verification. It is tamper-evident when records and keys are protected, not tamper-proof.
- **Security Auditor (`cargo rullst audit --ai`)**: Added bounded `.env`, dependency, unsafe-source and IDOR checks plus deterministic remediation suggestions. The legacy `--ai` flag does not invoke an LLM or certify the project.
- **Automated TypeScript SDK Sync (`cargo rullst dev --ts-sync`)**: Added `--ts-sync` flag to `cargo rullst dev` to automatically regenerate `sdk.ts` on file changes during development.
- **Object Storage Boundary (`rullst::storage`)**: Added a contained local driver. Unimplemented S3/R2 and media operations fail with typed `Unsupported` errors rather than reporting success.
- **Dynamic Package Manager CLI (`cargo rullst pkg`)**: Added `cargo rullst pkg add <name>` and `cargo rullst pkg list` to inspect and inject `RullstPackage` community dependencies.
- **Visual Migration & Seeder Manager in Rullst Studio**: Added in-browser execution panel for `db:migrate`, `db:rollback`, and `db:seed` directly from `http://localhost:5555`.
- **AI & RAG Playground in Rullst Studio**: Integrated interactive prompt test bench and RAG context builder UI inside Rullst Studio (`http://localhost:5555`).
- **Full CRUD Resource Scaffolding (`cargo rullst make:resource <Name>`)**: Added `make:resource` command in `cargo-rullst` to scaffold Model, Migration, Controller, and HTMX Views in a single command.
- **Interactive Dev Error Console**: Enhanced `rullst-core/src/error_console.rs` with a Whoops/Ignition-style interactive in-browser error stack trace, source line preview, and HTTP request inspector under `cfg(debug_assertions)`.
- **Multiplatform Dev Build Tuning**: Enhanced `.cargo/config.toml` scaffolding in `cargo rullst new` with target-specific debug symbol splitting (`split-debuginfo = "unpacked"` for Linux/macOS) and FastLink support (`link-arg=/DEBUG:FASTLINK` for Windows MSVC).
- **Expressive & Borrow-Checker Safe Transactions**: Added `Orm::transaction(|tx| async move { ... })` closure helper in `rullst-orm` with automatic task-local scoping (`CURRENT_TX`), commit-on-success, and rollback-on-error behavior.
- **Fast-Linker Scaffolding**: Integrated pre-configured `.cargo/config.toml` in `cargo rullst new` for sub-second incremental recompilations via `mold` (Linux) and `lld` (macOS/Windows).
- **AST-Based Deterministic Codemods**: Upgraded module generation in `cargo-rullst` to use AST parsing (`syn::parse_file`) for `register_mod_ast`, eliminating regex code injection errors.
- **Actionable Proc-Macro Diagnostics**: Enhanced relation and model validation error messages in `rullst-orm-macros` with actionable resolution hints.
- **Reverse ORM Scaffolding (`cargo rullst make:models-from-db`)**: Added `make:models-from-db` alias in `cargo-rullst` for reverse-engineering Rust `struct` models from existing database schema tables.
- **CLI Inspection Tooling (`cargo rullst inspect`)**: Introduced `cargo rullst inspect [target]` to statically inspect active route tables (`route`), ORM struct models (`model`), and JSON structural schemas (`schema`) directly in the terminal, eliminating proc-macro opacity.
- **Zero Lock-In & Axum/SQLx Interoperability Guide**: Published [`docs/src/axum-sqlx-migration.md`](docs/src/axum-sqlx-migration.md) detailing 1:1 extractor equivalences and step-by-step escape-hatch refactoring instructions for developers using raw Axum or SQLx.
- **Community Extension Package Specification**: Published [`docs/src/packages-spec.md`](docs/src/packages-spec.md) establishing the `RullstPackage` trait and manifest standard for third-party community extensions.
- **Rullst-Studio Advanced Tooling**: Added three new interactive tools to the Studio dashboard:
  - **Environment Viewer**: Safely inspect all active environment variables (with auto-masking for sensitive keys like passwords and secrets).
  - **Feature Flags Manager**: A zero-config UI to list and toggle database-backed feature flags (`rullst_feature_flags`) in real-time.
  - **Visual ER Diagram Generator**: Automatically parses your active SQLite or Postgres database and renders a live Mermaid.js interactive Entity-Relationship diagram in the browser.
- **Rullst-Nexus Batch Actions**: The admin panel now supports performing bulk operations across multiple selected rows. Includes out-of-the-box support for `Delete Selected`, `Deactivate/Activate` (auto-detects active boolean columns), `Export to CSV/JSON`, and `Duplicate`. Fully compatible with SQLite and Postgres.
- **Redis Hash Mapping (Key-Value):** Added support for native Redis Hashes mapping to ORM models. Using the `redis` feature, models now automatically generate `.save_to_redis()`, `.get_from_redis(id)`, and `.increment_redis_field()` methods that serialize model structures into Redis Hashes.
- **Distributed Graph Traversal (CTEs):** Added support for native SQL Common Table Expressions (`WITH` and `WITH RECURSIVE`) directly in the `QueryBuilder`.
- **Auto-Embeddings Sync (`rullst-ai` + `rullst-orm`):** Introduce `#[orm(embedding_for="content")]` macro attribute. Whenever a model is saved via `.save_with_embedding(&client)`, it automatically calls the Embedding API and saves the resulting vector (`pgvector`) to the database.
- **RAG Context Trait:** Introduce `#[orm(rag_context)]` macro attribute to auto-generate the `RagContext` trait, gluing text fields together safely at compile time.
- **RAG Prompts:** Added `rullst_ai::ai::rag::build_rag_prompt` to prevent LLM hallucinations by injecting contexts explicitly.
- **Resilient AI Routing (`FallbackProvider`):** Introduced a high-availability AI router. The framework automatically scans for multiple keys (`OPENAI_API_KEY`, `GEMINI_API_KEY`, etc.) and seamlessly fails over to fallback models if the primary provider goes down or times out.
- **Zero-Boilerplate Vision API:** Added `prompt_with_image` to the `AiClient` with automatic *Magic Byte Inference*. Developers can pass any raw buffer (`&[u8]`) and the framework instantly detects if it's a PNG, JPEG, WEBP or GIF, encoding it without manual MIME type specification.
- **Chat Memory CLI:** `cargo rullst make:chat-session` now emits compiling
  SQLx or Turso-primary `ChatSession`/`ChatMessage` models, a reversible schema
  migration and a bounded `StatefulChat` service. The generator enables the
  required umbrella features, registers modules, serializes sends per service,
  propagates database/provider errors, rejects unknown stored roles and refuses
  to overwrite an existing scaffold. Materialized tests exercise migration and
  persistent mock-provider conversations on both backends.
- Auto Migration (`make:migration:auto`): Automatically generate SQL migration scripts by diffing `#[derive(Orm)]` structs against the database schema. Destructive operations (DROP COLUMN/TABLE) are generated as commented-out code by default for safety.
- Turso Integration: Added `--turso` flag to `cargo rullst new` wizard. Generates a Docker Compose configuration with an embedded LibSQL `sqld` replica sidecar that syncs with your Turso remote database, allowing standard `sqlite` macros and drivers to continue working normally via local loopback. 🚀

- **Buildah Support (SecOps)**: Added the `--buildah` flag to `cargo rullst new` which generates a `build_buildah.sh` script for daemonless and rootless OCI image building, catering to extreme enterprise security requirements.
- **Role-Based Access Control (RBAC)**: Added the fail-closed `HasRole` /
  `RequireRoleLayer` boundary. The umbrella facade now exports
  `#[rullst::require_role("Admin")]`; it validates async handler shape and role
  literals at compile time, preserves arguments/generics/where clauses and
  requires an explicit authenticated `user` binding before returning 403.
- **Declarative Policies (Gates)**: Added fail-closed named
  `Policy<User, Resource>` structs for calls such as
  `PostPolicy::can_edit(&user, &post)`, while retaining the legacy
  `Gate<Resource>` user-implemented trait for compatibility.
- **Background Mail Queues**: Integrated `rullst-mail` seamlessly with `rullst-core::queue`. Invoking `rullst::mail::init_queue()` now automatically configures `Mail::send()` to dispatch emails asynchronously via the background worker, preventing main-thread blocking. Added `Mail::send_now()` to bypass the queue when synchronous delivery is explicitly required.
- **Capital `Billable` Trait**: Added `#[derive(Billable)]` to `rullst-macros` enabling models to seamlessly integrate with `rullst-capital`'s payment engine via a global `BillingProvider` registry.
- **Capital Webhooks Middleware**: Introduced `verify_webhook` Axum middleware to intercept, cryptographically verify, and parse Stripe/LemonSqueezy webhooks automatically before they hit your handlers.
- **Capital Invoicing**: Added HTML invoice generation module to easily format and send beautiful invoices upon successful payment.
- **Nexus Auto-Generation**: Introduced `#[derive(Nexus)]` macro to automatically introspect models and generate full CRUD administrative panels with zero configuration.
- **Nexus Data Tables**: Upgraded the admin panel tables to support server-side column sorting and searching (`sort_by`, `order`, `q`).
- **Nexus Data Formatting**: Improved the HTML form generation to automatically render booleans as toggle switches and enums as `<select>` dropdowns based on struct field types.
- **Capital Advanced Subscriptions**: Added native methods to `Billable` trait to cancel, pause, and report metered usage seamlessly.
- **Capital Team Billing**: Added support for organization-level billing. You can now scaffold subscriptions for `Team` or `Workspace` models via `cargo rullst make:billing --model Team`.
- **Capital Resource Quotas**: Added `tier_limit` and `check_quota` to the `Billable` trait to natively enforce tier-based resource constraints.
- **Capital Coupons & Trials**: Added `apply_coupon` and `extend_trial` methods for programmatic discount and trial management.
- **Capital Entitlements**: Added `can_access` to effortlessly check if a user has access to a tier-based feature.
- **Capital Scaffold**: Added `cargo rullst make:billing` to generate a beautiful Pricing Page, Webhooks, Checkout, and Customer Portal logic instantly.
- **Global Secret Scanning**: Integrated Trufflehog across the entire monorepo to prevent credential leaks in any crate.
- **Testing Foundations**: Scaffolded base integration tests for newly integrated crates (`rullst-auth`, `rullst-core`, `rullst-mail`, `rullst-capital`, `rullst-ai`) to ensure CI stability.

### Fixed
- **Duplicate ORM Initialization in Server Runtime (`rullst-core::server`)**: Added an idempotency check (`if rullst_orm::Orm::try_pool().is_ok() { return; }`) in `Server::init_database` to prevent redundant connection pool initialization and eliminate warning logs when user code initializes `Orm::init` before starting the server.
- **CI/CD Workflow Actions & Supply-Chain Hardening**:
  - Replaced unresolvable `dtolnay/rust-toolchain` and `actions/cache` commit SHAs with pinned, verified commits across `no_std-build.yml` and `iot-integration.yml`.
  - Pinned `FROM gcr.io/oss-fuzz-base/base-builder-rust` to an immutable SHA256 digest in `oss-fuzz/projects/rullst/Dockerfile` (Scorecard Pinned-Dependencies #200).
  - Integrated 5 Dependabot dependency updates (`github/codeql-action`, `taiki-e/install-action`, `codecov/codecov-action`, `actions/upload-pages-artifact`).
  - Added browser User-Agent headers and double-submit CSRF cookie/header forwarding in `e2e-smoke.yml` to satisfy production WAF and CSRF middleware rules.
- **Absolute Local Path Sanitation**: Purged all absolute local machine path references across repository markdown files, book tutorials, and metadata in favor of clean relative links.
- **Formal Verification (Kani) & State-Space Explosion**: Resolved SAT solver job cancellation (`The operation was canceled`) in `rullst-core` by optimizing unwinding bounds from `25` to `6` on string escaping and `5` on PII masking. Re-added `rullst-macros` to the Kani CI matrix.
- **Fuzzing Target (`fuzz_parser`) Compilation Error**: Fixed `error: an inner attribute is not permitted in this context` in `rullst-orm-macros/src/parser.rs` when included in fuzz targets via `include!`. Replaced `#![...]` inner attribute with outer module attribute `#[cfg_attr(mutants, mutants::skip)]`.
- **Mutation Coverage (`cargo-mutants`)**: Added unit test assertions and `mutants::skip` annotations across `rullst-capital`, `rullst-ai`, `rullst-connect`, `rullst-core`, `rullst-nexus`, and `cargo-rullst` CLI generators.
- **GitHub Actions Node.js 20 Deprecation Warnings**: Added `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` across all 23 workflow files in `.github/workflows/` to suppress runner deprecation warnings.
- **CI Test Reliability**: Fixed a persistent SQLite connection timeout (`PoolTimedOut`) during `llvm-cov` and parallel CI test runs in `feature_tests.rs`. Switched the test database from a shared-cache in-memory instance to a single-connection private memory database (`sqlite::memory:`) to completely eliminate file locking and connection pooling deadlocks on single-threaded Tokio test runtimes.

### Changed
- **Lockstep Versioning (Monorepo)**: Brought the entire Rullst ecosystem (`rullst-orm`, `rullst-connect`, `rullst-core`, `rullst-ai`, `rullst-auth`, etc.) into a unified Cargo Workspace Monorepo. All ecosystem crates will now share the same version number (`12.x.x`) to guarantee API compatibility and remove the dependency resolution nightmare for users.
- **Enterprise CI Architecture**: Completely overhauled the GitHub Actions pipeline for the Monorepo structure. Heavy security tasks (Miri, Kani, Fuzzing, Mutants) are parallelized via dynamic matrices but restricted to manual triggers to respect GitHub limits. Standard CI checks use aggressive caching to test all crates in under 10 minutes.
- **Dependabot**: Reduced update frequency to weekly to minimize PR spam.
- **Docker BuildKit & Nix Crane Caching**: Brought back Docker BuildKit caching (`DOCKER_BUILDKIT=1` and `sccache`) and Nix `crane` dependencies caching for the generated projects, significantly reducing Docker and Nix build times.
- **Unified Native Github Pages Deploy**: Consolidated all ecosystem websites (`Rullst`, `Rullst-Connect`, `Rullst-ORM` VitePress) under a single GitHub Actions workflow (`pages.yml`), dropping the legacy `gh-pages` branch for a cleaner repository history.

## [5.1.0] - 2026-07-27

### Added
- **Interactive Scaffolding AI Prompt**: `cargo rullst new` now prompts the developer whether they need Artificial Intelligence features (`rullst-ai`). Skipping this keeps the generated project leaner and compiles faster.
- **Modular Workspace Architecture**: The core monolithic `rullst` crate has been split into independent sub-crates (`rullst-core`, `rullst-auth`, `rullst-ai`, `rullst-capital`, `rullst-mail`, `rullst-nexus`, `rullst-studio`).
- **Optional Domain Modules**: `auth`, `ai`, `capital`, `mail`, `nexus`, and `studio` are optional facade features. `rullst-core` remains foundational, while ORM is enabled by default and can be disabled.

### Fixed
- **CLI Database Migrations**: Fixed a bug where `cargo rullst dash` would block indefinitely on projects that didn't have a database, as it waited for a `db:migrate` command that didn't exist in the project.
- **Blueprint Scaffolding**: Fixed an HTML macro syntax error in the `Blank Starter` blueprint generator caused by unescaped quotes, which could break compilation.
- **Feature Propagation**: Fixed a bug in `rullst` where enabling optional features (like `studio`, `telemetry`, `queue-redis`) failed to pass the flags down to `rullst-core`, causing `could not find module` compilation errors in generated projects.

## [5.0.1] - 2026-07-26 🚀

### Added
- **Documentation**: Created a comprehensive "Rullst CLI - Full Command Reference" at `docs/src/cli_reference.md`, detailing all subcommands, flags, and generators in English.
- **Formal Verification (Kani)**: Added mathematical safety proofs to the core framework to guarantee panic-free execution under all circumstances. Proved the memory-safety and output bounds of `html::escape_str` (XSS protection) and `security::mask_pii` (PII masking).
  - **v12 audited scope:** Kani explores only the explicit harnesses and bounds
    configured for those two helpers. It does not prove every runtime path,
    dependency, allocation, I/O failure or production deployment panic-free.
    Expanding focused harnesses is worthwhile; a universal guarantee is not.

### Fixed
- **Project Scaffolding**: Added missing SQLite database files (`*.db`, `*.sqlite`, `*.sqlite3`, `*.db-shm`, `*.db-wal`), IDE folders (`.vscode/`, `.idea/`), OS files (`.DS_Store`), and `.direnv/` (for Nix environments) to the default `.gitignore` template.
- **Docker Scaffolding**: Added missing `.env`, `.env.*`, `Foundry.toml`, `.DS_Store`, and SQLite database extensions to the default `.dockerignore` template to prevent sensitive secrets and local database state from leaking into production Docker images.

## [5.0.0] - 2026-07-25 🚀

### Added
- **Rullst Dev Dashboard (`cargo rullst dash`)**: Transformed the development server experience with a real-time Ratatui-powered visual dashboard. Splits the terminal into immersive panels displaying application logs and hot-reload system events simultaneously, complete with interactive shortcuts (e.g., `m` to run migrations). The classic textual dev server remains available via `cargo rullst dev` for CI/CD compatibility.
- **Hybrid Hot-Reloading (Dynamic Linking + AST Parsing)**: The ultimate DX revolution. For business logic changes, Rullst uses dynamic library hot-swapping (`dylib` / `.so`) to update the backend instantly. For frontend views (`html!` macros), the CLI intercepts changes, parses the AST, and squirts new HTML fragments over WebSockets to morphdom. The result? Sub-millisecond layout updates (like Vite/Dioxus) with a stateful Rust backend.
- **Native Reactive SSR (Rullst Live)**: Introduced `#[live_component]` and `#[live_event]` declarative macros. Developers can now build highly interactive, real-time WebSocket UIs (LiveView style) without writing JavaScript or WebAssembly. The macro automatically handles DOM diffing events and state syncing via HTMX.
- **Documentation Hub**: A premium VitePress documentation portal in `docs/` with dark mode aesthetics.
- **TypeScript SDK Generator (`generate:ts`)**: AST-based CLI command to dynamically generate typed `rullst-client.ts`.
- **Database-First Introspection (`generate:models`)**: Integrated the ORM schema introspection engine into the `cargo-rullst` CLI. Developers can now reverse-engineer live PostgreSQL, MySQL, and SQLite databases to automatically generate idiomatic Rust `#[derive(Orm)]` structs via `cargo rullst generate:models`.
- **Schema Visualizer (`generate:diagram`)**: Added `cargo rullst generate:diagram` to the CLI. It uses regex-based AST introspection to scan `.rs` files and generate a beautiful Mermaid ER diagram (`diagram.md`) visually mapping all relationships (`HasMany`, `BelongsTo`, etc.).
- **Ultra-Lightweight OpenTelemetry Core**: Integrated OTLP exporter behind a `telemetry` feature flag, keeping the framework lightweight by default.

### Breaking Changes
- **Rullst Connect v11.0.0 API**: Upgraded `rullst-connect` to `11.0.0`. Since `rullst-connect` is re-exported via `rullst::auth::connect`, developers using the `oauth` feature must adapt to any breaking changes.
- **Major Version Bump**: Rullst was upgraded to `5.0.0` to respect SemVer due to the `rullst-connect` API breaking changes.

### Changed
- **Dependencies Upgrade**: Updated all dependencies in the workspace to their latest stable compatible versions.
- **OpenTelemetry v0.32 API Upgrade**: Migrated `telemetry.rs` to use the new `SdkTracerProvider` and `Resource` builder patterns mandated by the `0.32.1` release.
- **Dependency Cleanups**: Pruned unused packages across `cargo-rullst`, `test1`, and examples using `cargo machete` for a leaner workspace.
- **Rust 1.97.1 Upgrade**: Upgraded Rullst internal development to use Rust 1.97.1, while keeping Rust 1.96.0 as the minimum supported rust version (MSRV) para os usuários, and updated internal Dockerfile templates.

### Security
- **SSH/SCP Option & Command Injection Defense**: Hardened `cargo-rullst` deployment generator (`foundry.rs`) by adding POSIX option delimiters (`--`) before destination arguments in `ssh` and `scp` invocations, enforcing strict character validation on upload binary names (`bin_name`), and validating environment variable keys and values to prevent newline or argument injection during remote server provisioning.
- **Supply Chain Security**: Pinned `softprops/action-gh-release` and `rust-lang/crates-io-auth-action` to their absolute commit SHAs in the release workflow to resolve Scorecard Pinned-Dependencies alerts.
- **Windows Shell Injection Mitigation**: Replaced unsafe `cmd /C npm` invocations with direct `npm.cmd` and `npx.cmd` binary executions in the `cargo-rullst` desktop generator to prevent potential command injection on Windows environments.
- **Vite & Dependabot Vulnerability Patch**: Updated `vite`, `esbuild`, and `launch-editor` in the documentation hub (`docs/package.json`) to eliminate High and Moderate security alerts (path traversal and NTLMv2 hash disclosure bypasses).
- **Zero-Panic Enforcement in Server Router**: Replaced `unwrap()` calls with graceful error handling `unwrap_or_else` in `rullst/src/server.rs` HotSwapService to satisfy `-D clippy::unwrap_used` constraints.

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
- **Native Security Matrix (CI/CD)**: Expanded the CI pipeline with Rust-native security tooling. Each job has a bounded scope and does not certify the framework or a deployment.
  - Added **cargo-deny** to ban unapproved licenses and vulnerable dependencies.
  - Added **OSSF Scorecards** to establish a public, enterprise security score.
  - Added **OWASP ZAP** DAST pipeline to proactively attack generated SaaS blueprints in real-time.
    - Updated the `blank` blueprint generation to natively include `headers_middleware`, ensuring all new projects pass DAST scanning out-of-the-box.
  - Added **cargo-tarpaulin** for native, terminal-based code coverage reporting within PRs.
  - Showcased GitHub Actions badges in the `README.md`.
  - Added **cargo-mutants** to enforce test suite quality via deliberate mutation injections.
  - Added **cargo-fuzz** with an initial target (`mask_pii`) to guarantee DoS immunity against malformed byte sequences.
    - **v12 audited scope:** The target exercises `mask_pii` against generated
      inputs for the duration and corpus of a particular run. It can reveal
      crashes or pathological inputs but cannot guarantee DoS immunity. Keep and
      expand bounded fuzz targets with time, corpus and commit recorded.
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
- **Security Validation**: Reviewed reported CLI command-injection, hot-reload `unsafe`, and uptime-scaffold findings and recorded the resulting fixes or rationale. This was not a comprehensive security score.

### CLI & Tooling
- **Docker Cache Bugfix**: Fixed an issue in `cargo rullst dockerize` and `--docker` scaffolding where Docker's `mtime` caching behavior would cause Cargo to skip compilation of `.rs` files after building dependencies, resulting in empty binaries that exited with code 0.
- **Lean Core Refactor**: Completely removed the internal `rullst-press` SSG tool from the framework workspace and CLI menu. Rullst is now strictly focused on backend/fullstack productivity, and the main documentation has migrated to a dedicated modern SPA site.
- **Clippy Strict Compliance**: Re-audited and passed `cargo clippy --workspace --all-targets --all-features -- -D warnings`, resolving a stray `clippy::useless_vec` warning in the interactive menu.

## [2.0.4] - 2026-06-09 🔒

### Security & Stability

- **Zero-Panic Policy Enforcement (P1)**: Replaced an `unwrap()` call inside the Nexus Basic Auth middleware with a fallible response path. Repository policy checks, rather than this isolated fix, define the reviewed scope.
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

- **`AUDIT.md`**: Added a security and architecture review. The checked-in document is now a reproduction guide rather than a PASS certificate; current advisory exceptions are maintained with owners and expiry in `docs/src/security-advisory-exceptions.md`.

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
- **Documentation Coverage Baseline**: Enabled `#![warn(missing_docs)]` across the main library and seeded missing API documentation. The lint is a baseline, not proof that every document is complete or accurate.

## [2.0.1] - 2026-06-03 🐛

### Changed
- **CLI Upgrades**: Improved the `cargo rullst` CLI wizard hints and simplified the dev server startup message.
- **Blueprint Fixes**: Fixed `routes!` syntax (`get("/" => handler),`) and `html!` macro syntax (`required="true"`) that caused compilation errors in newly generated ERP and Uptime Monitor projects.
- **Blueprint Resilience**: Added a 3-second initialization delay to background workers to completely prevent SQLx `Orm must be initialized before querying` panics on startup.
- **Design Standardization**: Updated all 5 starter blueprints to strictly use the Rullst branding colors (Emerald Green `emerald-500` and Orange `orange-500`) instead of generic blues and purples.
- **RullstPress Engine**: Completely rewrote the Rullst documentation using the internal SSG Engine, providing accurate tutorials for the new interactive CLI.

## [2.0.0] - 2026-06-01 🚀

### Historical deep-audit milestone (superseded)
- **Audit Follow-Up**: Addressed the findings tracked by that historical review. Scores and absolute certification language are intentionally not carried forward; current CI artifacts and the SST define the supported state.
- **Studio SQL Security**: Hardened SQL identifier sanitization with strict 64-character length limits to prevent buffer exhaustion.
- **HTML Macro Zero-Allocation**: The `html!` compile-time macro now pre-computes static AST sizes and injects `String::with_capacity(STATIC_SIZE)` for maximum memory efficiency.
- **AI-Native Maintainability**: Created standard `AGENTS.md` and `.ai-rules` files to govern AI tooling workflows securely.
- **Async I/O Optimization**: Refactored `RedisDriver::flush` cache pruning to utilize a single batched `DEL` roundtrip, eliminating event-loop blocking from sequential iterators.
- **Complex View Engine Sanitization**: Added strict HTMX-safe validation and encoding checks for complex Javascript data types mapped to HTML strings.
- **AWS S3 Disablement**: Deactivated the `storage-s3` feature and removed its AWS SDK dependency path. This narrows the dependency graph but does not prove the entire workspace vulnerability-free.

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
  - **v12 audited scope:** This records the dependency update performed for that
    historical version. “Latest” is time-dependent and does not guarantee
    security; the current lockfile, advisory scan, source policy and governed
    exceptions define the evidence for a particular release commit.
- **Rng Stability & rand_core Resolution**: Resolved version conflicts between `rand_core` versions and removed the direct explicit dependency from the facade. Password hashing remains fallible and reports typed errors.

### Community Health
- **Community Standards**: Added `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant).
- **Issue Templates**: Added structured GitHub templates for Bug Reports and Feature Requests.
- **PR Checklists**: Added strict `PULL_REQUEST_TEMPLATE.md` to ensure code quality prior to review.

## [1.0.10] - 2026-05-29 🛡️

### Security & Quality Audits (10/10 Milestone)
- **Historical Audit Follow-Up**: Recorded the checks performed for that release. It is not a current certification; reproduce the present repository gates before relying on it.
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
- **SQLite Replication Configuration Preview**: Added configuration types, but no real synchronization backend. The current manager returns `Unsupported` rather than reporting simulated success.
- **Non-Intrusive Background Version Checker**: Implemented a background crates.io version updater in the `cargo-rullst` CLI that runs on a spawned thread and caches version status under the OS temporary directory (`rullst_version_cache.txt`). The network fetch is limited to at most once per day, ensuring 0ms impact on developer terminal execution speeds.
- **Terminal Update Banner**: Visual, colored terminal banner rendered at CLI tool exit when a newer version is cached, prompting users to upgrade.
- **Self-Healing CLI `upgrade` Codemods**: Refactored the `cargo rullst upgrade` command into a full autonomous refactoring pipeline: automatically updates `Cargo.toml` dependency tags to the latest release, runs search-and-replace codemods across `src/**/*.rs` to patch legacy APIs and enforce dependency shielding automatically, and runs validation compilation checks (`cargo check`) as a final quality gate.
- **Dependency Shielding Abstraction cascades**: Encapsulated transitive external dependencies into secure modular namespaces within Rullst core's public API: `rullst::db` (wrapping `sqlx`, `rullst_orm`), `rullst::web` (wrapping `axum`, `tower`, `tower_http`), `rullst::async_runtime` (wrapping `tokio`), and `rullst::email_client` (wrapping `lettre`). This isolates downstream applications from external breaking changes.
- **Resilient Traffic Shielding & Adaptive Backpressure**: Introduced a router-level load shielding and backpressure system, now maintained in [`rullst-core/src/resilience.rs`](rullst-core/src/resilience.rs), that monitors Tokio event-loop lag, an optional low-frequency database probe, and active requests. The middleware returns `503 Service Unavailable` under configured critical thresholds and applies a bounded delay under moderate load; it reduces overload pressure but cannot guarantee prevention of resource exhaustion.
- **Token-Bucket Rate Limiter**: Added a thread-safe, atomic rate limiting system powered by a concurrent Shared-Memory (`DashMap`) engine. Features a highly customizable `RateLimitConfig` constructed with the Builder Pattern for strict backward-compatibility, and includes convenient factory builders (`per_second`, `per_minute`, `per_hour`). Seamlessly handles proxy environments by resolving client identifiers through standard headers (`X-Forwarded-For`, `X-Real-IP`) and peer addresses (`ConnectInfo`).
- **Edge-Optimized Assets & Pre-Compression (Brotli + Zstandard)**: Implemented an advanced high-performance pre-compression pipeline within the `cargo-rullst` CLI tool (`cargo rullst build [--debug]`) that recursively compiles the production binary and compresses all text-based static assets (HTML, CSS, JS, SVG, JSON, WASM, TXT, XML) in the `static/` directory using **Brotli (level 11)** and **Zstandard (level 19)** formats, saving `.br` and `.zst` files alongside their original sources. Upgraded the Rullst core library static asset serving (`ServeDir::new("static")`) inside `rullst/src/server.rs` to support pre-compressed Brotli served natively, and integrated a fast zero-overhead rewriting middleware `zstd_static_middleware` that intercepts client requests, checks for `Accept-Encoding: zstd`, rewrites the request URI to `.zst` zero-copy if the file is present, and overrides proper `Content-Encoding: zstd` and mime-specific `Content-Type` headers for blazing-fast edge-optimized transfers.
- **Native WebAuthn Building Blocks (Passkeys & Biometrics)**: Added WebAuthn ceremony parsing and signature verification (`rullst::auth::passkey`) backed by `ring`, plus negative invariant tests and CLI scaffolding for registration/sign-in flows. Deployments must persist challenges and counters atomically, configure origins/RP IDs correctly, and complete their own security review; this entry is not a WebAuthn certification.
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
