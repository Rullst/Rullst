# Rullst Capital 💰
### *"Provider-Neutral SaaS Billing with Stripe and InfinitePay Adapters"*

`rullst-capital` provides a provider-neutral billing base for SaaS and digital
commerce in Rust: billing/metering traits, checkout and subscription contracts,
entitlements, quotas, revenue analytics, paid-invoice rendering, canonical
webhook verification with replay protection, Axum/Actix adapters and offline
mocks. It ships two adapters: **Stripe** (supported) and **InfinitePay**
(experimental until validated against a live account). Live readiness must be
established per operation and environment.

Integrate any other gateway in application code by implementing the provider
traits: see **[Writing your own payment provider](../capital-custom-provider.md)**.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Stripe Adapter** | 🟢 `[Supported / Bounded]` | Owner-bound customer, subscription checkout, bound portal, reconciliation reads, direct charge, meter events, coupons and trials with deterministic mocks. Live account acceptance remains per deployment. |
| **InfinitePay Adapter** | 🟡 `[Experimental]` | Deterministic offline checkout/portal/webhook fixtures. Live plan-only checkout and live callbacks fail closed until a reviewed contract is validated against a live account. |
| **Custom Providers** | 🟢 `[Extension Point]` | Applications implement `BillingProvider` (and optionally `MeteredBillingProvider`) for another gateway and reuse the canonical webhook verifier. See [the guide](../capital-custom-provider.md). |
| **Outbound Failure Boundary** | 🟢 `[Implemented / Bounded]` | Reviewed live methods share finite timeouts, disabled redirects/ambient proxies, one-MiB JSON parsing, HTTPS checkout-location validation, and redacted permanent/transient/rate-limited failures. Rullst performs no automatic mutation retry. |
| **Subscription Lifecycle** | 🟠 `[Partial]` | Checkout, portal, cancellation, pause, usage, coupon, trial, status, and webhook APIs exist; Stripe implements the reviewed live paths, while InfinitePay returns `UnsupportedOperation` for operations it does not support. |
| **Webhook Processing** | 🟢 `[Implemented / Bounded]` | Axum and opt-in Actix middleware call one canonical bounded verifier; adapters implement signature verification, with timestamp freshness checks for Stripe. The opt-in `webhook-sql` ledger shares bounded payload or semantic-event claims across SQLite, PostgreSQL, MySQL, and MariaDB processes. Relational handlers can claim a stable provider event ID with one domain mutation in a caller transaction. Cross-system exactly-once and reconciliation remain application work. |
| **Metered Billing** | 🟢 `[Implemented / Bounded]` | Current Stripe Meter Events shape with provider-specific identity, bounded response binding and deterministic non-live mocks. Custom adapters can return receipts that require application-outbox deduplication. Provider-account evidence remains explicit. |
| **v13 Plan Entitlements** | 🔵 `[Candidate]` | Typed per-action tenant/owner, exact feature/plan, mode, status, expiry and reconciliation-age checks. The generated SaaS report performs revision-fenced Stripe refreshes. Local and hosted acceptance remain tracked in the [delivery plan](../v13-delivery-plan.md); snapshots assert trusted adapter state and are not payment evidence. |
| **Paid Invoice Rendering** | 🟢 `[Implemented / Feature-gated]` | Exact validated minor units, escaped HTML, bounded paginated A4 PDF and a final-success e-mail/amount/currency binding. The downstream Mail bridge sends the attachment but durable outbox claiming and exactly-once delivery remain application work. |
| **SaaS MRR/ARR Analytics** | 🟢 `[Implemented / Bounded]` | In-memory revenue metrics and churn calculations for supplied records; this is not an accounting ledger or provider reconciliation engine. |

---

## 📦 Built-in providers

| Provider | Status | Scope |
| :--- | :--- | :--- |
| 💳 **Stripe** | Supported | Card checkout, Customer Portal, recurring subscriptions, Payment Intents charges and Meter Events within the reviewed operation boundaries. |
| ⚡ **InfinitePay** | Experimental | Brazilian Pix/card provider; offline fixtures only until live checkout and callback authentication are validated. |

Neither entry means every provider product, fee, payment method, tax promise or
live API path has been homologated by Rullst. The
[crate README](https://github.com/Rullst/Rullst/blob/main/rullst-capital/README.md)
lists the current operation boundary of each adapter.

### Removed in v13

v13 removed the Paddle, Lemon Squeezy, Polar, Razorpay, Mercado Pago, Alipay,
Coinbase Commerce and PicPay adapters, the Wise payout adapter with the payout
contracts (`PayoutProvider`, `PayoutStatus`, `PayoutEvent` and the payout
registry functions) and the NFS-e fiscal preparation module (`fiscal`, the
`nfse`/`capital-nfse` features and `Invoice::to_dps`). NFS-e was never
validated with a real municipality; it may return as a separate product outside
the framework. The code remains in git history. Applications that used a
removed provider implement the traits themselves
([guide](../capital-custom-provider.md)); see the
[v13 migration guide](../migration-v13.md) for the row "Capital providers and
NFS-e removed".

---

## Provider conformance ladder

Validate every provider operation independently. A successful checkout does
not validate a portal, refund, usage report, cancellation or webhook contract,
and evidence from one provider cannot be transferred to another.

| Level | Required evidence |
| :--- | :--- |
| **1. Deterministic offline** | Bounds, redaction, failure classification, idempotency material and explicit mock behavior without network access. |
| **2. Protocol fixtures** | Exact signed payloads, negative signature/freshness/replay cases and bounded provider response parsing. |
| **3. Official test environment** | Provider sandbox/test-mode checkout, webhook, lifecycle and reconciliation exercises. |
| **4. Controlled live acceptance** | The smallest provider-permitted real transaction only after account, legal, secret, refund, observability and reconciliation controls are ready; retain redacted evidence. |

The generated SaaS blueprint and `make:billing` provide a durable Stripe
integration only. It is not a conformance app for InfinitePay or for custom
adapters. Record the exact provider, operation, environment and observed
result; never summarize partial evidence as “all payments work.” Refer to the
official
[Stripe testing](https://docs.stripe.com/testing) and
[Stripe sandbox](https://docs.stripe.com/sandboxes)
guides when constructing acceptance cases.

---

## 🚀 Usage Examples

### Shared outbound failure contract

`CapitalError::Provider` carries a redacted `ProviderFailure` for request
construction, transport, non-success HTTP status, oversized/malformed JSON, or
semantic response mismatch. Its provider and operation labels are static and
safe for low-cardinality telemetry; the value deliberately omits URLs,
credentials, bodies, and raw transport errors.

```rust,no_run
use rullst_capital::{CapitalError, ProviderFailureClass};

fn record_disposition(error: &CapitalError) -> &'static str {
    match error {
        CapitalError::Provider(failure) => match failure.class() {
            ProviderFailureClass::Permanent => "permanent",
            ProviderFailureClass::Transient => "transient",
            ProviderFailureClass::RateLimited => "rate_limited",
            _ => "unknown",
        },
        _ => "not_provider_transport",
    }
}
```

HTTP 429 is rate-limited; transport failures and HTTP 408, 425, and 5xx are
transient; request-build, response-shape, and other HTTP failures are
permanent. Only numeric `Retry-After` delta seconds are retained and they are
capped at 24 hours. These are scheduling hints, not a generic retry engine:
non-idempotent operations must not be repeated without a durable,
provider-forwarded idempotency key and reconciliation.

### 1. Initializing a Provider and Creating a Checkout Session

```rust,no_run
use rullst_capital::providers::stripe::StripeProvider;
use rullst_capital::BillingProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stripe = StripeProvider::new(
        "sk_live_your_stripe_api_key",
        "whsec_your_webhook_signing_secret",
    );

    let session = stripe
        .create_checkout_session(
            "customer@example.com",
            "price_pro_monthly",
            "https://example.com/billing/complete",
        )
        .await?;

    println!("Checkout URL: {session}");
    Ok(())
}
```

This legacy email-based trait method remains for source compatibility. New
Stripe integrations use the customer-bound `create_customer` and
`create_subscription_checkout` flow; see the
[owner-bound Stripe checkout](../payment-gateways-guide.md#2-create-an-owner-bound-stripe-checkout).

### 2. Provider-Specific Metered Usage

`MeteredBillingProvider` uses an associated request type so each adapter keeps
its own provider identity instead of guessing from one subscription ID.
`StripeMeterEvent` implements the current form-encoded Meter Events contract
and forwards its identifier as both event identity and idempotency header.

The Stripe path validates positive bounded quantities, binds accepted
responses, caps response JSON to one MiB and returns visibly non-live
deterministic mocks. A Stripe identifier has only rolling provider
deduplication. A custom adapter whose provider exposes no deduplication key
returns `UsageDeduplication::ApplicationOutboxRequired`, so the application
claims the event key in a durable outbox before sending. Live-account
acceptance, retry/reconciliation and entitlements remain application/release
evidence.

### 3. Payment-Bound Invoice PDF and Mail

Enable `rullst/capital-mail` or the separate `rullst-capital/invoice-pdf` and
`rullst-mail/capital-invoice` features. A `PaidInvoice` can be constructed only
from final `Succeeded` evidence matching the invoice recipient, exact
minor-unit total and currency. `PaidInvoiceDelivery::prepare` generates escaped
HTML and a bounded PDF attachment and runs Mail's mandatory pre-flight.

The stable delivery key is an application outbox identity, not a distributed
lock. The application must reconcile webhooks, claim that key atomically and
own at-least-once retries/provider attachment policy.

### 4. Verified Webhook Signature Handling

The low-level provider contract below illustrates exact-byte verification. HTTP
applications should normally mount `verify_webhook` on Axum or
`verify_webhook_actix_with_state` on Actix so body limits, normalized event
insertion, and replay rejection are applied before the handler. The default
store is process-local; the opt-in `webhook-sql` feature accepts an
`Arc<SqlWebhookReplayStore>` in `WebhookMiddlewareState` for cross-process
admission. Active claims are never evicted to admit new work. Webhooks
use constant-time cryptographic verification where applicable:

```rust
use axum::{body::Bytes, http::HeaderMap, response::IntoResponse};
use rullst_capital::providers::stripe::StripeProvider;
use rullst_capital::BillingProvider;
use std::collections::HashMap;

pub async fn handle_stripe_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let stripe = StripeProvider::new(
        "sk_live_api_key",
        "whsec_your_webhook_signing_secret",
    );

    let signature = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(axum::http::StatusCode::BAD_REQUEST)?;

    let provider_headers = HashMap::from([(
        "stripe-signature".to_string(),
        signature.to_string(),
    )]);

    // Verifies the provider signature and timestamp before parsing the event.
    let event = stripe
        .handle_webhook(&body, &provider_headers)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    println!(
        "Verified subscription {} with status {:?}",
        event.subscription_id,
        event.status,
    );
    Ok(axum::http::StatusCode::OK)
}
```

SQL-backed middleware claims the payload before dispatch. Treat it as a
fail-closed replay firewall, not an exactly-once delivery guarantee. For an
atomic relational state change, verify the exact payload through the selected
provider, obtain its stable event identifier, then call
`check_and_record_event_key_with_transaction` inside the same transaction as
the domain mutation. Provider API calls, e-mail, queues, and other systems still
need an outbox, idempotent consumers, and reconciliation.

---

## 🔒 Security Invariants

1. **Constant-Time Verification:** HMAC webhook signatures and explicit `mock_*` fixture secrets are compared with `subtle::ConstantTimeEq` or ring's constant-time `hmac::verify`.
2. **Fail-Closed Live Modes:** Operations without a reviewed live contract, including InfinitePay live checkout and callbacks, return `UnsupportedOperation` before network I/O instead of a fabricated success.
3. **Bounded Egress:** Reviewed live provider methods use a pooled client with
   finite connect/request timeouts, disabled redirects and ambient proxy
   discovery, bounded JSON, and redacted typed failure evidence. Returned
   checkout URLs must be absolute credential-free HTTPS without fragments.
