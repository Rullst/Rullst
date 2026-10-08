# 💳 Payment Gateways & Financial Infrastructure Guide

Rullst Capital (`rullst-capital`) provides typed payment, subscription and
webhook contracts with two built-in adapters. Unsupported operations return
typed errors, and mock credentials select deterministic offline behavior.

The adapters share common traits, but they do not all implement every
operation. An adapter's presence is not a promise of geographic availability,
tax treatment, settlement time, pricing, or regulatory suitability.

---

## 📊 Adapter inventory

| Adapter | Status | Rullst contract |
| :--- | :--- | :--- |
| Stripe | Supported | Owner-bound customer and subscription checkout, bound portal, reconciliation reads, Payment Intents charges, Meter Events, coupons and trials; unsupported methods fail explicitly. |
| InfinitePay | Experimental | Deterministic offline checkout, portal and webhook fixtures. Live plan-only checkout and live callbacks return `UnsupportedOperation` until a reviewed contract is validated against a live account. |
| Your own adapter | Extension point | Implement `BillingProvider` (and optionally `MeteredBillingProvider`) in the application; see [Writing your own payment provider](capital-custom-provider.md). |

v13 removed the Paddle, Lemon Squeezy, Polar, Razorpay, Mercado Pago, Alipay,
Coinbase Commerce, PicPay and Wise (payout) adapters; see the
[v13 migration guide](migration-v13.md) row "Capital providers and NFS-e
removed". Provider pricing and terms change. Check the provider's current
official documentation and the concrete trait implementation before selecting
an adapter.

---

## Failures, timeouts, and retry ownership

Reviewed live methods use a single bounded egress contract: five-second connect
and twenty-second whole-request timeouts, no redirects, no ambient proxy
variables, and at most one MiB of JSON. Checkout responses additionally require
a bounded credential-free HTTPS URL and provider-specific origin/identity checks.
Stripe hosted Checkout preserves its documented opaque fragment; other checkout
locations must not carry a fragment. These controls do not prove that a
provider account, product, price, or operation is accepted live.

`CapitalError::Provider` exposes only static provider/operation labels, a
`ProviderFailureKind`, optional HTTP status, bounded numeric `Retry-After`, and
one of three dispositions: permanent, transient, or rate-limited. It does not
retain a raw URL, credential, response body, or `reqwest` diagnostic. Log those
structured fields instead of formatting the original request.

Rullst deliberately does not retry mutations. A transient classification means
only that a later attempt may succeed. Before retrying, the application must
prove that the concrete operation forwards the same persisted idempotency key;
otherwise reconcile provider state first. Backoff, jitter, attempt budgets,
dead-letter handling, and operator alerts remain explicit application policy.

---

## 🔍 Selection model

Choose a provider only after checking which trait methods the Rullst adapter
implements, the currencies and countries enabled on the actual merchant account,
the current provider contract, webhook replay/idempotency requirements, and the
application's legal and tax responsibilities. Merchant-of-record status and tax
handling are external contractual properties, not guarantees made by Rullst.

---

## 💻 Rust Code Integration Examples

### 1. Choose the implemented operation

The [Capital README](https://github.com/Rullst/Rullst/blob/main/rullst-capital/README.md)
lists each adapter's current operations. Stripe has a generated durable
subscription integration (`make:billing` and the SaaS blueprint). InfinitePay
is experimental and only its offline fixtures succeed. For another gateway,
write an application-owned adapter as described in
[Writing your own payment provider](capital-custom-provider.md).

### 2. Create an owner-bound Stripe checkout

```rust,no_run
use rullst_capital::{CapitalError, StripeCheckoutRequest, StripeProvider};

async fn checkout() -> Result<(), CapitalError> {
    let provider = StripeProvider::new("mock_key", "mock_webhook");
    // In the host: authorize this owner and persist the account/mode-scoped
    // customer binding and immutable attempt before dispatch.
    let attempt = StripeCheckoutRequest::new(
        "cus_existing", "price_monthly", "owner_opaque", "attempt_unique",
        "https://app.example/billing/success", "https://app.example/billing/cancel",
    )?;
    let session = provider.create_subscription_checkout(&attempt).await?;
    let _ = session; // Persist its ID before a 303 handoff; never infer payment.
    Ok(())
}
```

The account's recurring price allowlist is server-owned. A local user may never
supply another customer's ID or choose a raw provider URL. For the complete
SQLx/Turso application flow, use the new SaaS/`make:billing` modules and follow
its generated `BILLING.md`; see [12.1 migration](migration-v12-1.md).

### 3. Cryptographically Verified Webhook Endpoint

Signature verification authenticates raw delivery bytes. It does not establish
application ownership or settlement. Use the provider's typed verified envelope
with the persisted owner/customer/checkout binding; do not grant or revoke access
from `WebhookEvent.customer_email`, an event name alone or a browser return.

For Stripe, the generated handlers verify signed Checkout/subscription events,
read current provider state under a database revision fence and atomically
commit the scoped event receipt with subscription state. Exact replays do not
repeat changes; conflicting receipts, foreign customers and obsolete attempts
are rejected. Unknown provisioning/checkout outcomes require bounded recovery.
InfinitePay accepts only explicit `mock_*` webhook fixtures; a real secret
returns `UnsupportedOperation` until callback authentication is validated. A
custom adapter supplies its own durable orchestration, atomic event processing
and reconciliation.

Rullst also supplies canonical Axum/Actix webhook middleware for supported
normalized events. The production entry points reject empty/`mock_*` secrets.
A normalized `WebhookEvent` is a lower-level input, not a complete entitlement
decision; preserve raw verified event identity and scope where needed.

#### Actix Web adapter

Enable `rullst-capital` with `default-features = false, features = ["actix"]`,
or enable `rullst/capital-actix` through the umbrella crate, and add
`actix-web` as a direct application dependency. An explicit
provider-bound state avoids global provider configuration and makes the replay
boundary visible:

```rust,no_run
use actix_web::{App, HttpMessage, HttpRequest, HttpResponse, HttpServer, middleware, web};
use rullst_capital::{
    InMemoryWebhookReplayStore, StripeProvider, WebhookEvent,
    WebhookMiddlewareState, verify_webhook_actix_with_state,
};
use std::sync::Arc;

async fn handle_billing_event(request: HttpRequest) -> HttpResponse {
    let Some(event) = request.extensions().get::<WebhookEvent>().cloned() else {
        return HttpResponse::InternalServerError().finish();
    };
    // This example only demonstrates middleware extraction. Before any domain
    // mutation, bind verified identity and reconcile current state as above.
    let _ = event;
    HttpResponse::NoContent().finish()
}

async fn serve() -> std::io::Result<()> {
    let provider = Arc::new(StripeProvider::new(
        "sk_live_from_secret_store",
        "whsec_from_secret_store",
    ));
    let replay = Arc::new(InMemoryWebhookReplayStore::default());
    let state = WebhookMiddlewareState::production_with_provider(provider, replay);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::from_fn(verify_webhook_actix_with_state))
            .route("/webhooks/capital", web::post().to(handle_billing_event))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

The default in-memory replay store is atomic only inside one process. Enable
`rullst-capital/webhook-sql` (or umbrella `rullst/capital-webhook-sql`) to share
a bounded replay ledger across SQLite, PostgreSQL, MySQL, or MariaDB processes:

```rust,no_run
use rullst_capital::{
    SqlWebhookReplayStore, StripeProvider, WebhookMiddlewareState,
};
use std::{sync::Arc, time::Duration};

async fn webhook_state(
    database_url: String,
) -> Result<WebhookMiddlewareState, rullst_capital::CapitalError> {
    let replay = Arc::new(
        SqlWebhookReplayStore::connect(
            database_url,
            100_000,
            Duration::from_secs(24 * 60 * 60),
        )
        .await?,
    );
    replay.prepare_schema().await?;
    let provider = Arc::new(StripeProvider::new(
        "sk_live_from_secret_store",
        "whsec_from_secret_store",
    ));
    Ok(WebhookMiddlewareState::production_with_provider(
        provider, replay,
    ))
}
```

Run equivalent reviewed DDL through deployment migrations instead of relying
on request-time setup. Capacity/TTL are immutable for an existing ledger;
drift, corruption, storage failure, and a full unexpired ledger fail closed.
Only provider-scoped SHA-256 claims are stored, not raw payloads or event IDs.

SQL-backed middleware claims the payload before handler dispatch. It prevents
cross-process replay but cannot make handler delivery exactly once: a crash can
still occur between admission and a business mutation. For an atomic
relational path, verify the exact provider payload through its low-level
contract, select the provider's stable event ID, and call
`check_and_record_event_key_with_transaction` in the same transaction as the
domain mutation. Do not also pre-claim that event through SQL middleware.
External calls, e-mail, and queues still need an outbox, idempotent consumers,
and reconciliation.

### 4. Provider-Specific Metered Usage

Use `MeteredBillingProvider` with `StripeMeterEvent`. The Stripe request
carries customer, configured event name, positive value, bounded timestamp and
an identifier forwarded to the provider and HTTP idempotency header.

Stripe's provider identifier has only a rolling uniqueness window, so keep a
durable application record of submitted events and make reconciliation
idempotent. A custom adapter whose provider has no deduplication key returns
`UsageDeduplication::ApplicationOutboxRequired`; claim the event key durably
before sending. Protocol fixtures verify request/response shape and bounds;
they do not replace live provider-account testing.

### 5. Payment-Bound PDF Invoice Delivery

With the umbrella `capital-mail` feature, bind an authoritative invoice to the
final charge receipt and prepare a pipeline-validated HTML/PDF message through
`rullst::mail::PaidInvoiceDelivery`. Non-final/mock receipts and mismatched
recipient, minor-unit total or currency fail before delivery. Persist the
stable delivery key under a unique constraint before calling `send`; the
bridge is at-least-once and does not infer webhook reconciliation.

The complete runnable shape and its outbox boundary are shown in
[Tutorial 19](tutorials/19-saas-billing-capital.md#4-render-and-deliver-the-invoice-only-after-final-success).

### 6. Adding another gateway

Rullst no longer ships adapters for other gateways or payouts. Implement
`BillingProvider` for the gateway in application code, declare its webhook
verification mode, verify the provider's exact signed bytes inside
`handle_webhook`, and mount it behind the same canonical middleware with
`WebhookMiddlewareState::production_with_provider`. The
[custom provider guide](capital-custom-provider.md) walks through a compiling
example. Payouts, transfers and fiscal documents are application or
separate-product responsibilities.

---

## 🛡️ Security controls and boundaries

1. **Bounded verification:** webhook handlers should bound the body before
   parsing and reject a missing or malformed signature. Reading and parsing still
   allocate according to the concrete HTTP stack and payload.
2. **Cryptographic verification:** supported webhook adapters use HMAC or
   constant-time verification for the exact signed bytes. Each provider's
   timestamp/replay policy and deployed secret lifecycle still require review.
   The default replay store is process-local; multi-instance deployments need
   the opt-in `webhook-sql` ledger or another durable shared idempotency
   boundary. It holds at
   most 10,000 proofs for 24 hours each and answers 503 when full rather than
   evict an unexpired proof; size a store for `verify_webhook_with_state` when
   one process verifies more than about 10,000 deliveries per day.
3. **Typed parsing:** supported provider responses map into Rust enums and
   structs without runtime reflection. A typed response does not establish
   authorization, idempotency, or correctness of the upstream service.
