# Rullst Capital 💰
### *"Enterprise Multi-Gateway Billing, SaaS Analytics & Fiscal Engine"*

`rullst-capital` provides a unified financial foundation for SaaS, digital commerce, and marketplace platforms written in Rust. It includes multi-provider adapter surfaces, recurring-subscription models, international payout helpers, and a bounded Brazilian National NFS-e preparation pipeline. Live provider and fiscal production readiness must be established per adapter and environment.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Direct Gateways** | 🟠 `[Partial]` | 11 payment/payout adapter surfaces with pooled HTTP clients and deterministic mocks. Live method coverage, provider acceptance tests, retry semantics, and reconciliation are not uniform yet. |
| **Subscription Lifecycle** | 🟠 `[Partial]` | Checkout, portal, cancellation, pause, usage, coupon, trial, status, and webhook APIs exist, but not every provider implements and verifies every method end-to-end. |
| **Webhook Processing** | 🟢 `[Implemented / Bounded]` | Axum and opt-in Actix middleware call one canonical bounded verifier; named adapters implement signature verification and freshness checks. Cross-instance durable replay/idempotency remains application or future framework work. Alipay RSA2 remains fail-closed. |
| **Paid Invoice Rendering** | 🟢 `[Implemented / Feature-gated]` | Exact validated minor units, escaped HTML, bounded paginated A4 PDF and a final-success e-mail/amount/currency binding. The downstream Mail bridge sends the attachment but durable outbox claiming and exactly-once delivery remain application work. |
| **SaaS MRR/ARR Analytics** | 🟢 `[Implemented / Bounded]` | In-memory revenue metrics and churn calculations for supplied records; this is not an accounting ledger or provider reconciliation engine. |
| **NFS-e 1.01 Local Pipeline** | 🟢 `[Implemented / Bounded]` | Strict ordinary-service DPS builder, checksum-pinned closed-catalog validation of official XSD sources with one exact documented production regex-anchor compatibility normalization, protected PKCS#12 RSA-SHA256/inclusive-C14N XMLDSig, independent local signature verification, deterministic `dpsXmlGZipB64` request JSON, bounded signed-authorization and structured-rejection parsing, and bounded rustls mTLS client construction. |
| **NFS-e Offline Sandbox** | 🟡 `[Offline Mock]` | Deterministic offline mock fixtures (`NfseEnvironment::Mock`) for local development and CI testing. |
| **SEFIN Live NFS-e Homologation** | 🔵 `[Roadmap / External Evidence]` | Full emitter/ICP-Brasil certificate policy, durable idempotency/audit, retained official protocol fixtures, real A1 restricted-environment tests, independent review, and official homologation. Transmission is disabled. |

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

### 2. Payment-Bound Invoice PDF and Mail

Enable `rullst/capital-mail` or the separate `rullst-capital/invoice-pdf` and
`rullst-mail/capital-invoice` features. A `PaidInvoice` can be constructed only
from final `Succeeded` evidence matching the invoice recipient, exact
minor-unit total and currency. `PaidInvoiceDelivery::prepare` generates escaped
HTML and a bounded PDF attachment and runs Mail's mandatory pre-flight.

The stable delivery key is an application outbox identity, not a distributed
lock. The application must reconcile webhooks, claim that key atomically and
own at-least-once retries/provider attachment policy.

### 3. Verified Webhook Signature Handling

The low-level provider contract below illustrates exact-byte verification. HTTP
applications should normally mount `verify_webhook` on Axum or
`verify_webhook_actix_with_state` on Actix so body limits, normalized event
insertion, and local replay rejection are applied before the handler. Webhooks
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

---

## 🏛️ Brazilian Digital Invoicing (NFS-e Nacional)

`rullst-capital` includes a dedicated fiscal module (`rullst_capital::fiscal`)
shaped around the National NFS-e domain. Its local schema, signature, bounded
issuance-codec, and mTLS preparation contracts are implemented and tested; it
is not yet an officially homologated issuer.

Enable `rullst-capital/nfse` (or umbrella `rullst/capital-nfse`) for the pinned
XSD, XMLDSig, GZip/Base64 protocol codec, and mTLS preparation dependencies.
Selecting the feature does not enable SEFIN transmission.

### Architecture & Pipeline

```
[SaaS Sale] ─► [NfseDpsV101] ─► [Pinned XSD] ─► [PKCS#12 XMLDSig] ─► [Bounded JSON codec]
                    │                                                    │
                    ▼                                                    ▼
          [Offline deterministic fixture]                 [mTLS prepared; transmission disabled]
```

### Emitting an Invoicing Document (DPS)

```rust
use rullst_capital::fiscal::{
    build_dps_xml_v1_01, FiscalCustomer, FiscalEmitter, IssRetention,
    IssTaxation, NfseDpsV101, NfseEnvironment, TaxRegime,
};
use chrono::{NaiveDate, Utc};

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

let dps = NfseDpsV101 {
    id: "DPS355030821122233300018100001000000000000101".to_string(),
    series: "1".to_string(),
    number: 101,
    issued_at: Utc::now(),
    competence_date: NaiveDate::from_ymd_opt(2026, 8, 30).ok_or("invalid date")?,
    service_code: "010301".to_string(),
    description: "Assinatura Mensal SaaS Rullst Pro".to_string(),
    amount_cents: 9_900,
    iss_rate_basis_points: Some(200),
    iss_taxation: IssTaxation::Taxable,
    iss_retention: IssRetention::NotRetained,
    service_city_ibge: "3550308".to_string(),
};

let unsigned_xml = build_dps_xml_v1_01(
    &emitter,
    &customer,
    &dps,
    NfseEnvironment::Homologation,
)?;
```

See [Preparing a National NFS-e 1.01 homologation
candidate](../tutorials/40-nfse-homologation-preparation.md) for pinned artifact
validation, local signing, and the external gates that still prevent live
transmission.

---

## 🔒 Security Invariants

1. **Constant-Time Verification:** Webhook signatures use `subtle::ConstantTimeEq` to prevent side-channel timing attacks.
2. **Fail-Closed Live Modes:** Local XMLDSig/XSD/codec/mTLS preparation does not enable a request. `Homologation` and `Production` return a typed `FiscalError::Unsupported` without network I/O until the external trust, audit and homologation gates pass.
3. **No Phantom Persistences:** All provider drivers use explicit connection pooling (`reqwest::Client`) with keep-alive to avoid socket storms.
