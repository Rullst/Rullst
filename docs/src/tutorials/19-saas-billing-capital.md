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

### 2.1 Handle provider failure without blindly repeating a charge

Built-in live adapters return a redacted `CapitalError::Provider` for outbound
request construction, transport, HTTP status, bounded-response, JSON, and
response-contract failures. Inspect its stable class for telemetry or a durable
job decision:

```rust
use rullst_capital::{CapitalError, ProviderFailureClass};

fn provider_disposition(error: &CapitalError) -> Option<ProviderFailureClass> {
    match error {
        CapitalError::Provider(failure) => Some(failure.class()),
        _ => None,
    }
}
```

Do not turn `Transient` or `RateLimited` into an unconditional loop. Persist the
original command and idempotency identity, confirm that the selected operation
actually forwards that identity, cap attempts with backoff/jitter, and
reconcile signed provider events. Checkout creation in the legacy unified trait
does not accept an application idempotency key, so reconcile before repeating
it. The shared client already applies finite timeouts, disables redirects and
ambient proxies, caps JSON to one MiB, and validates returned checkout URLs as
absolute credential-free HTTPS.

## 3. Make a bounded immediate charge when checkout is not the right flow

For a payment method already tokenized and authorized for off-session reuse at
Stripe, the model deriving `Billable` can perform one fully specified charge:

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn charge_saved_method(
    account: &(impl rullst_capital::Billable + Sync),
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

## 5. Report metered usage without confusing provider identities

Use the provider-specific static trait for new code. Stripe Meter Events need a
customer ID and configured event name, not a subscription-item ID:

```rust,no_run
use rullst_capital::{
    CapitalError, MeteredBillingProvider as _, StripeMeterEvent, StripeProvider,
};

async fn report_ai_exercises(stripe: &StripeProvider) -> Result<(), CapitalError> {
    let event = StripeMeterEvent::new(
        "cus_from_authoritative_state",
        "ai_exercises",
        3,
        "usage:school-7:attempt-99",
    )?;
    let receipt = stripe.report_metered_usage(&event).await?;
    assert_eq!(receipt.quantity(), 3);
    Ok(())
}
```

Lemon Squeezy instead needs its numeric subscription-item relationship and an
aggregation action:

```rust,no_run
use rullst_capital::{
    CapitalError, LemonSqueezyProvider, LemonSqueezyUsageAction,
    LemonSqueezyUsageRecord, MeteredBillingProvider as _,
};

async fn report_lesson_minutes(
    lemon: &LemonSqueezyProvider,
) -> Result<(), CapitalError> {
    let record = LemonSqueezyUsageRecord::new(
        "42",
        "lesson_minutes",
        15,
        LemonSqueezyUsageAction::Increment,
        "usage:school-7:lesson-session-123",
    )?;

    // Atomically claim record.event_key() in a durable outbox before this call.
    let receipt = lemon.report_metered_usage(&record).await?;
    assert_eq!(receipt.quantity(), 15);
    Ok(())
}
```

Use `Increment` only with a sum-of-usage aggregation and `Set` only with the
matching latest-value aggregation. Stripe receives the identifier but enforces
it only within a rolling window. Lemon's request does not receive the
application event key at all, so durable application deduplication is mandatory.
The adapters cap and bind responses, while provider sandbox/live acceptance,
retry, invoice reconciliation and entitlement updates remain release and
application work. Empty or `mock_*` API keys produce a deterministic
`UsageStatus::Mock`, never billable evidence.

## 6. Verify webhooks before business processing

Mount `rullst_capital::verify_webhook` on the exact provider callback route as
shown in the [Capital crate guide](../5-rullst-capital.md). Apply any CSRF
exemption only to that exact signed route. Never update access or subscription
state from an unverified request.

The default replay store protects one process. For multiple processes,
`rullst-capital/webhook-sql` provides a bounded SQL ledger for SQLite,
PostgreSQL, MySQL, and MariaDB. Its middleware form claims the signed payload
before dispatch and therefore does not guarantee exactly-once processing after
a crash. When a subscription mutation must be atomic, use the provider's
verified stable event ID with
`SqlWebhookReplayStore::check_and_record_event_key_with_transaction` in the
same database transaction as that mutation; do not also pre-claim that event
through SQL middleware. See the [payment guide](../payment-gateways-guide.md#3-cryptographically-verified-webhook-endpoint)
for setup and operational boundaries.

## 7. Use a bounded subscription handle and grace period

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

The same statically dispatched handle validates coupon IDs and gives the
historical relative-trial API its intended meaning:

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn grant_a_retention_offer(
    workspace: &impl rullst_capital::Billable,
    stripe: &StripeProvider,
    command_created_at: i64,
) -> Result<(), CapitalError> {
    let subscription = workspace.subscription_with(stripe)?;
    subscription.apply_coupon("RETENTION_25").await?;

    // Fifteen whole days. Persist command_created_at and reuse it on retries.
    subscription
        .extend_trial_days_at(15, command_created_at)
        .await
}
```

`extend_trial(15)` uses the current UTC time for interactive convenience.
Workers should persist their command time and use `extend_trial_days_at` so a
retry sends the identical expiration; `set_trial_end` is the explicit absolute
timestamp operation. Stripe has the reviewed live coupon path. Lemon Squeezy
discount codes belong to checkout and therefore fail explicitly when applied
to an existing live subscription; both Stripe and Lemon Squeezy have reviewed
trial-update protocol fixtures. Authorize the subscription owner before
building the handle, serialize conflicting updates, and reconcile the signed
provider webhook because trial changes can affect billing anchors and charges.

## 8. Enforce one shared workspace quota before creation

Enable `quota-sql` directly, or `capital-quota-sql` on the umbrella crate. The
authenticated middleware must first establish the active `TenantContext`; do
not build a billing subject from an arbitrary header or request field.

`Billable::quota_request` reads the limit from the subscription owner's
`tier_limit` implementation. Give every attempted creation a stable event key,
normally the ID of the application command/request rather than a random value
generated on every retry.

```rust,no_run
use rullst::{
    capital::{Billable as _, BillingSubject, QuotaError, SqlQuotaStore},
    security::TenantContext,
};

async fn create_project(
    workspace: &impl rullst::capital::Billable,
    tenant: &TenantContext,
    quotas: &SqlQuotaStore,
    project_id: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let subject = BillingSubject::from_tenant(tenant)?;
    let request = workspace.quota_request(
        subject,
        "projects",
        format!("create-project:{project_id}"),
        1,
    )?;

    let mut transaction = quotas.pool().begin().await?;
    let grant = match quotas
        .reserve_with_transaction(&mut transaction, &request)
        .await
    {
        Ok(grant) => grant,
        Err(QuotaError::LimitExceeded { .. }) => {
            transaction.rollback().await?;
            return Ok(false);
        }
        Err(error) => {
            transaction.rollback().await?;
            return Err(error.into());
        }
    };

    if grant.is_replay() {
        transaction.rollback().await?;
        return Ok(true);
    }

    let inserted = rullst::orm::sqlx::query(
        "INSERT INTO projects (id, workspace_id) VALUES (?, ?)",
    )
    .bind(project_id)
    .bind(tenant.tenant_id.as_str())
    .execute(&mut *transaction)
    .await;
    if let Err(error) = inserted {
        transaction.rollback().await?;
        return Err(error.into());
    }
    transaction.commit().await?;
    Ok(true)
}
```

The placeholder above is SQLite/MySQL syntax; use `$1`, `$2` for a raw
PostgreSQL insert, or use the ORM operation that participates in the same
transaction. `SqlQuotaStore` uses a unique event claim plus a conditional
counter update, so concurrent members cannot both pass the last available
unit. An exact retry returns `is_replay()` without consuming again; reusing the
same key with different units or a different limit fails closed.

For work that cannot share the SQL transaction, `QuotaGate::execute` still
blocks the callback before an over-limit/replayed operation and releases the
reservation after an ordinary callback error. A process crash between a
standalone reservation and the external side effect is intentionally
conservative and needs application reconciliation; the framework never risks
exceeding the quota to guess whether that external effect happened.

## 9. Supply an optional local revenue snapshot

`RevenueDashboardManager` does not derive money or subscribers from event names.
After durable reconciliation, the application may call `update_metrics` with its
authoritative snapshot and `record_event` with a bounded inspection record. The
standalone Studio can display that process-local source at `/studio/capital`
when it is explicitly connected. It neither auto-discovers webhook routes nor
auto-syncs the application database.

Provider-hosted checkout normally keeps card collection away from the
application, but the final data boundary depends on the selected provider flow,
application logs, analytics, and deployment.
