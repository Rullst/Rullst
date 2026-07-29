# Rullst Capital - Roadmap

Rullst Capital simplifies the billing and subscription complexities of building a SaaS application in Rust.

## Phase 1: Payment Gateways Integration
- [ ] **Unified Payment Drivers**: First-class support for Stripe and LemonSqueezy with a standard Rust Trait interface.
- [ ] **The `Billable` Trait**: Add `#[derive(Billable)]` to your User model to instantly gain methods like `user.charge(50.00).await` or `user.subscribe("pro_plan").await`.

## Phase 2: Billing Operations
- [ ] **Secure Webhooks**: A ready-to-use Actix/Axum middleware that automatically parses and cryptographically verifies webhook signatures from Stripe/LemonSqueezy to prevent fraud.
- [ ] **Invoicing Generation**: Generate beautiful PDF invoices natively in Rust and email them directly to the customer when a payment succeeds.
