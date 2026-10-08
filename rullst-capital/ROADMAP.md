# Rullst Capital - Roadmap

> **Status policy (2026-08-26):** the roadmap remains ambitious. A checked
> foundation does not imply every provider method. See
> the audited [`rullst-capital` row](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> and the [capability ledger](https://rullst.github.io/Rullst/book/capability-ledger.html).

Rullst Capital simplifies the billing and subscription complexities of building a SaaS application in Rust.

> **v13 scope (2026-10):** Capital keeps the provider-neutral base, the
> supported Stripe adapter and the experimental InfinitePay adapter. The
> Paddle, Lemon Squeezy, Polar, Razorpay, Mercado Pago, Alipay, Coinbase
> Commerce and PicPay adapters, the Wise payout adapter with its payout
> contracts, and the NFS-e/fiscal preparation module were removed. NFS-e may
> return later as a separate product outside the framework. Other gateways are
> application-owned adapters; see
> [Writing your own payment provider](https://github.com/Rullst/Rullst/blob/main/docs/src/capital-custom-provider.md)
> and the [v13 migration guide](https://github.com/Rullst/Rullst/blob/main/docs/src/migration-v13.md).

## Phase 1: Payment Gateways Integration
- [x] **Unified Payment Drivers**: A standard Rust trait interface with a supported Stripe adapter; InfinitePay remains experimental until validated against a live account.
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
- [ ] **InfinitePay live validation**: Implement reviewed checkout pricing, callback authentication and authoritative payment lookup, then validate against a live account before removing the experimental label.
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
- [x] **Metered Billing (Usage-Based)**: `MeteredBillingProvider` reports provider-specific consumption; Stripe Meter Events are implemented and custom adapters reuse the bounded `UsageReceipt`.

## Phase 4: Customer Portal & UI Scaffold
- [x] **Customer Portal Link**: `StripeProvider::create_bound_customer_portal` creates a Stripe Customer Portal session for an already persisted customer ID; the email-based `billing_portal_url` remains an offline fixture with live credentials unsupported.
- [x] **Local Billing Scaffold**: `cargo rullst make:billing --model Workspace` generates registered SQLx or Turso-primary models/migrations, demo pricing and guarded billing routes. Materialized contracts compile, migrate, persist offline fixtures, deny cross-owner reuse and refuse collisions. Real or mixed credentials return HTTP 503 until durable scoped ownership, attempts and atomic webhook processing replace the legacy demo flow; provider sandbox acceptance remains separate.

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
  Unreviewed adapters return `UnsupportedOperation` in live mode instead of
  reporting false success.
- [x] **Trial Extensions**: `extend_trial(15)` now means 15 bounded whole days,
  with an explicit-clock variant for stable retries. Stripe sends its current
  form update contract and binds the returned subscription and expiration;
  unreviewed live adapters fail explicitly.

## Phase 9: Multi-Currency (Localized Pricing)
- [ ] **Dynamic Geolocation Checkout**: Automatically detect a user's country/IP and resolve the correct gateway Price ID (e.g., charging in BRL for Brazil and USD for the USA) natively through the `Billable` trait.
