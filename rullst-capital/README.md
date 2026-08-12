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

`rullst-capital` includes an Axum middleware [`verify_webhook`](src/webhook.rs) that cryptographically verifies HMAC signatures using constant-time comparisons (`subtle::ConstantTimeEq`), parsing payloads into unified [`WebhookEvent`](src/providers/mod.rs) structs.

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

## 🔐 Security Invariants

- **Constant-Time Verification**: All HMAC-SHA256 signatures are validated with `subtle::ConstantTimeEq` to prevent side-channel timing attacks.
- **Zero Allocations on Invalid Signatures**: Requests missing or with corrupted signature headers are rejected with `401 Unauthorized` before passing down the middleware pipeline.
