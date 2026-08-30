# Cargo feature matrix

This page is the public feature contract for the 15 packages in the Rullst
release train. The package manifests remain the machine-readable source of
truth. The matrix explains the behavior those names select in v12 and makes
the default build visible before an application adopts optional integrations.

Cargo features are additive across a dependency graph. An application can
disable a package's defaults at the dependency edge, but it cannot disable a
feature enabled by another dependency. Inspect the final selection with:

```bash
cargo tree -e features
```

The release gates compile every package with no default features, representative
individual boundaries, and the complete workspace with all features. See
[`check-feature-boundaries.sh`](../../.github/check-feature-boundaries.sh) for
the exact individual checks.

## Umbrella crate: `rullst`

The default `rullst` dependency enables `orm` and `queue-sqlite`. Applications
that only need the HTTP runtime can opt out:

```toml
[dependencies]
rullst = { version = "12.0.0", default-features = false }
```

| Feature | Default | Enables |
| --- | :---: | --- |
| `orm` | yes | `rullst-orm` and Core's ORM integration |
| `orm-mongodb` | no | `orm` plus the MongoDB document adapter |
| `orm-duckdb` | no | `orm` plus the in-process DuckDB analytics adapter |
| `orm-turso` | no | `orm` plus typed Turso-primary CRUD/query, parameterized remote libSQL SQL, transactions, reversible checked migrations, and a persistent offline fallback |
| `orm-surrealdb` | no | `orm` plus SurrealDB HTTP document and bounded graph adapters |
| `orm-polyglot` | no | Convenience feature enabling all four optional persistence adapters |
| `queue-sqlite` | yes | Core's durable SQLite queue backend |
| `nexus` | no | The generated Nexus administration interface |
| `studio` | no | Studio plus Core's Studio integration marker |
| `auth` | no | Authentication, sessions, passkeys, and RBAC helpers from `rullst-auth` |
| `auth-jwt` | no | `auth` plus the strict application-issued JWT policy |
| `mail-smtp` | no | `rullst-mail` with its SMTP transport |
| `mailer` | no | Compatibility alias for `mail-smtp`; prefer `mail-smtp` in new manifests |
| `queue-redis` | no | Redis dependency and Core's Redis queue backend |
| `cache-redis` | no | Redis dependency and Core's Redis cache backend |
| `redis` | no | Convenience alias enabling both `queue-redis` and `cache-redis` |
| `oauth` | no | OAuth2/OIDC providers from `rullst-connect` |
| `ai` | no | Provider-agnostic AI clients and local safeguards from `rullst-ai` |
| `capital` | no | Payment, payout, analytics, and offline fiscal-preview APIs from `rullst-capital` |
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
| `studio` | Integration marker used by the umbrella Studio boundary; it adds no dependency by itself |
| `telemetry` | OpenTelemetry tracing and OTLP export dependencies |
| `strict-postgres` | `orm` plus the ORM PostgreSQL backend selection |
| `strict-mysql` | `orm` plus the ORM MySQL backend selection |
| `strict-sqlite` | `orm` plus the ORM SQLite backend selection |

Core's process-local Radar and span collector do not require `telemetry`.
That feature is specifically for OpenTelemetry/OTLP integration.

### `rullst-orm`

Default features: none. With no `strict-*` feature, public pool and database
aliases use SQLx `Any`.

| Feature | Enables |
| --- | --- |
| `redis` | Redis connection-manager support and Redis-aware ORM errors |
| `mongodb` | Official MongoDB driver plus typed document CRUD and offline fallback |
| `duckdb` | Bundled DuckDB client plus parameterized, bounded analytics queries |
| `turso` | Official remote libSQL driver, typed primary CRUD/query facade, parameterized SQL, transactions, reversible checksummed migrations, and a persistent SQLite-compatible offline fallback |
| `surrealdb` | SurrealDB HTTP document CRUD and bounded read-only ISO GQL; no embedded SDK |
| `polyglot` | Convenience feature enabling `mongodb`, `duckdb`, `turso`, and `surrealdb` |
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

Default features: none.

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
| `axum-session` | Axum plus `tower-sessions` session extractors |
| `mock` | Deterministic offline provider modules outside test builds |

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
| `axum` | Axum HTTP response integration for Capital errors and handlers |

### `rullst-mail`

Default features: none. HTTP mail providers remain available without SMTP.

| Feature | Enables |
| --- | --- |
| `mail-smtp` | Lettre-based SMTP transport |

### `rullst-auth`

Default features: none.

| Feature | Enables |
| --- | --- |
| `oauth` | Optional `rullst-connect` OAuth2/OIDC integration and re-exports |
| `jwt` | Application-issued JWT claims, key rotation, and revocation-store policy |

The umbrella crate exposes this as `auth-jwt`, which also enables `auth`.

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
rullst = { version = "12.0.0", default-features = false }
```

SQLite application using the release default:

```toml
rullst = "12.0.0"
```

PostgreSQL application with explicit domain integrations:

```toml
rullst = {
    version = "12.0.0",
    default-features = false,
    features = ["strict-postgres", "auth", "security", "telemetry"]
}
```

Embedded IoT model without the standard library:

```toml
rullst-iot = { version = "12.0.0", default-features = false }
```

Experimental IoT fixtures are deliberately separate:

```toml
rullst-iot = {
    version = "12.0.0",
    default-features = false,
    features = ["experimental-simulators"]
}
```
