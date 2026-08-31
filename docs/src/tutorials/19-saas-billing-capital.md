# Tutorial 19: SaaS billing with Rullst Capital 💳

This tutorial creates a checkout adapter boundary and explains how to feed the
optional local Studio view without treating it as an accounting system.

## 1. Scaffold and configure the selected adapter

```bash
cargo rullst make:billing --model Workspace
```

The command detects a relational SQLx or Turso-primary project, adds the exact
`orm` and `capital` facade features once, generates a reversible matching
migration, registers the models/controller/page modules, and refuses to
overwrite an earlier billing scaffold. The generated runtime supports the
selected `stripe` or `lemonsqueezy` adapter. It deliberately does not imply
support for every Capital adapter or mount application routes without review.

Set `BILLING_ALLOWED_PLAN_IDS` to the exact comma-separated provider price or
variant IDs the server may accept. Production startup rejects a missing
allowlist; a query-string plan outside it is denied before creating a billing
customer.

Use the exact environment names emitted by the generated files and keep live
credentials outside source control. Credentials beginning with `mock_` select a
documented deterministic offline path; they are not accepted by the
production-safe webhook middleware.

## 2. Create a checkout session

```rust,no_run
use rullst_capital::{init_provider, provider, StripeProvider};

async fn checkout_url() -> Result<String, String> {
    let api_key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|error| format!("missing Stripe key: {error}"))?;
    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .map_err(|error| format!("missing webhook secret: {error}"))?;

    init_provider(Box::new(StripeProvider::new(api_key, webhook_secret)));
    let selected = provider().ok_or_else(|| "billing provider is not configured".to_string())?;
    selected
        .create_checkout_session(
            "customer@example.com",
            "price_pro_monthly",
            "https://app.example/billing/success",
        )
        .await
        .map_err(|error| error.to_string())
}
```

The scaffold requires an authenticated `BillingIdentity` for checkout/portal,
enforces that server-owned plan allowlist, and rejects subscription reuse across
owners. The application still owns the identity middleware, correct plan
configuration, return-URL policy, durable provider-event
idempotency/reconciliation, and provider sandbox validation.

## 3. Make a bounded immediate charge when checkout is not the right flow

For a payment method already tokenized and authorized for off-session reuse at
Stripe, the model deriving `Billable` can perform one fully specified charge:

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn charge_saved_method(
    account: &impl rullst_capital::Billable,
    stripe: &StripeProvider,
) -> Result<String, CapitalError> {
    let receipt = account
        .charge_with(
            stripe,
            2_500,
            "BRL",
            "cus_from_authoritative_state",
            "pm_from_authoritative_state",
            "order_2026_0001-attempt_1",
        )
        .await?;
    Ok(receipt.charge_id().to_string())
}
```

This deliberately is not `charge(amount)`: currency, provider customer,
tokenized payment method and retry identity cannot be inferred safely. The
Stripe adapter uses Payment Intents with immediate off-session confirmation and
the provider idempotency header. Only `succeeded` and `processing` are accepted;
an amount/currency mismatch or a flow requiring customer action fails closed.
Credentials beginning with `mock_` return the same deterministic receipt with
the distinct non-success `ChargeStatus::Mock` for an exact retry. The mock is
not a mandate, durable idempotency store or live sandbox test. Other adapters
return `UnsupportedOperation` until reviewed individually.

## 4. Render and deliver the invoice only after final success

Enable `capital-mail` on the umbrella crate (or `invoice-pdf` on Capital plus
`capital-invoice` on Mail). Build the invoice from authoritative order state,
then bind it to the returned receipt:

```rust,no_run
use chrono::Utc;
use rullst::capital::{ChargeReceipt, Invoice, InvoiceItem};
use rullst::mail::PaidInvoiceDelivery;

async fn deliver_invoice(receipt: &ChargeReceipt) -> Result<(), Box<dyn std::error::Error>> {
    let invoice = Invoice {
        invoice_id: "INV-2026-0001".to_string(),
        customer_email: "customer@example.com".to_string(),
        date: Utc::now(),
        items: vec![InvoiceItem {
            description: "Pro subscription".to_string(),
            amount: 25.00,
        }],
        total: 25.00,
        currency: "BRL".to_string(),
    };

    let paid = invoice.bind_succeeded_charge(receipt)?;
    let delivery = PaidInvoiceDelivery::prepare(&paid)?;

    // In production, atomically claim this stable key in a durable outbox.
    let _delivery_key = delivery.delivery_key();
    delivery.send().await?;
    Ok(())
}
```

The binding rejects `Processing`, `Mock`, a mismatched recipient, amount or
currency. The default PDF is paginated, bounded to sixteen MiB and supports
WinAnsi text (including common Portuguese characters); pass a checked TTF/OTF
to Capital for other scripts. Mail applies its mandatory pre-flight before the
facade queues or sends the HTML message and attachment.

This helper does not subscribe to webhooks by itself. Reconcile the provider
event, build the authoritative invoice and insert `delivery_key` under a unique
database constraint in the same application workflow. Mail delivery remains
at least once: a crash and retry can still require provider/application
deduplication.

## 5. Verify webhooks before business processing

Mount `rullst_capital::verify_webhook` on the exact provider callback route as
shown in the [Capital crate guide](../5-rullst-capital.md). Apply any CSRF
exemption only to that exact signed route. Never update access or subscription
state from an unverified request.

## 6. Use a bounded subscription handle and grace period

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, GracePeriod, StripeProvider};

async fn pause_with_local_policy(
    workspace: &impl rullst_capital::Billable,
    provider: &StripeProvider,
) -> Result<(), CapitalError> {
    let grace = GracePeriod::new(1_900_000_000, 1_900_604_800)?;
    let handle = workspace
        .subscription_with(provider)?
        .with_grace_period(grace);
    handle.pause().await
}
```

The grace value does not schedule the pause or grant access by itself. Persist
it with authoritative subscription state, evaluate it against a trusted clock
inside the entitlement check, and confirm the selected adapter's live pause or
cancel semantics.

## 7. Supply an optional local revenue snapshot

`RevenueDashboardManager` does not derive money or subscribers from event names.
After durable reconciliation, the application may call `update_metrics` with its
authoritative snapshot and `record_event` with a bounded inspection record. The
standalone Studio can display that process-local source at `/studio/capital`
when it is explicitly connected. It neither auto-discovers webhook routes nor
auto-syncs the application database.

Provider-hosted checkout normally keeps card collection away from the
application, but the final data boundary depends on the selected provider flow,
application logs, analytics, and deployment.
