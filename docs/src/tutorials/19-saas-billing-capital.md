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

## 3. Verify webhooks before business processing

Mount `rullst_capital::verify_webhook` on the exact provider callback route as
shown in the [Capital crate guide](../5-rullst-capital.md). Apply any CSRF
exemption only to that exact signed route. Never update access or subscription
state from an unverified request.

## 4. Use a bounded subscription handle and grace period

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

## 5. Supply an optional local revenue snapshot

`RevenueDashboardManager` does not derive money or subscribers from event names.
After durable reconciliation, the application may call `update_metrics` with its
authoritative snapshot and `record_event` with a bounded inspection record. The
standalone Studio can display that process-local source at `/studio/capital`
when it is explicitly connected. It neither auto-discovers webhook routes nor
auto-syncs the application database.

Provider-hosted checkout normally keeps card collection away from the
application, but the final data boundary depends on the selected provider flow,
application logs, analytics, and deployment.
