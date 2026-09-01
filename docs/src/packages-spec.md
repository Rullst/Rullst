# Community dependency helper and package roadmap

Rullst v12 does **not** define a runtime `RullstPackage` plugin ABI, execute
third-party generators, discover a private package registry, or register routes
automatically. Community integrations are ordinary Rust crates selected through
Cargo and reviewed like any other dependency.

## Current `cargo rullst pkg` boundary

The CLI provides two small local manifest helpers:

```bash
cargo rullst pkg add rullst-auth
cargo rullst pkg list
cargo check
```

`pkg add`:

- accepts only ASCII Cargo names of at most 64 bytes beginning with `rullst-`
  or `rullst_` and ending in an alphanumeric character;
- parses `Cargo.toml` as TOML and inserts the dependency into the real
  `[dependencies]` table using the installed CLI's version;
- leaves an existing dependency and its features/version unchanged;
- does not contact a registry, execute code, edit routes, or run a scaffold.

The subsequent `cargo check` performs normal Cargo resolution and compilation.
A prefix is only a naming filter; it is not proof that a crate is official,
safe, compatible, maintained, or endorsed by Rullst. Review the crate source,
publisher, checksum, license, advisories, feature graph and release policy
before adding it.

`pkg list` reads dependency keys from the parsed manifest and prints those with
the same Rullst prefix. It is not a vulnerability, license or provenance scan;
use the repository's audit and dependency-policy tooling for those questions.

## Integrating a community crate today

A community crate can expose ordinary Axum routers, Tower layers, SQLx types or
constructors. The host application initializes it explicitly and mounts only
the capabilities it intends to trust:

```rust
use axum::Router;

# fn community_router() -> Router { Router::new() }
fn application_router() -> Router {
    Router::new().nest("/community", community_router())
}
```

Authentication, authorization, tenant policy, secrets, migrations, shutdown,
telemetry and failure handling remain visible application responsibilities.

## Future package protocol

A first-class extension protocol remains roadmap work for the next feature
line. It should not be declared stable until the repository has all of the
following:

- a versioned manifest and compatibility contract;
- explicit capability permissions for routes, storage, network and secrets;
- deterministic lifecycle and failure semantics without runtime reflection;
- package ownership, provenance and revocation policy;
- compile/runtime conformance tests and safe uninstall/upgrade behavior;
- a decision on whether untrusted extensions require a real Wasm sandbox with
  enforceable CPU, memory and I/O limits.

Until that contract exists, documentation and packages must not claim that
`RullstPackage.toml`, `RullstPackage`, automatic route registration, or
third-party generator execution are implemented v12 APIs. See the
[capability ledger](capability-ledger.md#connect-real-time-queues-storage-and-data)
and [v12/v13 classification](v12.md) for the preserved vision.
