# Rullst Nexus

`rullst-nexus` is Rullst's authenticated administrative CMS for registered `rullst-orm`
models. It provides server-rendered CRUD views, server-side field policies, RBAC enforcement,
telemetry, and the optional AI assistant.

## Secure mounting

Nexus is fail-closed: `try_build()` returns an error until an explicit access policy is selected.
The built-in Basic Auth policy rejects weak/example credentials, compares both credential fields in
constant time, limits failures by verified socket peer, and locks peers after repeated failures.

Basic credentials are only encoding, not encryption. The middleware therefore accepts them only on
an HTTPS request or when trusted deployment middleware inserts `NexusVerifiedTls`. This marker is a
security assertion: never create it solely because an untrusted request supplied
`X-Forwarded-Proto` or another forwarding header.

```rust
use axum::{Extension, Router};
use rullst_core::Server;
use rullst_nexus::{
    Nexus, NexusAuthPolicy, NexusVerifiedTls,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = NexusAuthPolicy::basic_from_env()?;
    let nexus = Nexus::new()
        // .register::<User>()
        .with_auth_policy(policy)
        .try_build()?;

    // This example assumes a trusted reverse proxy has already terminated and required TLS.
    // Insert the marker only at that trusted boundary.
    let app = Router::new()
        .nest("/nexus", nexus)
        .layer(Extension(NexusVerifiedTls::from_trusted_tls_termination()));

    Server::new(app).run(3000).await?;
    Ok(())
}
```

Set unique values for `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD`; the password must contain
at least 16 characters. Rullst's server supplies `ConnectInfo<SocketAddr>`, which the Basic Auth
guard requires so a forged forwarding header cannot choose the rate-limit identity.

For local development only, debug builds can explicitly select
`NexusAuthPolicy::loopback_only(LocalNexusAccess::loopback_only())`. It still requires a verified
loopback socket peer and is rejected in release builds.

## Security boundaries

Nexus includes CSRF protection and escapes untrusted values rendered into admin pages. Applications
must still place the complete production server behind TLS, install Rullst's secure headers and WAF,
and apply authorization/ownership policy to any custom routes mounted next to Nexus.
