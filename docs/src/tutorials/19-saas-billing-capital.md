# Tutorial 19: SaaS billing with Rullst Capital 💳

This tutorial creates a checkout adapter boundary and explains how to feed the
optional local Studio view without treating it as an accounting system.

## 1. Scaffold and configure the selected adapter

```bash
cargo rullst make:billing
```

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

The application still owns authenticated customer identity, return-URL policy,
durable subscription state, idempotent reconciliation, and provider sandbox
validation.

## 3. Verify webhooks before business processing

Mount `rullst_capital::verify_webhook` on the exact provider callback route as
shown in the [Capital crate guide](../5-rullst-capital.md). Apply any CSRF
exemption only to that exact signed route. Never update access or subscription
state from an unverified request.

## 4. Supply an optional local revenue snapshot

`RevenueDashboardManager` does not derive money or subscribers from event names.
After durable reconciliation, the application may call `update_metrics` with its
authoritative snapshot and `record_event` with a bounded inspection record. The
standalone Studio can display that process-local source at `/studio/capital`
when it is explicitly connected. It neither auto-discovers webhook routes nor
auto-syncs the application database.

Provider-hosted checkout normally keeps card collection away from the
application, but the final data boundary depends on the selected provider flow,
application logs, analytics, and deployment.
