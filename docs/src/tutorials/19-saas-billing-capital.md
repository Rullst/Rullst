# Tutorial 19: SaaS Billing with Rullst Capital 💳

Orchestrate Stripe / LemonSqueezy subscriptions and inspect live webhook events using `rullst-capital`.

---

## 🛠️ Step 1: Scaffold Billing Controllers

```bash
cargo rullst make:billing
```

Configure provider in `.env`:
```dotenv
BILLING_PROVIDER=stripe
BILLING_API_KEY=sk_test_...
BILLING_WEBHOOK_SECRET=whsec_...
```

---

## 💻 Step 2: Handle Checkout & Webhooks

```rust
use rullst_capital::dashboard::RevenueDashboardManager;

pub async fn checkout_redirect() -> Response {
    // Redirect user to secure Stripe checkout session
    rullst_capital::redirect_to_checkout("price_pro_monthly").await
}
```

View real-time MRR/ARR charts and incoming webhook logs in Rullst Studio: `http://localhost:5555/studio/capital`.

---

## 💡 Key Takeaways
- Zero sensitive credit card data touches your servers.
- `rullst-capital` auto-syncs local database subscription records on payment webhooks.
