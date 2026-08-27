# Rullst Capital 💰
### *"Enterprise Multi-Gateway Billing, SaaS Analytics & Fiscal Engine"*

`rullst-capital` provides a unified financial foundation for SaaS, digital commerce, and marketplace platforms written in Rust. It includes multi-provider adapter surfaces, recurring-subscription models, international payout helpers, and an offline Brazilian NFS-e DPS preview. Live provider and fiscal production readiness must be established per adapter and environment.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Direct Gateways** | 🟠 `[Partial]` | 11 payment/payout adapter surfaces with pooled HTTP clients and deterministic mocks. Live method coverage, provider acceptance tests, retry semantics, and reconciliation are not uniform yet. |
| **Subscription Lifecycle** | 🟠 `[Partial]` | Checkout, portal, cancellation, pause, usage, coupon, trial, status, and webhook APIs exist, but not every provider implements and verifies every method end-to-end. |
| **Webhook Processing** | 🟢 `[Implemented / Bounded]` | Named adapters implement signature verification and freshness checks; cross-instance durable replay/idempotency remains application or future framework work. Alipay RSA2 remains fail-closed. |
| **SaaS MRR/ARR Analytics** | 🟢 `[Implemented / Bounded]` | In-memory revenue metrics and churn calculations for supplied records; this is not an accounting ledger or provider reconciliation engine. |
| **NFS-e DPS XML Preview** | 🟡 `[Offline Test Mock]` | Escaped DPS XML construction and deterministic offline fixtures; no XMLDSig, transmission, official schema conformance, authorization, or homologation claim. |
| **NFS-e Offline Sandbox** | 🟡 `[Offline Mock]` | Deterministic offline mock fixtures (`NfseEnvironment::Mock`) for local development and CI testing. |
| **SEFIN Live NFS-e Homologation** | 🔵 `[Roadmap]` | W3C XMLDSig signing (C14N canonicalization, RSA-SHA256 with ICP-Brasil A1 PKCS#12) and mTLS transmission to SEFIN. |

---

## 📦 Supported Payment & Payout Providers

`rullst-capital` includes decoupled adapter surfaces for 11 global and regional gateways. The list preserves the intended product reach; it does **not** mean every provider product, fee, payment method, tax promise, or live API path has been independently homologated by Rullst:

1. 💳 **Stripe**: Global card checkouts, Customer Portal, and recurring subscriptions.
2. 🍋 **Lemon Squeezy**: Merchant of Record (MoR) with automated global tax compliance.
3. 🌎 **Mercado Pago**: LATAM subscriptions, Pix, and credit card checkouts.
4. ⚡ **InfinitePay**: Ultra-low-fee domestic Brazilian Pix and installment credit cards.
5. 📱 **PicPay**: Brazilian digital wallet and QR-code checkout flows.
6. 🐻‍❄️ **Polar**: Developer-first MoR for monetizing GitHub repositories and SaaS software.
7. 🛶 **Paddle**: Global B2B SaaS quote-to-cash with EU VAT handling.
8. 🇮🇳 **Razorpay**: Recurring UPI Autopay and credit card orders in India & APAC.
9. 💸 **Wise**: High-speed, multi-currency international contractor payouts (40+ currencies).
10. 🪙 **Coinbase Commerce**: On-chain cryptocurrency payments (Bitcoin, Ethereum, Solana, USDC).
11. 🌏 **Alipay**: Cross-border Chinese digital wallet checkouts (支付宝).

---

## 🚀 Usage Examples

### 1. Initializing a Provider and Creating a Checkout Session

```rust
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

### 2. Verified Webhook Signature Handling

Webhooks enforce constant-time cryptographic verification to eliminate side-channel timing attacks:

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

---

## 🏛️ Brazilian Digital Invoicing (NFS-e Nacional)

`rullst-capital` includes a dedicated offline fiscal preview module (`rullst_capital::fiscal`) shaped around the National NFS-e domain. It is not yet an officially conformant issuer.

### Architecture & Pipeline

```
[SaaS Sale Event] ──► [build_dps_xml()] ──► [XMLDSig C14N Signer (Roadmap)] ──► [SEFIN mTLS (Roadmap)]
                              │
                              ▼
                     [NfseEnvironment::Mock] ──► [Offline Deterministic Fixture]
```

### Emitting an Invoicing Document (DPS)

```rust
use rullst_capital::fiscal::{
    build_dps_xml, issue_nfse_direct, FiscalCertificate, FiscalCustomer,
    FiscalEmitter, NfseDps, NfseEnvironment, TaxRegime,
};
use chrono::Utc;

let emitter = FiscalEmitter {
    cnpj: "12.345.678/0001-90".to_string(),
    inscricao_municipal: "1234567".to_string(),
    legal_name: "Rullst SaaS & Software Ltda".to_string(),
    trade_name: Some("Rullst".to_string()),
    ibge_code: "3550308".to_string(), // São Paulo
    tax_regime: TaxRegime::SimplesNacional,
};

let customer = FiscalCustomer {
    doc_number: "123.456.789-00".to_string(),
    name: "João Silva".to_string(),
    email: "joao@example.com".to_string(),
    zip_code: Some("01310-100".to_string()),
    address: Some("Av Paulista, 1000".to_string()),
    ibge_code: Some("3550308".to_string()),
};

let dps = NfseDps {
    id: "DPS355030800010000000000000000000000000000001".to_string(),
    series: "1".to_string(),
    number: 101,
    issued_at: Utc::now(),
    service_code: "1.03.01".to_string(),
    description: "Assinatura Mensal SaaS Rullst Pro".to_string(),
    amount: 99.00,
    iss_rate: 2.0,
    iss_retained: false,
    service_city_ibge: "3550308".to_string(),
};

let cert = FiscalCertificate::from_base64("MIIKggIBAzCC...", "certificate_password");

// In Development/CI, runs deterministic offline mock
let response = issue_nfse_direct(&emitter, &customer, &dps, &cert, NfseEnvironment::Mock).await?;
println!("Mock Access Key: {}", response.access_key);
```

---

## 🔒 Security Invariants

1. **Constant-Time Verification:** Webhook signatures use `subtle::ConstantTimeEq` to prevent side-channel timing attacks.
2. **Fail-Closed Live Modes:** Unverified live modes (`Homologation` and `Production`) return a typed `FiscalError::Unsupported` until the official XMLDSig C14N and mTLS pipeline is validated end-to-end against the national government portal.
3. **No Phantom Persistences:** All provider drivers use explicit connection pooling (`reqwest::Client`) with keep-alive to avoid socket storms.
