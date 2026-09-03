# Cargo feature matrix

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

This page is the public feature contract for the 16 packages in the Rullst
release train. The package manifests remain the machine-readable source of
truth. The matrix explains the behavior those names select in v12 and makes
the default build visible before an application adopts optional integrations.

Cargo features are additive across a dependency graph. An application can
disable a package's defaults at the dependency edge, but it cannot disable a
feature enabled by another dependency. Inspect the final selection with:

```bash
cargo tree -e features
```

The release gates compile every package with no default features, every public
umbrella feature in isolation, representative domain-package boundaries, and
the complete workspace with all features. The isolated umbrella list is
checked automatically against `rullst/Cargo.toml`, so a newly added public
feature cannot silently escape the matrix. See
[`check-feature-boundaries.sh`](../../.github/check-feature-boundaries.sh) for
the exact individual checks.

## Umbrella crate: `rullst`

The default `rullst` dependency enables `orm` and `queue-sqlite`. Applications
that only need the HTTP runtime can opt out:

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", default-features = false }
```

| Feature | Default | Enables |
| --- | :---: | --- |
| `orm` | yes | `rullst-orm` and Core's ORM integration |
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
| `nexus` | no | The generated Nexus administration interface |
| `studio` | no | Studio plus Core's Studio integration marker |
| `auth` | no | Authentication, sessions, passkeys, and RBAC helpers from `rullst-auth` |
| `auth-jwt` | no | `auth` plus the strict application-issued JWT policy |
| `auth-sqlite` | no | `auth-jwt` plus bounded shared SQLite JWT revocation and passkey device lifecycle state |
| `mail` | no | `rullst-mail` with HTTP/offline transports and no SMTP dependency |
| `mail-sqlite` | no | `mail` plus bounded shared-local SQLite recipient suppression and provider-event replay evidence |
| `mail-smtp` | no | `mail` plus the optional SMTP transport |
| `mail-aws-ses` | no | `mail` plus native SES v2 delivery signed by the official AWS SDK |
| `messaging` | no | Native bounded broker-neutral messaging contracts and the deterministic process-local broker |
| `messaging-sqlite` | no | `messaging` plus fixed-schema durable local SQLite publication, lease, retry/DLQ, ACK and idempotency state |
| `messaging-orm-outbox` | no | `messaging` and `orm` plus the static relational outbox-to-broker relay; the publish/ACK crash window remains at-least-once |
| `mailer` | no | Compatibility alias for `mail-smtp`; prefer `mail-smtp` in new manifests |
| `queue-redis` | no | Redis dependency and Core's Redis queue backend |
| `cache-redis` | no | Redis dependency and Core's Redis cache backend |
| `redis` | no | Convenience alias enabling `queue-redis`, `cache-redis` and `orm-redis` |
| `offline-sync` | no | Native bounded offline queue, explicit conflict state machine, account-bound encrypted snapshots, and static-dispatch push/pull orchestration; platform storage and concrete transport remain application responsibilities |
| `oauth` | no | OAuth2/OIDC providers from `rullst-connect` |
| `ai` | no | Provider-agnostic AI clients and local safeguards from `rullst-ai` |
| `ai-sql-memory` | no | `ai` plus tenant-aware durable chat memory for SQLite, PostgreSQL, MySQL, and MariaDB |
| `capital` | no | Payment, payout, analytics, DPS builder, and offline fiscal APIs from `rullst-capital` |
| `capital-actix` | no | `capital` plus the Actix Web adapter for the canonical signed-webhook verifier |
| `capital-quota-sql` | no | `capital` and `orm` plus atomic shared resource quotas for SQLite, PostgreSQL, MySQL, and MariaDB |
| `capital-webhook-sql` | no | `capital` and `orm` plus bounded durable webhook replay/event claims for SQLite, PostgreSQL, MySQL, and MariaDB |
| `capital-nfse` | no | `capital` plus checksum-pinned official XSD validation, PKCS#12 XMLDSig, and rustls mTLS preparation |
| `capital-pdf` | no | `capital` plus bounded validated native invoice PDF rendering |
| `capital-mail` | no | `capital-pdf` plus Mail's payment-bound HTML/PDF attachment delivery bridge |
| `security` | no | RASP/WAF and application-security primitives from `rullst-security` |
| `security-redis` | no | `security` plus the atomic Redis rate limiter |
| `iot` | no | IoT models, frame helpers, and signed OTA verification from `rullst-iot` |
| `telemetry` | no | OpenTelemetry dependencies and Core's OTLP integration |
| `strict-postgres` | no | `orm` with the concrete PostgreSQL pool/backend selected |
| `strict-mysql` | no | `orm` with the concrete MySQL pool/backend selected when PostgreSQL is not also selected |
| `strict-sqlite` | no | `orm` with the concrete SQLite pool/backend selected when PostgreSQL and MySQL are not also selected |

The three `strict-*` backend features are supported as single selections.
Feature unification can activate more than one; the current deterministic
precedence is PostgreSQL, then MySQL, then SQLite. Do not depend on that
precedence as backend negotiation. Select one strict backend in an application,
or select none to use SQLx `Any`.

## Runtime and data crates

### `rullst-core`

Default features: none.

| Feature | Enables |
| --- | --- |
| `orm` | Optional `rullst-orm` and SQLx support, including Artisan and database-backed feature flags |
| `queue-sqlite` | SQLx-backed durable SQLite queues without enabling the full ORM facade |
| `queue-redis` | Redis-backed queues |
| `cache-redis` | Redis-backed cache storage |
| `redis` | Convenience alias for both Redis queue and cache backends |
| `offline-sync` | Native bounded offline state, AES-256-GCM snapshots, and timeout/budget/cursor-checked transport orchestration; excludes platform storage and a concrete authenticated transport |
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

Default features: none. With no `strict-*` feature, public pool and database
aliases use SQLx `Any`.

| Feature | Enables |
| --- | --- |
| `redis` | Redis query cache plus bounded namespaced Hash, Set and Sorted Set datastore operations |
| `mongodb` | Official MongoDB driver plus typed document CRUD and offline fallback |
| `duckdb` | Bundled DuckDB client plus parameterized, bounded analytics queries |
| `turso` | Direct official Hrana HTTP v3 transport, typed primary CRUD/query facade, parameterized SQL, atomic batches, reversible checksummed migrations, and a persistent SQLite-compatible offline fallback |
| `surrealdb` | SurrealDB HTTP document CRUD and bounded read-only ISO GQL; no embedded SDK |
| `scout-http` | Bounded Meilisearch, Elasticsearch and Algolia adapters with deterministic offline fallbacks; Meilisearch also has a live container contract |
| `pgvector` | Typed pgvector SQLx values and parameterized L2/cosine/inner-product helpers; the live contract also selects `strict-postgres` |
| `qdrant` | Bounded dense-vector collection/upsert/delete/cosine query operations over HTTP with offline fallback |
| `polyglot` | Convenience feature enabling `mongodb`, `duckdb`, `turso`, `surrealdb`, and `qdrant` |
| `strict-postgres` | Concrete PostgreSQL pool, database, query-result, and query paths |
| `strict-mysql` | Concrete MySQL paths when PostgreSQL is not also selected |
| `strict-sqlite` | Concrete SQLite paths when PostgreSQL and MySQL are not also selected |

The strict backend selection rules and precedence are the same as the umbrella
crate. SQLx drivers remain implementation dependencies; `strict-*` selects
concrete public types and query paths rather than acting as a driver download
switch.

The Polyglot features expose capability-specific APIs under
`rullst_orm::polyglot`; they do not participate in a shared cross-backend
transaction. Turso can additionally be selected explicitly by
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
| `strict-postgres` | Compatibility marker matching the ORM backend vocabulary; no macro expansion changes in v12 |
| `strict-mysql` | Compatibility marker matching the ORM backend vocabulary; no macro expansion changes in v12 |
| `strict-sqlite` | Compatibility marker matching the ORM backend vocabulary; no macro expansion changes in v12 |

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
| `mock` | Deterministic offline provider modules outside test builds |

### `rullst-messaging`

Default features: none. The deterministic process-local broker, versioned
envelope, idempotency, consumer groups, leases, retry, dead-letter, and purge
contracts are available without optional dependencies. Remote broker adapters
are not implemented and therefore are not represented by placeholder features.

| Feature | Enables |
| --- | --- |
| `sqlite` | Fixed-schema durable local broker with serialized SQLite writes and immutable plaintext or explicit AES-256-GCM content profiles; restart/corruption/rotation/tamper/two-instance evidence is local, while metadata visibility, key custody and remote replication/failover remain explicit boundaries |
| `orm-outbox` | Static bridge from the relational `rullst-orm` outbox to one configured broker topic, with exact replay after the publish-before-ACK crash window; worker operations and remote atomicity remain application boundaries |

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
| `nfse` | Checksum-pinned official XSD validation, PKCS#12 RSA-SHA256 XMLDSig, deterministic GZip/Base64 issuance JSON, bounded signed-authorization and structured-rejection parsing, and rustls mTLS preparation; it does not enable live SEFIN transmission or establish certificate trust/homologation |
| `invoice-pdf` | Bounded paginated A4 invoice PDF with embedded WinAnsi or a validated caller-supplied TTF/OTF; payment/mail orchestration is separate |

### `rullst-mail`

Default features: none. HTTP mail providers remain available without SMTP.

| Feature | Enables |
| --- | --- |
| `mail-smtp` | Lettre-based SMTP transport |
| `aws-ses` | Official AWS SES v2 SDK, regional SigV4, temporary/rotating credential providers and native attachments/CID; AWS account readiness and inbox delivery remain external |
| `capital-invoice` | Capital's native invoice PDF plus the final-payment-bound delivery bridge; durable outbox claiming remains application-owned |
| `sqlite` | File-backed shared-local suppression state with exact provider-event replay binding and immutable quotas; webhook authentication, encryption and multi-host replication remain application-owned |

### `rullst-auth`

Default features: none.

| Feature | Enables |
| --- | --- |
| `oauth` | Optional `rullst-connect` OAuth2/OIDC integration and re-exports |
| `jwt` | Application-issued JWT claims, key rotation, and revocation-store policy |
| `sqlite` | `jwt` plus bounded file-backed shared JWT revocation and passkey device lifecycle state |

The umbrella crate exposes these as `auth-jwt` and `auth-sqlite`; both enable
`auth`, while `auth-sqlite` also enables `auth-jwt`.

### `rullst-security`

Default features: none.

| Feature | Enables |
| --- | --- |
| `redis-rate-limit` | Atomic namespaced Redis fixed-window limiter plus its explicit offline mock mode; CI/release run the independent-client contract against a digest-pinned Redis service |

The umbrella crate exposes this as `security-redis`, which also enables
`security`.

## Dashboard crates

`rullst-nexus` and `rullst-studio` both have no default features and expose the
same database selection boundary:

| Crate | Feature | Enables |
| --- | --- | --- |
| `rullst-nexus` | `strict-postgres` | PostgreSQL selection in Core and ORM |
| `rullst-nexus` | `strict-mysql` | MySQL selection in Core and ORM |
| `rullst-nexus` | `strict-sqlite` | SQLite selection in Core and ORM |
| `rullst-studio` | `strict-postgres` | PostgreSQL selection in Core and ORM |
| `rullst-studio` | `strict-mysql` | MySQL selection in Core and ORM |
| `rullst-studio` | `strict-sqlite` | SQLite selection in Core and ORM |

Use the same single-selection rule described for `rullst-orm`.

## Packages without optional features

These packages have no public optional Cargo features in v12:

| Package | Always-available scope |
| --- | --- |
| `rullst-macros` | Core procedural macros |
| `rullst-ai` | Provider clients, prompt inspection, and PII masking |
| `cargo-rullst` | CLI commands, generators, auditing, and deployment helpers |

No optional feature does not mean that a provider is contacted automatically.
External integrations still require explicit runtime configuration and use the
documented deterministic offline behavior for empty or `mock_*` credentials.

## Selection recipes

Minimal HTTP runtime:

```toml
rullst = { version = "12.0.0-rc.1", default-features = false }
```

SQLite application using the release default:

```toml
rullst = "12.0.0-rc.1"
```

PostgreSQL application with explicit domain integrations:

```toml
rullst = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["strict-postgres", "auth", "security", "telemetry"]
}
```

Embedded IoT model without the standard library:

```toml
rullst-iot = { version = "12.0.0-rc.1", default-features = false }
```

Experimental IoT fixtures are deliberately separate:

```toml
rullst-iot = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["experimental-simulators"]
}
```
