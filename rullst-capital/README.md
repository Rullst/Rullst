# Rullst Capital 💳

`rullst-capital` is the native monetization, subscription, and financial billing engine for the Rullst Framework. It abstracts away the complexity of handling global SaaS subscriptions, domestic low-fee gateways, digital wallets, crypto payments, international B2B payouts, and cryptographically verified webhooks.

## 🚀 Core Features

- **Multi-Provider Architecture:** Zero-panic, first-class support for 10 global, regional, and Web3 payment providers. Switch providers effortlessly without rewriting application handlers.
- **Revenue Dashboard (`/studio/capital`):** Native MRR (Monthly Recurring Revenue), ARR (Annual Recurring Revenue), Net Revenue, active subscriber metrics, and churn rate calculations built right into Rullst Studio.
- **Live Webhook Audit Inspector:** Real-time log inspector recording every received payment event payload, signature verification status, and timestamp.
- **Webhook Handling & Database Synchronization:** Secure, constant-time HMAC-verified webhook handlers that listen to subscription creations, renewals, upgrades, and cancellations, automatically updating database records via `rullst-orm`.

---

## ✨ Supported Providers

| Provider | Type / Archetype | Key Features & Strengths |
| :--- | :--- | :--- |
| **Stripe** | Global Direct Merchant | 135+ currencies, Apple/Google Pay, customer portals, metered usage. |
| **Lemon Squeezy** | Global Merchant of Record (MoR) | Automated EU VAT and US state sales tax compliance for global sales. |
| **InfinitePay** | Brazil Domestic Gateway (CloudWalk) | **Pix at 0.00% fee**, instant D+0 payouts, lowest domestic credit card rates (~0.75% to 1.44%) with transparent installment interest pass-through. |
| **Polar.sh** | Developer-First MoR & Open Source | Built specifically for developers, GitHub funding, software licenses, and micro-SaaS. |
| **Paddle** | Enterprise Global MoR | Comprehensive Merchant of Record for European & US B2B SaaS. |
| **Razorpay** | India & Southeast Asia | Recurring UPI autopay, Indian credit cards, net banking, and Asian subscriptions. |
| **Mercado Pago** | Latin America (Regional) | Broadest regional coverage across Brazil, Argentina, Mexico, Chile, and Colombia. |
| **Coinbase Commerce** | Global Web3 / Crypto | Self-custody and hosted crypto charges (BTC, ETH, SOL, USDC/USDT) with automated on-chain webhook verification. |
| **PicPay** | Brazil Digital Wallet & QR Code | Instant consumer wallet and QR-code payments for Brazilian users. |
| **Wise** | Global Multi-Currency Payouts | High-speed, low-fee international B2B payouts and contractor disbursements across 40+ currencies. |

---

## 🚀 Quickstart

Add `rullst-capital` to your `Cargo.toml`:

```toml
[dependencies]
rullst-capital = "12.0.0"
```

### Initializing a Provider

```rust
use rullst_capital::{
    init_provider, StripeProvider, LemonSqueezyProvider, InfinitePayProvider,
    PolarProvider, PaddleProvider, RazorpayProvider, MercadoPagoProvider,
    CoinbaseCommerceProvider, PicPayProvider, WiseProvider,
};

// 1. Stripe (Global Direct)
init_provider(Box::new(StripeProvider::new(
    std::env::var("STRIPE_SECRET_KEY").unwrap(),
    std::env::var("STRIPE_WEBHOOK_SECRET").unwrap(),
)));

// 2. InfinitePay (Brazil - Pix 0% fee)
// init_provider(Box::new(InfinitePayProvider::new(
//     std::env::var("INFINITEPAY_API_KEY").unwrap(),
//     std::env::var("INFINITEPAY_WEBHOOK_SECRET").unwrap(),
// )));

// 3. Polar.sh (Developer-first MoR)
// init_provider(Box::new(PolarProvider::new(
//     std::env::var("POLAR_ACCESS_TOKEN").unwrap(),
//     std::env::var("POLAR_WEBHOOK_SECRET").unwrap(),
// )));
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
