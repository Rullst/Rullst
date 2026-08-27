# Migration guide: v6 to v12

Baseline: repository source commit `5229132f`, which bumped the facade to
`6.0.0`. There is no repository v6 tag, so first verify the exact crate versions
in the application's `Cargo.lock`; do not assume this snapshot matches every
artifact an application may have consumed.

Follow the common [safe upgrade procedure](migration-v12.md).

## 1. Normalize the mixed-version ecosystem

The v6 snapshot combined `rullst = 6.0.0`, `rullst-orm = 6.1.1`,
`rullst-connect = 11.0.0`, path-based internal crates, and unversioned internal
path requirements. V12 uses one synchronized version for all 15 publishable
packages.

For the old no-default behavior:

```toml
[dependencies]
rullst = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["orm", "queue-sqlite"]
}
```

The explicit features above opt into v12's local database behavior. Omit them
for a database-free HTTP service. If the application relies on v12 defaults,
`rullst = "12.0.0-rc.1"` enables both automatically.

Remove obsolete direct `lettre` wiring from the facade migration and use
`mail-smtp`. Review the new `security`, `iot`, `redis`, and `strict-*` boundaries
in the [feature matrix](feature-matrix.md). Path-only dependencies are not
changed by `cargo rullst upgrade`.

## 2. Preserve valid ecosystem escape hatches

V6 applications often imported Axum, SQLx, and Tokio directly. Those imports
remain valid application choices. V12 exposes convenience paths, but migration
does not require global substitutions such as `axum::` to `rullst::server::`.
Review imports using compiler errors and API intent, not blind text replacement.

Prefer explicit `routes!` registration for the central router and propagate
startup errors from `Server::run`. Deprecated route attributes must not be
treated as functional registration.

## 3. Revalidate modular crate boundaries

V6 had already started splitting Core, Auth, Mail, AI, Nexus, Capital, and
Studio. V12 makes those package and feature relationships explicit and adds the
independently versioned ORM macros, Connect, Security, and IoT packages to the
same release train.

- Update direct imports only when the corresponding feature is enabled.
- Remove code that depended on empty facade features compiling without their
  domain crate.
- For SMTP, use `rullst-mail` through `mail-smtp`; HTTP mail providers do not
  require that feature.
- For OpenTelemetry export, enable `telemetry`; process-local Radar data does
  not require it.

## 4. Apply v12 security changes

- Build Nexus with an explicit access policy and `try_build()`.
- Keep Studio debug-only and loopback-bound.
- Derive tenant identity from authenticated membership, never directly from a
  selector header, subdomain, or URL parameter.
- Recheck exact CSRF exemptions, trusted proxy/TLS metadata, webhook replay
  protection, body limits, and parameterized-route ownership.
- Treat local security telemetry events as unsigned unless a configured audit
  verifier proves HMAC integrity.

## 5. Revalidate data and providers

Choose one strict database backend or SQLx `Any`, run migrations on restored
data, and exercise real SQLite/PostgreSQL/MySQL behavior used by the application.
Then validate live OAuth, mail, billing, and AI credentials separately from
their deterministic offline modes.

Finish with the full commands in the [common guide](migration-v12.md), the
application's exact feature build, a production-profile smoke test, and a
documented rollback rehearsal.
