# Cargo feature matrix

> [!IMPORTANT]
> Dependency examples target `12.1.0`. Check the [release record](v12.md)
> for publication status and commit Cargo.lock for reproducible builds.
> Use path dependencies only when intentionally testing checkout-local changes.

This page is the public feature contract for the 20 packages in the v13
[release inventory](../../.github/release-order.json): the 16 published 12.x
packages plus the unpublished `rullst-privacy`, `rullst-supervision`,
`rullst-media` and `rullst-labs` candidates. The package manifests remain the machine-readable
source of truth. The matrix explains the behavior those names select in this
v13 source and makes the default build visible before an application adopts
optional integrations.

Cargo features are additive across a dependency graph. An application can
disable a package's defaults at the dependency edge, but it cannot disable a
feature enabled by another dependency. Inspect the final selection with:

```bash
cargo tree -e features
```

The release gates compile every package with no default features, every public
umbrella feature in isolation, representative domain-package boundaries, and
the complete workspace with all features. The isolated umbrella list, and this
page's umbrella table and default features, are checked automatically against
`rullst/Cargo.toml`, so a newly added public feature cannot silently escape the
matrix. See
[`check-feature-boundaries.sh`](../../.github/check-feature-boundaries.sh) for
the exact individual checks.

## Unpublished v13 privacy additions

The v13 candidate adds `rullst-privacy` as an optional release package. These
features are absent from published 12.x releases; use the matching v13
source until its package and release admission complete. They do not change
default dependencies or select application policies automatically.

| Umbrella feature | Standalone privacy feature | Contract |
| :--- | :--- | :--- |
| `privacy` | None | Independent empty base |
| `privacy-age` | `age-assurance` | Proportional age-policy contracts |
| `privacy-challenge-tokens` | `challenge-tokens` | Authenticated server challenge transport |
| `privacy-sqlite` | `sqlite` | Shared-local age replay protection |
| `privacy-postgres` | `postgres` | Age replay protection on one authoritative PostgreSQL database |
| `privacy-consent` | `consent` | Purpose/version choices and effective withdrawal |
| `privacy-consent-sqlite` | `consent-sqlite` | Shared-local consent state |
| `privacy-consent-postgres` | `consent-postgres` | New candidate for shared consent across application hosts; independent of age assurance |

See the [privacy package guide](../../rullst-privacy/README.md) for initialization,
runtime roles, data minimization, backup/failover obligations and acceptance.

The local email-login candidate adds Auth `email-login-sqlite` and
`email-login-postgres`, with facade `auth-email-login-sqlite` /
`auth-email-login-postgres`. These optional paths reuse authoritative recovery
accounts and opaque sessions; they do not enable email login on accounts or
replace tenant/MFA policy. Hosted source/package admission remains pending; see
[the email-login contract](email-login.md).

The scoped API-token candidate adds Auth `api-tokens-sqlite` /
`api-tokens-postgres` and facade `auth-api-tokens-sqlite` /
`auth-api-tokens-postgres`. They enable the corresponding recovery backend only;
email login, JWT and OAuth remain independent. See the
[API-token contract](api-tokens.md) for current admission and boundaries.

## Umbrella crate: `rullst`

The unpublished private-storage candidate adds Core/facade `storage-multipart`,
which selects `storage-s3`, bounded XML parsing and checkpoint key zeroization.
It enables no database, queue or default feature. See
[private multipart uploads](private-multipart-uploads.md) for server-mediated
parts, encrypted resumable checkpoints, cleanup responsibilities and admission.

The default `rullst` dependency enables `orm`, `drivers-all` and
`queue-sqlite`: the ORM with the SQLite, PostgreSQL and MySQL/MariaDB SQLx
drivers, and Core's durable SQLite queue. Applications that only need the HTTP
runtime can opt out:

```toml
[dependencies]
rullst = { version = "12.1.0", default-features = false }
```

An application that disables defaults and keeps the ORM must also select
`drivers-all` or one `strict-*` backend. `orm` itself adds no SQLx driver: it
compiles, but `Orm::init` and `Server::with_db` then fail at runtime for every
URL scheme whose driver no other enabled feature happens to add (`queue-sqlite`,
for example, adds only SQLite).

| Feature | Default | Enables |
| --- | :---: | --- |
| `orm` | yes | `rullst-orm` and Core's ORM integration, without a SQLx driver; pair it with `drivers-all` or one `strict-*` backend |
| `drivers-all` | yes | `orm` plus the SQLite, PostgreSQL and MySQL/MariaDB SQLx drivers in the ORM and Core, and in Studio and Nexus when `studio` or `nexus` is enabled |
| `orm-mongodb` | no | `orm` plus the MongoDB document adapter |
| `orm-duckdb` | no | `orm` plus the in-process DuckDB analytics adapter |
| `orm-turso` | no | `orm` plus typed Turso-primary CRUD/query, parameterized remote libSQL SQL over Hrana HTTP v3, transactions, reversible checked migrations, and a persistent offline fallback |
| `orm-surrealdb` | no | `orm` plus SurrealDB HTTP document and bounded graph adapters |
| `orm-scout` | no | `orm` plus bounded Meilisearch, Elasticsearch and Algolia Scout HTTP adapters |
| `orm-pgvector` | no | `orm` plus typed pgvector SQLx values; use with `strict-postgres` for the supported live query contract |
| `orm-qdrant` | no | `orm` plus bounded dense-vector Qdrant HTTP operations and offline fallback |
| `orm-redis` | no | `orm` plus namespaced Redis Hash, Set and Sorted Set operations |
| `orm-polyglot` | no | Convenience feature enabling MongoDB, DuckDB, Turso, SurrealDB and Qdrant adapters |
| `queue-sqlite` | yes | Core's durable SQLite queue backend |
| `storage-s3` | no | v13 candidate: Core's bounded AWS S3 and Cloudflare R2 object operations and signed GET URLs; see [private object storage](private-object-storage.md) |
| `storage-multipart` | no | v13 candidate: `storage-s3` plus server-mediated multipart uploads with bounded XML parsing and checkpoint key zeroization; see [private multipart uploads](private-multipart-uploads.md) |
| `nexus` | no | The generated Nexus administration interface |
| `studio` | no | Studio plus Core's Studio integration marker |
| `auth` | no | Authentication, sessions, passkeys, and RBAC helpers from `rullst-auth` |
| `auth-jwt` | no | `auth` plus the strict application-issued JWT policy |
| `auth-sqlite` | no | `auth-jwt` plus bounded shared SQLite JWT revocation and passkey device lifecycle state |
| `auth-passkey-postgres` | no | Optional v13 account/session-bound PostgreSQL passkey ceremony candidate; host credential-counter CAS remains required |
| `auth-sessions-sqlite` | no | v13 candidate: `auth` plus the SQLite account registry, opaque sessions and session inventory (Auth `recovery-sqlite`); see [session management](session-management.md) |
| `auth-sessions-postgres` | no | v13 candidate: `auth` plus the PostgreSQL account registry, opaque sessions and session inventory (Auth `recovery-postgres`) |
| `auth-email-login-sqlite` | no | v13 candidate: `auth` plus email login over the SQLite recovery accounts and sessions; see [the email-login contract](email-login.md) |
| `auth-email-login-postgres` | no | v13 candidate: `auth` plus email login over the PostgreSQL recovery accounts and sessions |
| `auth-api-tokens-sqlite` | no | v13 candidate: `auth` plus scoped API tokens over the SQLite recovery backend; see [the API-token contract](api-tokens.md) |
| `auth-api-tokens-postgres` | no | v13 candidate: `auth` plus scoped API tokens over the PostgreSQL recovery backend |
| `account-mail-sqlite` | no | `auth` and `mail` plus the SQLite recovery store and the `account_mail` delivery bridge; see [account mail](account-mail-v12-1.md) |
| `account-mail-postgres` | no | `auth` and `mail` plus the PostgreSQL recovery store and the `account_mail` delivery bridge |
| `mail` | no | `rullst-mail` with HTTP/offline transports and no SMTP dependency |
| `mail-sqlite` | no | `mail` plus bounded shared-local SQLite recipient suppression and provider-event replay evidence |
| `mail-postgres` | no | v13 candidate: shared PostgreSQL suppression with keyed identifiers, authoritative dispatch checks and independent quotas |
| `mail-smtp` | no | `mail` plus the optional SMTP transport |
| `mail-aws-ses` | no | `mail` plus native SES v2 delivery signed by the official AWS SDK |
| `messaging` | no | Native bounded broker-neutral messaging contracts and the deterministic process-local broker |
| `messaging-sqlite` | no | `messaging` plus fixed-schema durable local SQLite publication, lease, retry/DLQ, ACK and idempotency state |
| `messaging-schedules-postgres` | no | v13 candidate: encrypted PostgreSQL recurring-publication outbox and fenced relay into an explicitly selected broker; no default SQLite/ORM |
| `messaging-webhooks` | no | v13 candidate: encrypted shared-local SQLite outgoing outbox, immutable HTTPS destination, signed exact bytes, fenced retries and minimized terminal inspection |
| `messaging-orm-outbox` | no | `messaging` and `orm` plus the static relational outbox-to-broker relay; the publish/ACK crash window remains at-least-once |
| `messaging-redis` | no | v13 candidate: `messaging` plus the Redis Streams broker with fenced delivery indexes; enables no ORM, mail, cache or Core queue; see [Redis Streams messaging](redis-messaging.md) |
| `privacy` | no | v13 candidate: the independent empty `rullst-privacy` base; the `privacy-*` features are listed in [the privacy additions](#unpublished-v13-privacy-additions) |
| `privacy-age` | no | v13 candidate: `privacy` plus proportional age-policy contracts |
| `privacy-challenge-tokens` | no | v13 candidate: `privacy-age` plus authenticated server challenge transport |
| `privacy-sqlite` | no | v13 candidate: `privacy-age` plus shared-local age replay protection |
| `privacy-postgres` | no | v13 candidate: `privacy-age` plus age replay protection on one authoritative PostgreSQL database |
| `privacy-consent` | no | v13 candidate: `privacy` plus purpose/version choices and effective withdrawal |
| `privacy-consent-sqlite` | no | v13 candidate: `privacy-consent` plus shared-local consent state |
| `privacy-consent-postgres` | no | v13 candidate: `privacy-consent` plus shared PostgreSQL consent state |
| `mailer` | no | Compatibility alias for `mail-smtp`; prefer `mail-smtp` in new manifests |
| `queue-redis` | no | Redis dependency and Core's Redis queue backend |
| `cache-redis` | no | Redis dependency and Core's Redis cache backend |
| `redis` | no | Convenience alias enabling `queue-redis`, `cache-redis` and `orm-redis` |
| `offline-sync` | no | Native bounded offline queue, explicit conflict state machine, account-bound encrypted snapshots, and static-dispatch push/pull orchestration; platform storage and concrete transport remain application responsibilities |
| `oauth` | no | OAuth2/OIDC providers from `rullst-connect` |
| `oauth-sqlite` | no | `oauth` plus bounded encrypted shared-local token-generation state with exact SQLite compare-and-swap |
| `ai` | no | Provider-agnostic AI clients and local safeguards from `rullst-ai` |
| `ai-sql-memory` | no | `ai` plus tenant-aware durable chat memory for SQLite, PostgreSQL, MySQL, and MariaDB |
| `capital` | no | Payment, payout, analytics, DPS builder, and offline fiscal APIs from `rullst-capital` |
| `capital-actix` | no | `capital` plus the Actix Web adapter for the canonical signed-webhook verifier |
| `capital-quota-sql` | no | `capital` and `orm` plus atomic shared resource quotas for SQLite, PostgreSQL, MySQL, and MariaDB |
| `capital-webhook-sql` | no | `capital` and `orm` plus bounded durable webhook replay/event claims for SQLite, PostgreSQL, MySQL, and MariaDB |
| `capital-nfse` | no | `capital` plus checksum-pinned official XSD validation, PKCS#12 XMLDSig, signed-environment protocol binding, authenticated local command journal, and rustls mTLS preparation |
| `capital-pdf` | no | `capital` plus bounded validated native invoice PDF rendering |
| `capital-mail` | no | `capital-pdf` plus Mail's payment-bound HTML/PDF attachment delivery bridge |
| `security` | no | RASP/WAF and application-security primitives from `rullst-security` |
| `security-redis` | no | `security` plus the atomic Redis rate limiter |
| `iot` | no | IoT models, frame helpers, and signed OTA verification from `rullst-iot` |
| `telemetry` | no | OpenTelemetry dependencies and Core's OTLP integration |
| `strict-postgres` | no | `orm` with the concrete PostgreSQL pool/backend selected |
| `strict-mysql` | no | `orm` with the concrete MySQL pool/backend selected when PostgreSQL is not also selected |
| `strict-sqlite` | no | `orm` with the concrete SQLite pool/backend selected when PostgreSQL and MySQL are not also selected |

The three `strict-*` backend features are supported as single selections;
each enables only its own SQLx driver. Feature unification can activate more
than one; the current deterministic precedence is PostgreSQL, then MySQL, then
SQLite. Do not depend on that precedence as backend negotiation. Select one
strict backend in an application, or select none and keep `drivers-all` to use
SQLx `Any` with all three drivers.

### Shared-local SQLite composition profile

A bounded single-host application may compose the following umbrella features
over one file-backed SQLite URL:

```toml
[dependencies]
rullst = { version = "12.1.0", default-features = false, features = [
  "auth-sqlite",
  "capital-quota-sql",
  "mail-sqlite",
  "messaging-sqlite",
  "oauth-sqlite",
  "queue-sqlite",
] }
```

The stores use distinct fixed table namespaces. Initialize/check them
sequentially, keep application readiness false until every required component
is healthy, and reuse exactly the same URL, quotas, namespaces, and keys after
restart. Close every handle before file-level backup or restore. The dedicated
[`facade_recovery` test](../../rullst/tests/facade_recovery.rs) proves restart,
idempotency, encrypted-secret plaintext absence, queue recovery, aggregate
readiness, and isolated fail-closed corruption for this exact profile.

Sharing the file is not a cross-domain transaction or a multi-host design. The
host still owns permissions, encryption-key custody, a consistent whole-file
backup procedure, recovery drills, contention policy, and domain
authorization. Prefer separate databases when failure isolation or write
throughput is more important than simple local operation.

## Runtime and data crates

### `rullst-core`

Default features: none.

| Feature | Enables |
| --- | --- |
| `orm` | Optional `rullst-orm` and SQLx support, including Artisan and database-backed feature flags; it adds no SQLx driver |
| `drivers-all` | `orm` plus the ORM's SQLite, PostgreSQL and MySQL/MariaDB drivers |
| `queue-sqlite` | SQLx-backed durable SQLite queues without enabling the full ORM facade |
| `queue-redis` | Redis-backed queues |
| `cache-redis` | Redis-backed cache storage |
| `redis` | Convenience alias for both Redis queue and cache backends |
| `offline-sync` | Native bounded offline state, AES-256-GCM snapshots, and timeout/budget/cursor-checked transport orchestration; excludes platform storage and a concrete authenticated transport |
| `storage-s3` | v13 candidate: bounded AWS S3 and Cloudflare R2 object operations and signed GET URLs |
| `storage-multipart` | v13 candidate: `storage-s3` plus server-mediated multipart uploads with bounded XML parsing and checkpoint key zeroization |
| `studio` | Integration marker used by the umbrella Studio boundary; it adds no dependency by itself |
| `telemetry` | OpenTelemetry tracing and OTLP export dependencies |
| `strict-postgres` | `orm` plus the ORM PostgreSQL backend selection |
| `strict-mysql` | `orm` plus the ORM MySQL backend selection |
| `strict-sqlite` | `orm` plus the ORM SQLite backend selection |

Core's process-local Radar and span collector do not require `telemetry`.
That feature is specifically for OpenTelemetry/OTLP integration.
The `rullst.client` v1 codec and bounded `#[server_function]` transport are
available without a feature flag; only the explicit generated route exists on
native targets, while the same annotated function becomes its Wasm caller.
Identity, authorization and tenant policy remain application layers.

### `rullst-orm`

Default feature: `drivers-all`. With no `strict-*` feature, public pool and
database aliases use SQLx `Any`, which opens only the drivers compiled in. The
workspace and the umbrella crate depend on the ORM with `default-features =
false` and forward `drivers-all` or a `strict-*` backend explicitly.

| Feature | Enables |
| --- | --- |
| `drivers-all` | The SQLite, PostgreSQL and MySQL/MariaDB SQLx drivers |
| `redis` | Redis query cache plus bounded namespaced Hash, Set and Sorted Set datastore operations |
| `mongodb` | Official MongoDB driver plus typed document CRUD, identifier inventory, encrypted recovery participation and offline fallback |
| `duckdb` | Bundled DuckDB client plus parameterized, bounded analytics queries |
| `turso` | Direct official Hrana HTTP v3 transport, typed primary CRUD/query facade, parameterized SQL, atomic batches, reversible checksummed migrations, and a persistent SQLite-compatible offline fallback |
| `surrealdb` | SurrealDB HTTP document CRUD, identifier inventory, encrypted recovery participation and bounded read-only ISO GQL; no embedded SDK |
| `scout-http` | Bounded Meilisearch, Elasticsearch and Algolia adapters with deterministic offline fallbacks; Meilisearch also has a live container contract |
| `pgvector` | Typed pgvector SQLx values and parameterized L2/cosine/inner-product helpers; the live contract also selects `strict-postgres` |
| `qdrant` | Bounded dense-vector collection/upsert/delete/cosine query operations over HTTP with offline fallback |
| `polyglot` | Convenience feature enabling `mongodb`, `duckdb`, `turso`, `surrealdb`, and `qdrant` |
| `ai` | `pgvector` plus the generated `save_with_embedding` for `#[orm(embedding_for = "...")]` models; the application also depends on `rullst-ai` |
| `strict-postgres` | The PostgreSQL driver plus concrete PostgreSQL pool, database, query-result, and query paths |
| `strict-mysql` | The MySQL driver plus concrete MySQL paths when PostgreSQL is not also selected |
| `strict-sqlite` | The SQLite driver plus concrete SQLite paths when PostgreSQL and MySQL are not also selected |

The strict backend selection rules and precedence are the same as the umbrella
crate. Each `strict-*` feature enables its own SQLx driver and selects concrete
public types and query paths; disable the ORM defaults to keep the other
drivers out of the graph.

The Polyglot features expose capability-specific APIs under
`rullst_orm::polyglot`; they do not participate in a shared cross-backend
transaction. The base deterministic document store and the MongoDB/SurrealDB
adapters implement `DocumentInventory` for an application-operated bounded
snapshot/restore contract; it does not supply online isolation or managed
backup. Turso can additionally be selected explicitly by
`#[orm(backend = "turso")]` and the blank/API scaffold. See the
[Polyglot Persistence guide](polyglot-persistence.md).

### `rullst-orm-macros`

Default features: none. `#[derive(Orm)]` uses a fail-closed structured parser:
unknown/duplicate options, missing persisted targets, conflicting relations,
unsafe identifiers, and SQLx mappings that generated persistence cannot honor
are compile errors. The exact derive grammar and its raw soft-delete-expression
boundary are defined in the packaged crate README and the
[SST](spec.md#51-model-definition--crud).

| Feature | Enables |
| --- | --- |
| `runtime-driver-codecs` | Enum codecs follow the ORM runtime's selected SQLx drivers; `rullst-orm` enables it |
| `runtime-feature-gates` | v13 candidate: Redis and embedding APIs are emitted or omitted at expansion time from the forwarded `redis`/`ai` features instead of the application's own features; `rullst-orm` enables it |
| `redis` | v13 candidate: forwarded by `rullst-orm/redis`; emits the Redis cache, hash and event code under `runtime-feature-gates` |
| `ai` | v13 candidate: forwarded by `rullst-orm/ai`; emits `save_with_embedding` under `runtime-feature-gates` |
| `strict-postgres` | Compatibility marker matching the ORM backend vocabulary; no macro expansion changes |
| `strict-mysql` | Compatibility marker matching the ORM backend vocabulary; no macro expansion changes |
| `strict-sqlite` | Compatibility marker matching the ORM backend vocabulary; no macro expansion changes |

### `rullst-connect`

Default features: none. Provider clients and framework-independent OAuth/OIDC
types remain available without a web-framework adapter.

| Feature | Enables |
| --- | --- |
| `axum` | Axum callback extractors and the local mock IdP router |
| `actix` | Actix Web callback extractors |
| `leptos` | Framework-independent callback extractor module for Leptos integration; no Leptos runtime dependency |
| `rullst` | Convenience integration boundary that enables `axum` |
| `retry` | Retry-aware HTTP client behavior using `reqwest-middleware` and `reqwest-retry` |
| `reqwest-middleware` | The optional middleware dependency alone; prefer `retry` for retry behavior |
| `axum-session` | Axum plus a ten-minute, one-active-challenge `tower-sessions` state/PKCE/OIDC-nonce transaction and callback extractor |
| `sqlite` | File-backed shared-local encrypted token snapshots with persisted quota, restart recovery and exact generation compare-and-swap; remote refresh leases, key custody and multi-host operation remain application concerns |
| `mock` | Deterministic offline provider modules outside test builds |

### `rullst-messaging`

Default features: none. The deterministic process-local broker, versioned
envelope, idempotency, consumer groups, leases, retry, dead-letter, and purge
contracts are available without optional dependencies. The only remote broker
adapter is the unpublished v13 Redis Streams candidate; other remote brokers
are not implemented and have no placeholder features.

| Feature | Enables |
| --- | --- |
| `sqlite` | Fixed-schema durable local broker with serialized SQLite writes and immutable plaintext or explicit AES-256-GCM content profiles; restart/corruption/rotation/tamper/two-instance evidence is local, while metadata visibility, key custody and remote replication/failover remain explicit boundaries |
| `orm-outbox` | Static bridge from the relational `rullst-orm` outbox to one configured broker topic, with exact replay after the publish-before-ACK crash window; worker operations and remote atomicity remain application boundaries |
| `redis-streams` | v13 candidate: standalone Redis Streams broker with Rullst-owned fenced delivery indexes; see [Redis Streams messaging](redis-messaging.md) |
| `schedules-postgres` | v13 candidate: encrypted PostgreSQL recurring-publication outbox and fenced relay into a `MessageBroker`; enables neither SQLite nor ORM |
| `webhooks` | v13 candidate: `sqlite` plus one immutable approved HTTPS destination, HMAC-SHA256 signing, bounded retry/dead-letter state, cancellation and terminal retention |

### `rullst-iot`

Default feature: `std`.

| Feature | Enables |
| --- | --- |
| `std` | Standard-library support in serialization and Ed25519 dependencies; disabling it makes the crate `no_std` + `alloc` |
| `experimental-simulators` | Deterministic MQTT formatting, HSM, and PQC fixtures; not live transports, hardware-backed keys, or production PQC |

### `rullst-capital`

Default feature: `axum`.

| Feature | Enables |
| --- | --- |
| `axum` | Axum middleware for the canonical bounded signed-webhook verifier |
| `actix` | Actix Web middleware for the same verifier; it does not enable Axum when selected directly |
| `quota-sql` | Durable idempotent shared quota accounting over SQLite, PostgreSQL, MySQL, and MariaDB; schema setup/migrations and authoritative membership/tier state remain application-owned |
| `webhook-sql` | Bounded durable provider-scoped payload/event claims over SQLite, PostgreSQL, MySQL, and MariaDB, including a caller-owned transaction path; cross-system effects and reconciliation remain application-owned |
| `nfse` | Checksum-pinned official XSD validation, PKCS#12 RSA-SHA256 XMLDSig, signed-`tpAmb` binding, deterministic GZip/Base64 issuance JSON, bounded signed-authorization/rejection parsing, a HMAC-chained single-writer local command journal, and rustls mTLS preparation; it does not enable live SEFIN transmission, provide a distributed outbox/retry engine, or establish certificate trust/homologation |
| `invoice-pdf` | Bounded paginated A4 invoice PDF with embedded WinAnsi or a validated caller-supplied TTF/OTF; payment/mail orchestration is separate |

### `rullst-mail`

Default features: none. HTTP mail providers remain available without SMTP.

| Feature | Enables |
| --- | --- |
| `mail-smtp` | Lettre-based SMTP transport |
| `aws-ses` | Official AWS SES v2 SDK, regional SigV4, temporary/rotating credential providers and native attachments/CID; AWS account readiness and inbox delivery remain external |
| `capital-invoice` | Capital's native invoice PDF plus the final-payment-bound delivery bridge; durable outbox claiming remains application-owned |
| `sqlite` | File-backed shared-local suppression state with exact provider-event replay binding and immutable quotas; webhook authentication, encryption and multi-host replication remain application-owned |
| `postgres` | v13 candidate: namespaced suppression on one authoritative writable PostgreSQL database; independent of SQLite. See [shared mail suppression](shared-mail-suppression.md) for initialization, runtime grants, retention and pending admission |

### `rullst-auth`

Default features: none.

| Feature | Enables |
| --- | --- |
| `oauth` | Optional `rullst-connect` OAuth2/OIDC integration and re-exports |
| `jwt` | Application-issued JWT claims, key rotation, and revocation-store policy |
| `sqlite` | `jwt` plus bounded file-backed shared JWT revocation and passkey device lifecycle state |
| `recovery-sqlite` | SQLite account registry, recovery and opaque sessions (`SqlRecoveryStore`); the v13 candidate adds session inventory/logout |
| `recovery-postgres` | The same recovery store on PostgreSQL |
| `passkey-postgres` | v13 candidate: shared account/session-bound PostgreSQL passkey ceremonies |
| `email-login-sqlite` | v13 candidate: `recovery-sqlite` plus email login over its accounts and sessions; see [the email-login contract](email-login.md) |
| `email-login-postgres` | v13 candidate: `recovery-postgres` plus email login |
| `api-tokens-sqlite` | v13 candidate: `recovery-sqlite` plus scoped API tokens; see [the API-token contract](api-tokens.md) |
| `api-tokens-postgres` | v13 candidate: `recovery-postgres` plus scoped API tokens |

The umbrella crate exposes `jwt` and `sqlite` as `auth-jwt` and `auth-sqlite`
(`auth-sqlite` also enables `auth-jwt`), `recovery-*` as `auth-sessions-*`,
`passkey-postgres` as `auth-passkey-postgres`, and the email-login and
API-token features as `auth-email-login-*` and `auth-api-tokens-*`; each
enables `auth`. `account-mail-*` also selects `recovery-*`. The umbrella does
not forward Auth's `oauth`; its own `oauth` feature adds `rullst-connect`
directly.

### `rullst-security`

Default features: none.

| Feature | Enables |
| --- | --- |
| `redis-rate-limit` | Atomic namespaced Redis fixed-window limiter plus its explicit offline mock mode; CI/release run the independent-client contract against a digest-pinned Redis service |

The umbrella crate exposes this as `security-redis`, which also enables
`security`.

### `rullst-ai`

Default features: none. Provider clients, prompt inspection and PII masking
are always available.

| Feature | Enables |
| --- | --- |
| `sql-memory` | `SqlChatMemory`: tenant-aware durable chat memory for SQLite, PostgreSQL, MySQL and MariaDB |

The umbrella crate exposes this as `ai-sql-memory`, which also enables `ai`.

## Dashboard crates

`rullst-nexus` enables `drivers-all` by default, and `rullst-studio` enables
`drivers-all` and `queue-sqlite`. Disable their defaults to select one backend:

| Crate | Feature | Enables |
| --- | --- | --- |
| `rullst-nexus` | `drivers-all` | All three SQLx drivers in Core and ORM (default) |
| `rullst-nexus` | `strict-postgres` | PostgreSQL selection in Core and ORM |
| `rullst-nexus` | `strict-mysql` | MySQL selection in Core and ORM |
| `rullst-nexus` | `strict-sqlite` | SQLite selection in Core and ORM |
| `rullst-studio` | `drivers-all` | All three SQLx drivers in Core and ORM (default) |
| `rullst-studio` | `queue-sqlite` | Core's durable SQLite queue backend (default) |
| `rullst-studio` | `strict-postgres` | PostgreSQL selection in Core and ORM |
| `rullst-studio` | `strict-mysql` | MySQL selection in Core and ORM |
| `rullst-studio` | `strict-sqlite` | SQLite selection in Core and ORM |

Use the same single-selection rule described for `rullst-orm`.

## Unpublished v13 candidate packages

`rullst-supervision`, `rullst-media` and `rullst-labs` are in the v13 release
inventory but have no umbrella features; depend on them directly. None has
default features.

| Crate | Feature | Enables |
| --- | --- | --- |
| `rullst-supervision` | `exam` | Typed exam sessions, collection categories and browser/capture observations |
| `rullst-supervision` | `parental` | Course/window parental restriction policies |
| `rullst-supervision` | `analysis` | `exam` plus bounded camera-presence/audio-activity adapter contracts |
| `rullst-supervision` | `sqlite` | `exam` and `parental` plus the shared-local SQLite store |
| `rullst-media` | `bunny` | Bunny Stream adapter, signatures, bounded HTTP and the browser upload module |
| `rullst-media` | `sqlite` | Shared-local durable assets, leased operations and the application service |
| `rullst-media` | `s3` | Experimental S3-compatible object storage (AWS S3, Cloudflare R2, MinIO): presigned upload and playback of the original, no transcoding |
| `rullst-labs` | `sqlite` | Encrypted shared-local exercise and job storage with leases, cancellation and retention |
| `rullst-labs` | `receipt-signing` | Receipt signing for the application-owned runner controller |

## Packages without optional features

These packages have no public optional Cargo features:

| Package | Always-available scope |
| --- | --- |
| `rullst-macros` | Core procedural macros |
| `cargo-rullst` | CLI commands, generators, auditing, and deployment helpers; its internal `maintainer-tools` feature only builds the repository's `sync-badges` tool and is not part of the installed CLI |

No optional feature does not mean that a provider is contacted automatically.
External integrations still require explicit runtime configuration and use the
documented deterministic offline behavior for empty or `mock_*` credentials.

## Selection recipes

Minimal HTTP runtime:

```toml
rullst = { version = "12.1.0", default-features = false }
```

SQLite application using the release default:

```toml
rullst = "12.1.0"
```

PostgreSQL application with explicit domain integrations:

```toml
rullst = {
    version = "12.1.0",
    default-features = false,
    features = ["strict-postgres", "auth", "security", "telemetry"]
}
```

Embedded IoT model without the standard library:

```toml
rullst-iot = { version = "12.1.0", default-features = false }
```

Experimental IoT fixtures are deliberately separate:

```toml
rullst-iot = {
    version = "12.1.0",
    default-features = false,
    features = ["experimental-simulators"]
}
```
