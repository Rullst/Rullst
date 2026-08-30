# Rullst Capital 💳

`rullst-capital` provides payment/payout adapter foundations, normalized billing
types, bounded webhook verification helpers, application-supplied revenue
snapshots, and an offline-only NFS-e preview. Provider method coverage is not
uniform; inspect the selected adapter and test it in the provider sandbox.

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

---

## ✨ Supported Providers

| Provider | Adapter category | Current boundary |
| :--- | :--- | :--- |
| **Stripe** | Billing | Checkout and documented webhook foundations; verify required live methods. |
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
rullst-capital = "12.0.0"
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
}

fn has_pro_access(workspace: &Workspace) -> bool {
    workspace.can_access("pro")
}
```

`Billable` does not infer membership, usage from a database, currency or a
payment method. Applications must establish those identities before invoking a
provider operation.

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

`rullst-capital` includes an Axum middleware [`verify_webhook`](https://github.com/Rullst/Rullst/blob/main/rullst-capital/src/webhook.rs) that verifies supported provider signatures, enforces timestamp freshness for Stripe, Mercado Pago and Paddle, and rejects replayed payloads through a bounded TTL store. Empty webhook secrets are configuration errors. `mock_*` secrets are explicit local fixtures and are rejected by this production-safe middleware.

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

---

## 🧾 NFS-e Padrão Nacional — Contained Preview

The crate can build an escaped DPS XML fixture, but live NFS-e issuance is intentionally fail-closed. `Homologation` and `Production` return `FiscalError::Unsupported` until PKCS#12 key extraction, XML C14N/XMLDSig, XSD validation, mTLS, strict response parsing and official end-to-end homologation are independently verified. The legacy `sign_dps_xml` entry point never fabricates a signature.

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
let cert = FiscalCertificate::from_base64("", "");

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
- **Fiscal Containment**: No XMLDSig or official NFS-e authorization is claimed until the complete official integration is validated.
