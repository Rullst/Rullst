# Rullst Capital 💳

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io. The version below is the planned first RC, not a published claim.

`rullst-capital` provides payment/payout adapter foundations, normalized billing
types, bounded webhook verification helpers, application-supplied revenue
snapshots, and a bounded National NFS-e preparation pipeline. Provider method
coverage is not uniform; inspect the selected adapter and test it in the
provider sandbox.

## 🚀 Core Features

- **Multi-Provider Architecture:** A unified billing surface across global, regional, Web3, and payout adapters. Capabilities vary by provider, and unsupported live operations fail closed.
- **Revenue snapshot (`/studio/capital`):** Displays metrics supplied explicitly
  by the application to a process-local `RevenueDashboardManager`; it is not an
  accounting ledger and does not infer money from event names.
- **Webhook event inspector:** Holds records explicitly passed to the local
  manager. Capital does not connect every webhook route to Studio automatically.
- **Webhook verification:** Provider-specific signature/freshness/replay
  foundations for documented adapters. Durable reconciliation and database
  updates remain application responsibilities.
- **Payment-bound invoices:** The opt-in `invoice-pdf` feature validates money
  into exact minor units, renders bounded paginated PDF and binds delivery to a
  final receipt matching recipient, amount and currency.
- **Provider-specific metered billing:** Current Stripe Meter Events and Lemon
  Squeezy Usage Records request/response contracts, bounded protocol parsing,
  deterministic non-live mocks and explicit retry evidence.
- **Shared team/workspace quotas:** Bounded subject identities, idempotent
  reservations, replay-safe execution and an opt-in transactional SQL store for
  SQLite, PostgreSQL, MySQL and MariaDB.
- **Coupons and relative trials:** A bounded/redacted coupon value, current
  Stripe discount binding, and 1–730-day trial updates for Stripe/Lemon Squeezy
  with explicit-clock retries and fail-closed provider capability boundaries.

---

## ✨ Supported Providers

| Provider | Adapter category | Current boundary |
| :--- | :--- | :--- |
| **Stripe** | Billing | Checkout, bounded immediate Payment Intent charge, and documented webhook foundations; verify required live methods. |
| **Lemon Squeezy** | Billing | Adapter with explicit mock path; verify required live methods. |
| **InfinitePay** | Billing | Regional adapter foundation; pricing and settlement are external contracts. |
| **Polar** | Billing | Adapter foundation; verify provider API coverage. |
| **Paddle** | Billing | Adapter and signed-webhook foundation. |
| **Razorpay** | Billing | Adapter and signed-webhook foundation. |
| **Mercado Pago** | Billing | Adapter and signed-webhook foundation. |
| **Coinbase Commerce** | Billing | Adapter and signed-webhook foundation. |
| **PicPay** | Billing | Adapter foundation; verify provider API coverage. |
| **Wise** | Payout | Payout adapter foundation rather than a subscription provider. |

---

## 🚀 Quickstart

Add `rullst-capital` to your `Cargo.toml`:

```toml
[dependencies]
rullst-capital = "12.0.0-rc.1"
```

The heavier NFS-e schema/signature boundary is opt-in:

```toml
rullst-capital = { version = "12.0.0-rc.1", features = ["nfse"] }
```

Native invoice PDF is independently opt-in. One-call Mail delivery uses the
downstream `rullst-mail/capital-invoice` feature, or `rullst/capital-mail` when
using the umbrella crate:

```toml
rullst = { version = "12.0.0-rc.1", features = ["capital-mail"] }
```

Durable relational quota accounting is separately opt-in:

```toml
rullst = { version = "12.0.0-rc.1", features = ["capital-quota-sql"] }
# Or directly: rullst-capital = { version = "12.0.0-rc.1", features = ["quota-sql"] }
```

Applications using the umbrella crate can derive the bounded billing facade on
any named struct with an `email: String` field. Optional
`subscription_id: Option<String>` and `tier: Option<String>` fields enable the
corresponding helpers; provider initialization remains explicit:

```rust
use rullst::capital::Billable as _;

#[derive(rullst::Billable)]
struct Workspace {
    email: String,
    subscription_id: Option<String>,
    tier: Option<String>,
    grace_period_starts_at: Option<i64>,
    grace_period_ends_at: Option<i64>,
}

fn has_pro_access(workspace: &Workspace) -> bool {
    workspace.can_access("pro")
}
```

`Billable::subscription_with(&provider)` returns a statically dispatched
`SubscriptionHandle` with `cancel()` and `pause()`. When both grace-period
fields are present, the derive exposes a validated half-open window of at most
366 days; an incomplete pair is a compile error. `Billable` does not persist or
authorize provider state, schedule provider changes, infer membership, or
choose currency/payment methods. Its explicit `quota_request` helper can derive
a limit from the model's `tier_limit`; a separately configured quota store
performs the accounting. Applications must establish identities and policies
before invoking either boundary.

### Shared Team and Workspace Quotas

Use one `BillingSubject` for the authenticated tenant/workspace so every member
consumes the same limit. `Billable::quota_request` derives the limit from the
subscription owner's tier rather than a client payload. `QuotaGate` atomically
reserves before calling the application operation, skips exact idempotent
replays and releases a fresh reservation when the callback returns an error.

The always-available `InMemoryQuotaStore` is deterministic and process-local.
With `quota-sql`, `SqlQuotaStore` persists a unique event claim and conditionally
increments the shared counter on SQLite, PostgreSQL, MySQL or MariaDB. For a
relational create that must be atomic with accounting, open a transaction from
`store.pool()`, call `reserve_with_transaction`, execute the domain insert on
that same transaction and commit once. See the
[SaaS billing tutorial](https://rullst.github.io/Rullst/book/tutorials/19-saas-billing-capital.html#8-enforce-one-shared-workspace-quota-before-creation)
for the complete flow.

Membership/authentication, tier persistence and webhook reconciliation,
migrations, cleanup policy for abandoned standalone reservations, and
Turso/NoSQL adapters remain application responsibilities. Writes outside the
gate are not intercepted automatically.

An immediate charge is available without exposing a raw-card field. It requires
minor units, currency, authoritative provider customer/payment-method IDs and a
unique application retry key:

```rust
use rullst::capital::{Billable as _, CapitalError, StripeProvider};

async fn collect(
    workspace: &impl rullst::capital::Billable,
    stripe: &StripeProvider,
) -> Result<(), CapitalError> {
    let receipt = workspace
        .charge_with(
            stripe,
            4_990,
            "BRL",
            "cus_provider_owned",
            "pm_provider_tokenized",
            "order_42-attempt_1",
        )
        .await?;
    assert_eq!(receipt.amount_minor(), 4_990);
    Ok(())
}
```

Stripe is the only reviewed live direct-charge adapter. Exact `mock_*` retries
are deterministic but carry the distinct non-success `ChargeStatus::Mock`;
other adapters return `UnsupportedOperation`. Mandate/SCA,
durable idempotency, webhook reconciliation and entitlement changes remain
application responsibilities.

### Coupons and Relative Trials

The provider-bound handle validates coupon IDs before dispatch. Stripe uses the
current expanded subscription-discount update and checks that the response
contains both the requested subscription and coupon. Lemon Squeezy discount
codes are checkout-only, so applying one to an existing live subscription
returns `UnsupportedOperation`; unreviewed live adapters do the same.

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn retention_offer(
    workspace: &impl rullst_capital::Billable,
    stripe: &StripeProvider,
    command_created_at: i64,
) -> Result<(), CapitalError> {
    let subscription = workspace.subscription_with(stripe)?;
    subscription.apply_coupon("RETENTION_25").await?;
    subscription
        .extend_trial_days_at(15, command_created_at)
        .await
}
```

`extend_trial(15)` uses current UTC for convenience; persist a trusted command
time and use `extend_trial_days_at` for retry stability. `set_trial_end` remains
the explicit absolute operation. Stripe and Lemon Squeezy bind trial-update
responses, but authorization, concurrent-command serialization, billing-cycle
policy, signed-webhook reconciliation and real-account acceptance remain host
or release work.

### Provider-Specific Metered Usage

Use `MeteredBillingProvider` instead of the legacy uniform `report_usage`
method. Stripe requires a customer, configured event name, timestamp and
provider-forwarded identifier:

```rust,no_run
use rullst_capital::{
    CapitalError, MeteredBillingProvider as _, StripeMeterEvent, StripeProvider,
};

async fn report_lesson_minutes(stripe: &StripeProvider) -> Result<(), CapitalError> {
    let event = StripeMeterEvent::new(
        "cus_from_authoritative_state",
        "lesson_minutes",
        15,
        "usage:school-7:attempt-99",
    )?;
    let receipt = stripe.report_metered_usage(&event).await?;
    if receipt.is_live_accepted() {
        // Reconcile the provider meter; do not infer an entitlement from this alone.
    }
    Ok(())
}
```

`LemonSqueezyUsageRecord` instead requires the provider's numeric subscription
item ID and an explicit `Increment` or `Set` action. The action must match the
aggregation configured for that variant. Lemon Squeezy's request does not carry
the application's event key, so atomically claim `event_key()` in a durable
outbox before submission. Stripe's identifier is provider-forwarded but only
has a rolling deduplication guarantee. Empty or `mock_*` keys return a stable
`UsageStatus::Mock`, never a live acceptance.

### Payment-Bound Invoice Delivery

`Invoice::bind_succeeded_charge` accepts only final `Succeeded` evidence with
an exact recipient, minor-unit amount and currency match. The resulting
`PaidInvoice` can be rendered as escaped HTML or a bounded A4 PDF. Mail's opt-in
`PaidInvoiceDelivery` bridge attaches both formats, runs mandatory pre-flight
and sends through the configured facade, a tenant route or an explicit static
driver.

Persist and atomically claim `PaidInvoice::delivery_key()` in an application
outbox before retryable delivery. The bridge does not infer webhook state or
promise provider acceptance/exactly-once behavior. See the
[SaaS billing tutorial](https://rullst.github.io/Rullst/book/tutorials/19-saas-billing-capital.html#4-render-and-deliver-the-invoice-only-after-final-success).

### Initializing a Provider

```rust
use rullst_capital::{init_provider, StripeProvider};

fn configure_billing() -> Result<(), std::env::VarError> {
    let api_key = std::env::var("STRIPE_SECRET_KEY")?;
    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")?;
    init_provider(Box::new(StripeProvider::new(api_key, webhook_secret)));
    Ok(())
}
```

### Creating Checkout Sessions

```rust
use rullst_capital::provider;

async fn checkout_handler() -> Result<String, String> {
    if let Some(p) = provider() {
        let checkout_url = p.create_checkout_session(
            "customer@example.com",
            "plan_pro_monthly",
            "https://mysaas.com/billing/success",
        ).await?;
        
        Ok(checkout_url)
    } else {
        Err("No billing provider configured".to_string())
    }
}
```

### Intercepting and Verifying Webhooks

`rullst-capital` includes Axum and opt-in Actix Web middleware adapters over one canonical [`webhook` verifier](https://github.com/Rullst/Rullst/blob/main/rullst-capital/src/webhook.rs). Both bound the body, verify supported provider signatures, enforce timestamp freshness for Stripe, Mercado Pago and Paddle, restore the exact body, insert a normalized event, and reject replayed payloads through a bounded TTL store. Empty webhook secrets are configuration errors. `mock_*` secrets are explicit local fixtures and are rejected by the production-safe entry points.

The webhook route must receive a narrowly scoped CSRF exemption in the application router; never disable CSRF for browser routes. The exemption is safe only when this signature/freshness/replay middleware remains mandatory on that exact route. An outer blanket CSRF layer will reject legitimate provider callbacks before Capital can verify them.

```rust
use axum::{Router, routing::post, Extension};
use rullst_capital::{verify_webhook, WebhookEvent, SubscriptionStatus};

async fn handle_webhook_event(Extension(event): Extension<WebhookEvent>) {
    match event.status {
        SubscriptionStatus::Active => {
            println!("✅ Subscription active for customer: {}", event.customer_email);
        }
        SubscriptionStatus::Canceled => {
            println!("⚠️ Subscription canceled: {}", event.subscription_id);
        }
        SubscriptionStatus::PastDue => {
            println!("🚨 Payment past due for customer: {}", event.customer_email);
        }
        _ => {}
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/webhooks/billing", post(handle_webhook_event))
        .layer(axum::middleware::from_fn(verify_webhook))
}
```

For Actix Web, enable the crate's `actix` feature (or umbrella
`rullst/capital-actix`) and mount
`actix_web::middleware::from_fn(verify_webhook_actix_with_state)` with a
`web::Data<WebhookMiddlewareState>`. The state can bind an explicit provider
through `WebhookMiddlewareState::production_with_provider`, avoiding global
configuration. See the
[payment guide](https://rullst.github.io/payment-gateways-guide.html#actix-web-adapter)
for a complete example.

The bundled replay store is process-local. Multi-instance deployments must
claim the provider event ID in shared durable state before idempotent billing
side effects.

---

## 🧾 NFS-e Padrão Nacional — Homologation Preparation

The local pipeline now implements a bounded ordinary-service DPS 1.01 builder,
checksum-pinned validation against official production/restricted XSD sources,
PKCS#12 RSA-SHA256 XMLDSig with inclusive C14N 1.0, independent local
signature verification, deterministic GZip/Base64 issuance JSON, bounded
signed-authorization and structured-rejection parsing, and rustls mTLS client
construction.
Certificate bytes, passphrases, and derived PEM are redacted and zeroized where
owned by Rullst.
The production profile applies one exact, documented in-memory compatibility
normalization after hash verification: it removes `.NET` `^...$` anchors from
the known DPS-series pattern because XSD regex grammar treats them as literals.

This is preparation for homologation, not live issuance. `Homologation` and
`Production` still return `FiscalError::Unsupported` without network I/O until
full certificate/emitter and ICP-Brasil policy, durable idempotency/audit,
retained official protocol fixtures, real restricted-environment evidence,
independent review, and official homologation are complete.

Enable the crate's `nfse` feature (or umbrella `rullst/capital-nfse`) for the
XSD, XMLDSig, protocol codec, and mTLS preparation APIs. The strict DPS builder
and unmistakable offline mock remain available through the base Capital crate.

The runnable [`nfse_v101_preview`](examples/nfse_v101_preview.rs) example emits
the unsigned bounded DPS. When `RULLST_NFSE_XSD_DIR` points to an extracted
official production package whose files match the pinned hashes, it validates
the document before writing it:

```bash
RULLST_NFSE_XSD_DIR=/path/to/NFSe/Schemas/1.01 \
  cargo run -p rullst-capital --example nfse_v101_preview
```

Only `NfseEnvironment::Mock` is executable. Its response is typed as `FiscalResponseKind::OfflineMock`, uses `MOCK_NOT_AUTHORIZED`, and must never be accounted as an issued invoice:

```rust
use rullst_capital::fiscal::{
    issue_nfse_direct, FiscalCertificate, FiscalCustomer, FiscalEmitter,
    FiscalResponseKind, NfseEnvironment, TaxRegime,
};

// 1. Configure the emitting SaaS company
let emitter = FiscalEmitter {
    cnpj: "12.345.678/0001-90".to_string(),
    inscricao_municipal: "1234567".to_string(),
    legal_name: "Minha Empresa SaaS Ltda".to_string(),
    trade_name: Some("MeuSaaS".to_string()),
    ibge_code: "3550308".to_string(), // São Paulo
    tax_regime: TaxRegime::SimplesNacional,
};

// 2. Customer data
let customer = FiscalCustomer {
    doc_number: "123.456.789-00".to_string(),
    name: "João Silva".to_string(),
    email: "joao@cliente.com.br".to_string(),
    zip_code: Some("01310-100".to_string()),
    address: Some("Av Paulista, 1000".to_string()),
    ibge_code: Some("3550308".to_string()),
};

// 3. Convert paid invoice to national DPS format
let dps = invoice.to_dps("1.03.01", "3550308", 2.0); // 1.03.01 = SaaS & Hosting, 2.0% ISS

// 4. Mock mode does not load or use a real certificate.
let cert = FiscalCertificate::offline_mock();

// 5. Produce a deterministic offline fixture; no network request is made.
let response = issue_nfse_direct(
    &emitter,
    &customer,
    &dps,
    &cert,
    NfseEnvironment::Mock,
).await?;
assert_eq!(response.kind, FiscalResponseKind::OfflineMock);
assert!(!response.is_officially_authorized());
```

---

## 🔐 Security Invariants

- **Constant-Time Verification**: Supported HMAC/token signatures use cryptographic verification or `subtle::ConstantTimeEq`.
- **Fail-Closed Configuration**: Empty webhook secrets never authenticate a request; mock credentials require a deliberate `mock_*` value.
- **Freshness and Replay Protection**: Timestamped protocols have a configurable five-minute window, and middleware records provider-scoped payload hashes in a bounded 24-hour TTL store.
- **Alipay Containment**: Live RSA2 checkout and webhook verification return `UnsupportedOperation`; only explicitly mock-prefixed credentials operate offline.
- **Fiscal Containment**: Local XSD/XMLDSig/mTLS preparation is not an official NFS-e authorization; live transmission remains disabled until the documented external gates pass.
