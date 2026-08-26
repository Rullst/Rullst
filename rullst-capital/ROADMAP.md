# Rullst Capital - Roadmap

> **Status policy (2026-08-26):** the roadmap remains ambitious. A checked
> foundation does not imply every provider method or live fiscal contract. See
> the audited [`rullst-capital` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> and the [capability ledger](../docs/src/capability-ledger.md).

Rullst Capital simplifies the billing and subscription complexities of building a SaaS application in Rust.

## Phase 1: Payment Gateways Integration
- [x] **Unified Payment Drivers**: First-class support for Stripe and LemonSqueezy with a standard Rust Trait interface.
- [x] **The `Billable` Trait**: Add `#[derive(Billable)]` to your User model to instantly gain methods like `user.charge(50.00).await` or `user.subscribe("pro_plan").await`.

## Phase 2: Billing Operations
- [x] **Fail-closed Axum Webhooks**: Supported provider signatures reject empty secrets; timestamped protocols enforce freshness and middleware provides bounded TTL replay protection.
- [ ] **Distributed Idempotency Store**: Persist accepted event keys across processes and deployments before billing side effects.
- [ ] **Alipay RSA2**: Implement interoperable RSA-SHA256 request signing and notification verification against official contract tests; live Alipay remains disabled until then.
- [x] **Invoicing Generation**: Generate beautiful invoices (HTML/PDF) natively in Rust and email them directly to the customer when a payment succeeds.

## Phase 3: Advanced Subscription Management
- [x] **Grace Periods & Pausing**: Manage subscriptions grace periods, cancellations, and pausing natively through the trait (`user.subscription("pro").cancel().await`).
- [ ] **Proration Handling**: Automatically handle prorations when users upgrade or downgrade their tiers mid-billing cycle.
- [x] **Metered Billing (Usage-Based)**: API to report consumption (`user.report_usage("api_requests", 100).await`) for Stripe/LemonSqueezy metered limits.

## Phase 4: Customer Portal & UI Scaffold
- [x] **Customer Portal Link**: Method to generate a direct login link to Stripe Customer Portal or LemonSqueezy Customer Hub (`user.billing_portal_url().await`).
- [x] **Ready-made Scaffold**: Use the `cargo rullst` CLI to generate a full Billing/Pricing page (`cargo rullst make:billing`) that uses the active providers.

## Phase 5: Entitlements & Tax Management
- [x] **Tier-based Features**: Check if a user can access a feature based on their subscription tier (`user.can_access("pro_dashboard")`).
- [ ] **Global Tax Management**: Simplified support for VAT / Sales Tax calculations at checkout time, natively integrating with Stripe Tax.

## Phase 6: B2B & Team Billing (Organizations)
- [x] **Team Subscriptions**: Shift the `Billable` paradigm so that an entire `Workspace` or `Team` model can hold a single subscription, sharing limits among multiple `User` accounts seamlessly.

## Phase 7: Quotas & Feature Limits
- [x] **Strict Resource Limits**: Extend the `Billable` trait with `user.check_quota("max_projects")`. The framework integrates with the DB to automatically block creations if a user exceeds their tier limits.

## Phase 8: Coupons & Trial Management
- [x] **Native Discount APIs**: Methods to programmatically validate and apply discounts (`user.apply_coupon("BLACKFRIDAY").await`).
- [x] **Trial Extensions**: Easily manage and artificially extend trial periods (`user.extend_trial(15).await`) through code.

## Phase 9: Multi-Currency (Localized Pricing)
- [ ] **Dynamic Geolocation Checkout**: Automatically detect a user's country/IP and resolve the correct gateway Price ID (e.g., charging in BRL for Brazil and USD for the USA) natively through the `Billable` trait.

## Phase 10: NFS-e Nacional
- [x] **Contained Offline Fixture**: Deterministic `OfflineMock` response that is unambiguously not an authorization.
- [ ] **Validated Live Issuance**: PKCS#12 private-key handling, XML C14N/XMLDSig, XSD validation, mTLS, strict response parsing and official homologation. Homologation and production remain fail-closed until complete.
