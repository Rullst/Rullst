# Rullst Capital: SaaS Billing Made Easy

**Rullst Capital** is the billing and subscription orchestration layer for Rullst SaaS applications.

If you generated your project using the `SaaS` blueprint (`cargo rullst new` -> Select `SaaS Starter`), your application already comes wired with Capital, allowing you to charge your users from day one.

## Core Features

- **Multi-Provider Support:** Rullst Capital currently supports `Stripe` and `LemonSqueezy`. You can swap between them by simply changing the `BILLING_PROVIDER` environment variable. No code changes required!
- **Webhook Handling:** Secure webhook endpoints are automatically set up to listen to subscription updates, payment successes, and cancellations.
- **Database Synchronization:** When a payment succeeds, Capital automatically updates the `subscriptions` table in your local database via the `rullst-orm`, keeping user access in sync with their payment status.

## Configuration

In your `.env` file, configure your keys:

```env
BILLING_PROVIDER=stripe # or lemonsqueezy
BILLING_API_KEY=sk_test_...
BILLING_WEBHOOK_SECRET=whsec_...
```

## How It Works in Your App

The generated `billing_controller.rs` provides two primary endpoints:

1. **Checkout Redirect:** When a user clicks "Upgrade to Pro", they hit `/billing/checkout?plan=price_pro`. Rullst Capital instantly communicates with Stripe/LemonSqueezy to create a secure checkout session and redirects the user.
2. **Webhook Listener:** The `/billing/webhook` route securely verifies the signature of the incoming request and delegates it to the `webhook_handler`. The handler then updates the user's `plan_id` and `ends_at` timestamp in your local database.

This architecture ensures your application stays extremely fast and never stores sensitive payment data on your servers.
