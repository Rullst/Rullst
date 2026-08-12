# Rullst Capital: SaaS Billing & Financial Engine 💳

**Rullst Capital** (`rullst-capital`) is the native monetization, subscription orchestration, revenue analytics, and payment infrastructure layer for the Rullst Framework.

If you generated your project using the `SaaS` blueprint (`cargo rullst new my-saas --blueprint saas`) or ran `cargo rullst make:billing`, your application comes pre-wired with Capital, allowing you to charge users globally, accept Pix with zero fees in Brazil, support developer-first open-source backer tiers, or accept Web3 crypto payments from day one.

---

## 🌟 Supported Payment Gateways

Rullst Capital supports 10 major financial providers out of the box:

- 🌐 **Stripe**: Global direct merchant for cards, Apple Pay, Google Pay, and customer portals.
- 🍋 **Lemon Squeezy**: Global Merchant of Record (MoR) with automated EU VAT and US state sales tax compliance.
- 🇧🇷 **InfinitePay**: Brazil domestic gateway with **Pix at 0.00% fee**, instant D+0 settlement, and lowest credit card rates.
- ⚡ **Polar.sh**: Developer-first Merchant of Record for open-source funding, software licenses, and micro-SaaS.
- 🛡️ **Paddle**: Enterprise Merchant of Record for global B2B SaaS.
- 🇮🇳 **Razorpay**: Dominant payment gateway across India and Southeast Asia for UPI, cards, and subscriptions.
- 🌎 **Mercado Pago**: Broadest Latin American regional coverage (Argentina, Mexico, Chile, Colombia, Brazil).
- ₿ **Coinbase Commerce**: Borderless Web3 crypto payments (Bitcoin, Ethereum, Solana, USDC/USDT).
- 📱 **PicPay**: Consumer digital wallet and QR-code checkout in Brazil.
- 💸 **Wise**: High-speed, low-fee international B2B payouts and contractor disbursements across 40+ currencies.

> 📚 **Looking for a deep dive into provider selection and fees?** Check out the comprehensive **[Payment Gateways Guide](payment-gateways-guide.md)**.

---

## 🚀 Core Features

- **Multi-Provider Support:** First-class, zero-panic support for 10 top global, regional, and Web3 payment providers. Switch providers effortlessly by changing configuration or initializing the corresponding `BillingProvider`.
- **Revenue Dashboard (`/studio/capital`):** Native MRR (Monthly Recurring Revenue), ARR (Annual Recurring Revenue), Net Revenue, active subscriber statistics, and churn rate calculations built right into Rullst Studio.
- **Live Webhook Audit Inspector:** Real-time log inspector recording every received payment event payload, signature verification status, and timestamp.
- **Webhook Handling & Database Synchronization:** Secure, constant-time HMAC-verified webhook handlers that listen to subscription creations, renewals, upgrades, and cancellations, automatically synchronizing user state and access levels via `rullst-orm`.

---

## ⚙️ Configuration

In your `.env` file:

```env
# Choose provider: stripe | lemonsqueezy | infinitepay | polar | paddle | razorpay | mercadopago | coinbase | picpay
BILLING_PROVIDER=infinitepay

# Provider credentials
BILLING_API_KEY=your_api_key_or_token
BILLING_WEBHOOK_SECRET=your_webhook_signing_secret
```

---

## 💻 Code Example: Initializing in Rust

```rust
use rullst_capital::{init_provider, InfinitePayProvider, StripeProvider, PolarProvider, RazorpayProvider};

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: InfinitePay (0% Pix Fee in Brazil)
    init_provider(Box::new(InfinitePayProvider::new(
        std::env::var("BILLING_API_KEY")?,
        std::env::var("BILLING_WEBHOOK_SECRET")?,
    )));

    println!("💳 Rullst Capital initialized with InfinitePay");
    Ok(())
}
```
