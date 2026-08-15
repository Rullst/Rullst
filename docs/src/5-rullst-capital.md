# Rullst Capital: SaaS Billing & Financial Engine 💳

**Rullst Capital** (`rullst-capital`) is the native monetization, subscription orchestration, revenue analytics, and payment infrastructure layer for the Rullst Framework.

If you generated your project using the `SaaS` blueprint (`cargo rullst new my-saas --blueprint saas`) or ran `cargo rullst make:billing`, your application comes pre-wired with Capital, allowing you to charge users globally, accept Pix with zero fees in Brazil, support developer-first open-source backer tiers, or accept Web3 crypto payments from day one.

---

## 🌟 Supported Payment Gateways

Rullst Capital supports 11 major financial providers out of the box:

- 🌐 **Stripe**: Global direct merchant for cards, Apple Pay, Google Pay, and customer portals.
- 🍋 **Lemon Squeezy**: Global Merchant of Record (MoR) with automated EU VAT and US state sales tax compliance.
- 🇧🇷 **InfinitePay**: Brazil domestic gateway with **Pix at 0.00% fee**, instant D+0 settlement, and lowest credit card rates.
- ⚡ **Polar.sh**: Developer-first Merchant of Record for open-source funding, software licenses, and micro-SaaS.
- 🛡️ **Paddle**: Enterprise Merchant of Record for global B2B SaaS.
- 🇨🇳 **Alipay (支付宝 / Alipay+)**: China and APAC cross-border payments with over 1.3 billion users and Alipay+ wallet integrations.
- 🇮🇳 **Razorpay**: Dominant payment gateway across India and Southeast Asia for UPI, cards, and subscriptions.
- 🌎 **Mercado Pago**: Broadest Latin American regional coverage (Argentina, Mexico, Chile, Colombia, Brazil).
- ₿ **Coinbase Commerce**: Borderless Web3 crypto payments (Bitcoin, Ethereum, Solana, USDC/USDT).
- 📱 **PicPay**: Consumer digital wallet and QR-code checkout in Brazil.
- 💸 **Wise**: High-speed, low-fee international B2B payouts and contractor disbursements across 40+ currencies.

> 📚 **Looking for a deep dive into provider selection and fees?** Check out the comprehensive **[Payment Gateways Guide](payment-gateways-guide.md)**.

---

## 🚀 Core Features

- **Multi-Provider Support:** First-class, zero-panic support for 11 top global, regional, and Web3 payment providers. Switch providers effortlessly by changing configuration or initializing the corresponding `BillingProvider`.
- **Revenue Dashboard (`/studio/capital`):** Native MRR (Monthly Recurring Revenue), ARR (Annual Recurring Revenue), Net Revenue, active subscriber statistics, and churn rate calculations built right into Rullst Studio.
- **Live Webhook Audit Inspector:** Real-time log inspector recording every received payment event payload, signature verification status, and timestamp.
- **Webhook Handling & Database Synchronization:** Secure, constant-time HMAC-verified webhook handlers that listen to subscription creations, renewals, upgrades, and cancellations, automatically synchronizing user state and access levels via `rullst-orm`.

---

## ⚙️ Configuration

In your `.env` file:

```env
# Choose provider: stripe | lemonsqueezy | infinitepay | polar | paddle | alipay | razorpay | mercadopago | coinbase | picpay
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

---

## 🧾 Zero-Cost Native Digital Invoices (NFS-e Padrão Nacional / SEFAZ)

Rullst Capital includes a **native, direct digital invoice engine** for Brazilian SaaS with **R$ 0.00 intermediary fees per invoice**. It signs XML documents in memory using your company's A1 Digital Certificate (`.pfx`) and transmits them directly to the Receita Federal national portal:

```rust
use rullst_capital::fiscal::{
    issue_nfse_direct, FiscalCertificate, FiscalCustomer, FiscalEmitter,
    NfseEnvironment, TaxRegime,
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

// 4. Load A1 certificate (.pfx in base64 from vault)
let cert = FiscalCertificate::from_base64(
    &std::env::var("CERTIFICADO_A1_BASE64")?,
    &std::env::var("CERTIFICADO_A1_SENHA")?,
);

// 5. Emit directly to Receita Federal with R$ 0.00 fee!
let response = issue_nfse_direct(&emitter, &customer, &dps, &cert, NfseEnvironment::Production).await?;
println!("✅ NFS-e emitida com sucesso! Chave de Acesso: {}", response.access_key);
```

