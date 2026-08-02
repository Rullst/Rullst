# Rullst Extension Packages Specification (`Rullst Packages`)

To foster a healthy ecosystem and community extension marketplace without bloating the core framework, Rullst defines a standardized package interface.

---

## 1. Package Trait Definition

Any community extension (OAuth providers, payment gateways, e-commerce connectors) must implement the `RullstPackage` trait:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait RullstPackage: Send + Sync {
    /// Unique package name identifier (e.g. "rullst-stripe")
    fn name(&self) -> &'static str;

    /// Package version string
    fn version(&self) -> &'static str;

    /// Optional initialization logic called when attached to the Server
    async fn on_init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Optional router configuration hook
    fn register_routes(&self, router: rullst::server::Router) -> rullst::server::Router {
        router
    }
}
```

---

## 2. Attaching Packages to a Rullst Application

Developers attach community packages in `src/main.rs`:

```rust
use rullst::server::Server;
// Example community package
use rullst_stripe::StripePackage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::new()
        .package(StripePackage::new("sk_test_123"))
        .listen("127.0.0.1:3000")
        .await?;

    Ok(())
}
```

---

## 3. Package Manifest (`RullstPackage.toml`)

Packages published to Crates.io may include a `RullstPackage.toml` manifest to declare CLI generator extensions:

```toml
[package]
name = "rullst-stripe"
version = "1.0.0"
description = "Stripe integration for Rullst applications"

[generators]
scaffold = "rullst-stripe-cli"
```

This enables the CLI command:
```bash
cargo rullst add rullst-stripe
```
which fetches the dependency, registers the package in `src/main.rs`, and executes optional package scaffolding logic.
