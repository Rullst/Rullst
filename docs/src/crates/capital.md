# Rullst Capital 💳

`rullst-capital` is the native monetization and billing engine for the Rullst Framework. It abstracts away the immense complexity of handling SaaS subscriptions, webhooks, and invoice generation so you can focus on building your product.

## ✨ Features

- **Zero-Panic Guarantees:** 100% safe Rust. Fails gracefully on API rate limits or malformed webhook payloads.
- **Provider Agnostic (Almost):** First-class, deeply integrated support for **Stripe**, with abstractions in place for LemonSqueezy and Paddle.
- **Webhook Handling:** Built-in cryptographic signature verification for Stripe Webhooks to prevent replay and spoofing attacks.
- **Subscription Management:** Easily upgrade, downgrade, prorate, or cancel subscriptions directly from Rust structs.
- **Invoice & Tax Handling:** Automatically generates downloadable PDF receipts and synchronizes tax records.

## 🚀 Quickstart

Add `rullst-capital` to your project:

```bash
cargo add rullst-capital
```

### Handling Stripe Webhooks

Rullst Capital provides pre-built route handlers that automatically verify signatures and map JSON payloads into strongly-typed Rust enums.

```rust
use rullst::{Router, routing::post};
use rullst_capital::stripe::{WebhookHandler, WebhookConfig};
use rullst_capital::events::SubscriptionEvent;

async fn handle_subscription_updates(event: SubscriptionEvent) {
    match event {
        SubscriptionEvent::Created(sub) => println!("New subscriber: {}", sub.customer_id),
        SubscriptionEvent::Canceled(sub) => println!("Lost subscriber: {}", sub.customer_id),
        _ => {}
    }
}

#[tokio::main]
async fn main() {
    let config = WebhookConfig::new("whsec_your_stripe_secret");
    
    // The WebhookHandler automatically verifies signatures
    // and parses the payload into a typed event.
    let handler = WebhookHandler::new(config)
        .on_subscription(handle_subscription_updates);

    let app = Router::new()
        .route("/webhooks/stripe", post(handler.into_service()));
        
    // ... start server ...
}
```

## 🔐 Security Audit

`rullst-capital` treats webhook endpoints as hostile territory. It utilizes constant-time cryptographic comparisons (`hmac`) to verify Stripe signatures, preventing timing attacks. Missing or malformed `Stripe-Signature` headers result in an immediate `400 Bad Request` without allocating memory for the payload body.

## 📚 Documentation

For advanced usage, including metered billing, syncing products from Stripe, and customer portal session generation, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
