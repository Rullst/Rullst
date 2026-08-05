# Rullst Capital: SaaS Billing Made Easy

**Rullst Capital** is the billing, subscription orchestration, and revenue analytics layer for Rullst SaaS applications.

If you generated your project using the `SaaS` blueprint (`cargo rullst new` -> Select `SaaS Starter`) or ran `cargo rullst make:billing`, your application comes pre-wired with Capital, allowing you to charge users from day one.

## Core Features

- **Multi-Provider Support:** Supports `Stripe` and `LemonSqueezy`. Swap between them by changing `BILLING_PROVIDER` in `.env`.
- **Revenue Dashboard (`/studio/capital`):** Native MRR (Monthly Recurring Revenue), ARR (Annual Recurring Revenue), Net Revenue, active subscriber stats, and churn rate calculations built right into Rullst Studio.
- **Live Webhook Audit Inspector:** Real-time log inspector recording every received payment event payload, signature verification status, and timestamp.
- **Webhook Handling:** Secure webhook handlers listen to subscription creations, renewals, upgrades, and cancellations.
- **Database Synchronization:** Automatically updates the `subscriptions` table via `rullst-orm`, keeping user access in sync with payment status.

## Configuration

In your `.env` file:

```env
BILLING_PROVIDER=stripe # or lemonsqueezy
BILLING_API_KEY=sk_test_...
BILLING_WEBHOOK_SECRET=whsec_...
```

## How It Works in Your App

The generated `billing_controller.rs` provides primary endpoints:

1. **Checkout Redirect:** Hits `/billing/checkout?plan=price_pro`, creating a secure checkout session with Stripe/LemonSqueezy.
2. **Webhook Listener:** The `/billing/webhook` route verifies HMAC signatures and updates `plan_id` and `ends_at` timestamps in your database.
3. **Studio Dashboard:** Access `http://localhost:5555/studio/capital` to view live MRR/ARR charts and inspect incoming webhook payloads.
