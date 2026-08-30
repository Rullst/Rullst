# Rullst Capital - Roadmap

> **Status policy (2026-08-26):** the roadmap remains ambitious. A checked
> foundation does not imply every provider method or live fiscal contract. See
> the audited [`rullst-capital` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> and the [capability ledger](../docs/src/capability-ledger.md).

Rullst Capital simplifies the billing and subscription complexities of building a SaaS application in Rust.

## Phase 1: Payment Gateways Integration
- [x] **Unified Payment Drivers**: First-class support for Stripe and LemonSqueezy with a standard Rust Trait interface.
- [~] **The `Billable` Trait**: `#[derive(rullst::Billable)]` now preserves
  generics and exposes checkout subscriptions through the facade. The historic
  `charge(amount)` shorthand is not implemented because a safe charge also
  needs currency, customer/payment-method identity and an idempotency policy.

## Phase 2: Billing Operations
- [x] **Fail-closed Axum and Actix Webhooks**: Both framework adapters call one canonical verifier. Supported provider signatures reject empty secrets; timestamped protocols enforce freshness, bodies are bounded and middleware provides bounded TTL replay protection.
- [ ] **Distributed Idempotency Store**: Persist accepted event keys across processes and deployments before billing side effects.
- [ ] **Alipay RSA2**: Implement interoperable RSA-SHA256 request signing and notification verification against official contract tests; live Alipay remains disabled until then.
- [~] **Invoicing Generation**: Generates escaped invoice HTML and converts a
  paid invoice into an offline DPS preview. Native PDF rendering and automatic
  delivery after payment are not implemented.

## Phase 3: Advanced Subscription Management
- [~] **Grace Periods & Pausing**: Cancellation and pausing exist on the trait;
  grace-period state and the historic subscription-handle API do not.
- [ ] **Proration Handling**: Automatically handle prorations when users upgrade or downgrade their tiers mid-billing cycle.
- [x] **Metered Billing (Usage-Based)**: API to report consumption (`user.report_usage("api_requests", 100).await`) for Stripe/LemonSqueezy metered limits.

## Phase 4: Customer Portal & UI Scaffold
- [x] **Customer Portal Link**: Method to generate a direct login link to Stripe Customer Portal or LemonSqueezy Customer Hub (`user.billing_portal_url().await`).
- [x] **Ready-made Scaffold**: `cargo rullst make:billing --model Workspace` generates registered SQLx or Turso-primary models/migrations, pricing, authenticated checkout/portal and mandatory webhook code for the selected Stripe/LemonSqueezy adapter. Materialized contracts compile, migrate, persist, deny cross-owner reuse and refuse collisions; route mounting, live sandbox proof and distributed reconciliation stay application-owned.

## Phase 5: Entitlements & Tax Management
- [x] **Tier-based Features**: Check if a user can access a feature based on their subscription tier (`user.can_access("pro_dashboard")`).
- [ ] **Global Tax Management**: Simplified support for VAT / Sales Tax calculations at checkout time, natively integrating with Stripe Tax.

## Phase 6: B2B & Team Billing (Organizations)
- [~] **Team Subscriptions**: Any suitable struct can derive `Billable`,
  including a Team/Workspace, but membership lookup and shared usage accounting
  remain application-owned.

## Phase 7: Quotas & Feature Limits
- [~] **Strict Resource Limits**: `check_quota(feature, current_usage)` is a
  fail-closed pure entitlement check. It does not query the database or
  automatically intercept resource creation.

## Phase 8: Coupons & Trial Management
- [~] **Native Discount APIs**: The typed operation and Stripe adapter exist;
  live support is provider-specific and LemonSqueezy currently exposes only a
  validated mock/foundation path.
- [~] **Trial Extensions**: The typed timestamp operation and Stripe adapter
  exist; live support is provider-specific rather than uniform.

## Phase 9: Multi-Currency (Localized Pricing)
- [ ] **Dynamic Geolocation Checkout**: Automatically detect a user's country/IP and resolve the correct gateway Price ID (e.g., charging in BRL for Brazil and USD for the USA) natively through the `Billable` trait.

## Phase 10: NFS-e Nacional
- [x] **Contained Offline Fixture**: Deterministic `OfflineMock` response that is unambiguously not an authorization.
- [~] **Homologation-Ready Contract**: Current official 1.01 artifact profiles are checksum-pinned; the bounded DPS builder, closed-catalog XSD validator, protected PKCS#12 handling, enveloped inclusive-C14N/RSA-SHA256 XMLDSig, independent local signature verification and bounded rustls mTLS client construction are implemented. Official JSON envelopes, strict response/rejection parsing, certificate/emitter and ICP-Brasil chain policy, durable idempotency/audit, real restricted-environment evidence, independent review and official homologation remain open. Homologation and production transmission stay fail-closed.
