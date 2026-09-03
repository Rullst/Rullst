# Rullst Capital - Roadmap

> **Status policy (2026-08-26):** the roadmap remains ambitious. A checked
> foundation does not imply every provider method or live fiscal contract. See
> the audited [`rullst-capital` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> and the [capability ledger](../docs/src/capability-ledger.md).

Rullst Capital simplifies the billing and subscription complexities of building a SaaS application in Rust.

## Phase 1: Payment Gateways Integration
- [x] **Unified Payment Drivers**: First-class support for Stripe and LemonSqueezy with a standard Rust Trait interface.
- [x] **Bounded Gateway Failure Contract**: Reviewed live adapter methods share
  finite connect/request timeouts, disabled redirects and ambient proxies,
  one-MiB JSON parsing, validated HTTPS checkout locations, and redacted typed
  permanent/transient/rate-limited evidence. Mutations are never retried
  automatically; durable idempotency and reconciliation remain caller-owned.
- [x] **The bounded `Billable` Trait**: `#[derive(rullst::Billable)]` preserves
  generics and exposes checkout subscriptions plus immediate charges through
  the facade. `charge_with`/`charge` require integer minor units, currency,
  provider customer and tokenized payment-method IDs, and an idempotency key;
  Stripe has reviewed live support and exact mock retries return a deterministic
  receipt explicitly typed as non-success `Mock`.
  Other adapters fail explicitly until their own direct-charge protocol is
  reviewed. The intentionally absent unsafe `charge(amount)` shorthand cannot
  guess currency, mandate, payment identity or retry policy.

## Phase 2: Billing Operations
- [x] **Fail-closed Axum and Actix Webhooks**: Both framework adapters call one canonical verifier. Supported provider signatures reject empty secrets; timestamped protocols enforce freshness, bodies are bounded and middleware provides bounded TTL replay protection.
- [x] **Bounded Shared Idempotency Store**: The opt-in `webhook-sql` ledger
  persists provider-scoped payload digests or stable semantic event IDs across
  processes on SQLite, PostgreSQL, MySQL, and MariaDB. Immutable capacity/TTL,
  database-time serialized claims, expiry, restart, contention, configuration
  drift and fail-closed capacity are tested. A caller-owned transaction can bind a
  semantic event claim to one database mutation; external effects still need
  an outbox, idempotent consumers and reconciliation.
- [ ] **Alipay RSA2**: Implement interoperable RSA-SHA256 request signing and notification verification against official contract tests; live Alipay remains disabled until then.
- [~] **Payment-Bound Invoicing**: Validates the legacy invoice model into exact
  minor units, renders escaped HTML and opt-in bounded native PDF, and binds a
  delivery only to final `Succeeded` evidence matching recipient, amount and
  currency. The downstream opt-in Mail bridge attaches that PDF and sends via
  the mandatory pipeline while exposing a stable key for the application's
  durable outbox. Automatic webhook orchestration, atomic cross-process
  claiming, exactly-once provider delivery and attachment parity remain open.

## Phase 3: Advanced Subscription Management
- [x] **Bounded Grace Periods & Subscription Handle**: `SubscriptionHandle<P>` validates/redacts the provider ID and exposes `cancel()`/`pause()` with static dispatch when the provider is explicit. `GracePeriod` is a validated half-open window of at most 366 days, and `#[derive(Billable)]` recognizes an all-or-none start/end field pair. Persistence, trusted clock, entitlement enforcement, provider semantics and scheduling remain application/provider boundaries.
- [ ] **Proration Handling**: Automatically handle prorations when users upgrade or downgrade their tiers mid-billing cycle.
- [x] **Metered Billing (Usage-Based)**: API to report consumption (`user.report_usage("api_requests", 100).await`) for Stripe/LemonSqueezy metered limits.

## Phase 4: Customer Portal & UI Scaffold
- [x] **Customer Portal Link**: Method to generate a direct login link to Stripe Customer Portal or LemonSqueezy Customer Hub (`user.billing_portal_url().await`).
- [x] **Ready-made Scaffold**: `cargo rullst make:billing --model Workspace` generates registered SQLx or Turso-primary models/migrations, pricing, authenticated checkout/portal and mandatory webhook code for the selected Stripe/LemonSqueezy adapter. Materialized contracts compile, migrate, persist, deny cross-owner reuse and refuse collisions; route mounting, live sandbox proof and distributed reconciliation stay application-owned.

## Phase 5: Entitlements & Tax Management
- [x] **Tier-based Features**: Check if a user can access a feature based on their subscription tier (`user.can_access("pro_dashboard")`).
- [ ] **Global Tax Management**: Simplified support for VAT / Sales Tax calculations at checkout time, natively integrating with Stripe Tax.

## Phase 6: B2B & Team Billing (Organizations)
- [x] **Team Subscriptions**: A Team/Workspace may own `Billable`, while a
  validated `BillingSubject` derived from trusted tenant context gives all
  authorized members one shared quota namespace. Authentication still owns
  membership establishment and provider webhooks still own plan reconciliation.

## Phase 7: Quotas & Feature Limits
- [x] **Strict Resource Limits**: `Billable::quota_request` derives the
  authoritative tier limit; `QuotaGate` blocks callbacks before over-limit or
  replayed creation and compensates ordinary failures. The opt-in SQL store
  atomically reserves idempotent units on SQLite/PostgreSQL/MySQL/MariaDB and
  exposes a caller-owned transaction path for committing the domain insert and
  quota together. It cannot intercept arbitrary writes made outside that gate.

## Phase 8: Coupons & Trial Management
- [x] **Native Discount APIs**: `CouponCode` validates and redacts provider
  coupon identifiers. Stripe sends the current `discounts[0][coupon]` contract,
  requests an expanded discount and binds the returned subscription and coupon.
  Lemon Squeezy discount codes remain checkout-only; it and unreviewed adapters
  return `UnsupportedOperation` in live mode instead of reporting false success.
- [x] **Trial Extensions**: `extend_trial(15)` now means 15 bounded whole days,
  with an explicit-clock variant for stable retries. Stripe and Lemon Squeezy
  send their current form/JSON:API update contracts and bind the returned
  subscription and expiration; unreviewed live adapters fail explicitly.

## Phase 9: Multi-Currency (Localized Pricing)
- [ ] **Dynamic Geolocation Checkout**: Automatically detect a user's country/IP and resolve the correct gateway Price ID (e.g., charging in BRL for Brazil and USD for the USA) natively through the `Billable` trait.

## Phase 10: NFS-e Nacional
- [x] **Contained Offline Fixture**: Deterministic `OfflineMock` response that is unambiguously not an authorization.
- [~] **Homologation-Ready Contract**: Current official 1.01 artifact profiles are checksum-pinned; the bounded DPS builder, closed-catalog XSD validator, protected PKCS#12 handling, enveloped inclusive-C14N/RSA-SHA256 XMLDSig, independent local signature verification, deterministic `dpsXmlGZipB64` JSON, strict signed-authorization and structured-rejection parser and bounded rustls mTLS client construction are implemented. Certificate/emitter and ICP-Brasil chain policy, durable idempotency/audit, retained official protocol fixtures, real restricted-environment evidence, independent review and official homologation remain open. Homologation and production transmission stay fail-closed.
