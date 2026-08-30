# Rullst Nexus

`rullst-nexus` is a server-rendered administrative panel for models that
implement `NexusModel`, either manually or through `#[derive(Nexus)]`. The
derive infers primitive widgets and accepts explicit semantic metadata such as
`kind = "textarea"` and `kind = "enum", options = "draft, published"`. Nexus
provides registered-model CRUD, search, pagination, typed form widgets, bounded
selected-record delete/deactivate actions, telemetry, a security view and an
optional AI query page. The current interface uses server-side HTML and HTMX; Wasm islands,
drag-and-drop media management and automatic relationship discovery described by
older documentation were not implemented. They remain worthwhile separate
features, but must not be presented as current behavior.

## Build a protected panel

Nexus fails closed: `try_build()` requires an explicit validated access policy.
The generated-app helper permits credential-free access only in debug builds and
only for a loopback peer proven by Axum `ConnectInfo`. Release builds require
`NEXUS_ADMIN_USERNAME` and a unique `NEXUS_ADMIN_PASSWORD` of at least 16
characters.

```rust
use rullst_nexus::{Nexus, NexusAuthPolicy};

# fn build() -> Result<axum::Router, Box<dyn std::error::Error>> {
let access = NexusAuthPolicy::local_development_or_basic_from_env()?;
let nexus = Nexus::new()
    .with_auth_policy(access)
    .with_brand("Application Admin")
    // .register::<User>()
    .try_build()?;

let app = axum::Router::new().nest("/nexus", nexus);
# Ok(app)
# }
```

The serving boundary must preserve the socket address, for example with Axum's
`into_make_service_with_connect_info::<SocketAddr>()`. Basic Auth additionally
requires direct HTTPS or the application-owned `NexusVerifiedTls` capability
inserted only after validating a trusted TLS terminator. Never derive that
capability from an untrusted forwarded header.

`NexusAuthPolicy::protect_router` can apply the same administrator boundary to
application-owned operational routes, as the ERP blueprint does for inventory
mutations.

## Capability boundary

- Implemented: explicit model registration and a compile-tested derive;
  server-rendered tables/forms;
  parameterized and sanitized SQL identifiers; bound record values; CRUD,
  search, pagination, sort and batch operations; CSRF middleware; fail-closed
  loopback/Basic access; bounded Basic Auth failure throttling.
- Batch boundary: at most 1,000 explicitly selected IDs; deactivation is
  available only for a writable Boolean `is_active` or `active` field.
- Application responsibility: model/field authorization policy, database
  privileges, trusted proxy and TLS configuration, secret rotation, audit-log
  durability, tenant isolation and any ownership rules beyond the panel-wide
  administrator boundary.
- Not implemented: a generic `NexusLayer`, automatic ORM schema reflection,
  automatic `HasMany`/`BelongsTo` widgets, full rich-media management and a
  shared-production authentication service. These ideas may be implemented when
  they have typed contracts and proportional tests.

For a complete model example and the local/release access flow, see
[Rullst Nexus: Explicit Admin CMS](../4-rullst-nexus.md).
