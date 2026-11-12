# Writing your own payment provider

Rullst Capital ships two built-in adapters: **Stripe** (supported) and
**InfinitePay** (supported since 13.0.0-alpha.5, for one-off payments; see
[gateways with unsigned webhooks](#gateways-with-unsigned-webhooks)).
Every other gateway is integrated by the application through the same
provider-neutral contracts the built-in adapters use. Version 13 removed the
Paddle, Lemon Squeezy, Polar, Razorpay, Mercado Pago, Alipay, Coinbase
Commerce, PicPay and Wise adapters; see the
[migration guide](migration-v13.md#changes-from-the-published-1210-source)
row "Capital providers and NFS-e removed".

This guide walks through a complete adapter for a fictitious gateway,
**AcmePay**. The code below is included from
[`rullst-capital/examples/custom_provider.rs`](https://github.com/Rullst/Rullst/blob/main/rullst-capital/examples/custom_provider.rs),
which CI compiles and which runs offline from a clone of the Rullst repository:

```text
cargo run -p rullst-capital --example custom_provider
```

In an application, the same types are available through the facade as
`rullst::capital::*` with the `capital` feature. `BillingProvider` is an
`#[async_trait]` trait: use the `async-trait` crate, as the example does, or
the facade's `rullst::async_trait`. The example's HMAC check uses `ring`, `hex`
and `subtle`; add equivalent crates to your own `Cargo.toml`.

## What the framework does and what the adapter does

| Responsibility | Owner |
| :--- | :--- |
| Bounding the webhook body (2 MiB), rejecting duplicate Standard Webhooks headers, refusing `mock_*` secrets in production state, replay protection, restoring the exact body and inserting the `WebhookEvent` | `verify_webhook_with_state` (Axum) or `verify_webhook_actix_with_state` (Actix) |
| Declaring real or mock verification, authenticating the exact signed bytes, checking freshness, mapping only documented states | Your `BillingProvider` implementation |
| Owner binding, plan allowlists, persistence, reconciliation and entitlements | Your application, with the Capital checkout, entitlement and quota contracts |

The adapter is trusted server code. A common trait does not mean every gateway
supports every operation: an operation the gateway does not offer, or that you
have not reviewed, must return `CapitalError::UnsupportedOperation` instead of
fabricating success.

## 1. Implement `BillingProvider`

```rust,ignore
{{#include ../../rullst-capital/examples/custom_provider.rs:provider}}
```

Points that matter for security and testing:

- **Declare the verification mode.** The default
  `webhook_verification_mode` returns a configuration error, so an adapter that
  forgets it can never be mounted behind the middleware. An empty secret is a
  configuration error; a `mock_*` secret selects `WebhookVerificationMode::Mock`.
- **Authenticate before parsing.** `handle_webhook` verifies the exact body
  bytes with the gateway's documented algorithm and a constant-time comparison
  (`ring::hmac::verify` here), and checks the timestamp window when the protocol
  signs one. Mock fixtures still require the configured `mock_*` secret.
- **Map only documented states.** An unknown status or a missing identity is a
  `PayloadParseError`, never `Active`.
- **Offline fallback.** Empty or `mock_*` API keys return deterministic URLs on
  reserved `.invalid` hosts, so tests and local sandboxes never contact the
  gateway. Live paths must bind the provider response to the request before
  returning it.
- **Optional contracts.** Override `BillingProvider::charge` only after
  forwarding the request's idempotency key and binding amount and currency in
  the response. Metered billing uses the separate `MeteredBillingProvider`
  trait; return `UsageDeduplication::ApplicationOutboxRequired` from
  `UsageReceipt::from_verified_provider_response` when the gateway offers no
  deduplication key.

## 2. Plug it into checkout and subscriptions

```rust,ignore
{{#include ../../rullst-capital/examples/custom_provider.rs:billable}}
```

```rust,ignore
{{#include ../../rullst-capital/examples/custom_provider.rs:checkout}}
```

`subscription_with` and `charge_with` dispatch statically to the provider you
pass. The compatibility helpers (`Billable::subscribe`,
`Billable::billing_portal_url`, `Billable::cancel_subscription`) use the global
provider; select it once at start-up with
`rullst::capital::try_init_provider(Box::new(provider))`, which reports a
second initialization instead of replacing the first provider.

Derive the subscription ID and the plan from authenticated application state,
and keep a server-owned plan allowlist. A checkout redirect is not proof of
payment; grant access from verified, reconciled state, for example with the
`entitlements::EntitlementGate`.

## 3. Verify webhooks with the canonical middleware

```rust,ignore
{{#include ../../rullst-capital/examples/custom_provider.rs:webhook}}
```

Mount `WebhookMiddlewareState::production_with_provider` in production. It
rejects `mock_*` secrets with `MockWebhookNotAllowed`; `local_mock_with_provider`
is for local fixtures only. The middleware maps failures to HTTP status codes:
401 for a bad signature or stale timestamp, 409 for a replay, 413 for an
oversized body and 503 when the replay store is full or unavailable.

The in-memory replay store is process-local. With several processes, use the
`webhook-sql` feature: `SqlWebhookReplayStore` shares replay claims, and its
`check_and_record_event_key_with_transaction` claims the gateway's stable event
ID inside your own database transaction. Body-hash replay protection
does not stop a gateway from resending the same event with a different body;
prefer the stable event ID when the gateway provides one.

### Gateways with unsigned webhooks

Some gateways notify the application without signing the request.
InfinitePay's public [Checkout API](https://www.infinitepay.io/checkout-documentacao)
is one: a payment link is created with
`POST https://api.checkout.infinitepay.io/links`, the webhook carries no
signature, and a payment is confirmed with `POST /payment_check`. The built-in
`InfinitePayProvider::checkout(handle)` implements that API (see the
[crate README](https://github.com/Rullst/Rullst/blob/main/rullst-capital/README.md#infinitepay));
an adapter for another such gateway follows the same contract:

- declare `WebhookVerificationMode::ConfirmedAtSource`. The canonical
  middleware then never calls `handle_webhook` and never inserts a
  `WebhookEvent`, whose status would be the sender's word;
- implement `parse_payment_notification` to read only *which* payment the body
  names into a `PaymentNotification` (`reference`, `transaction_id`,
  `invoice_id`); the middleware inserts it into the request extensions;
- implement `create_one_off_checkout` from the server-side
  `OneOffCheckoutRequest`, never from amounts the browser sent, and return
  `OneOffCheckout::from_verified_provider_response`;
- implement `confirm_one_off_payment` with the gateway's own lookup and return
  `OneOffPayment::paid(provider, request, charged, paid, currency)`, which fails
  unless the charged amount equals the request's total and the currency
  matches, or `OneOffPayment::unpaid`.

The handler loads the request it persisted by `notification.reference()`,
calls `confirm_one_off_payment` and marks the order paid, idempotently, only
when `is_paid()` is true. These notifications skip the replay store: a repeated
one only repeats the lookup. Send the gateway requests with
`providers::ProviderRequest`, which applies the built-in adapters' HTTP rules
(no redirects, no ambient proxy, finite timeouts, 1 MiB responses, HTTPS,
redacted errors) without exposing an HTTP client type:

```rust,ignore
use rullst_capital::providers::ProviderRequest;

let check: serde_json::Value = ProviderRequest::post_json(
    "acmepay",
    "payment check",
    "https://api.acmepay.example/v1/check",
    &serde_json::json!({ "order": notification.reference() }),
)?
.header("X-Api-Key", api_key)
.send_json()
.await?;
```

## 4. Test the adapter

The example's `main` exercises the contract offline:

- a signed mock delivery returns 200 and its identical replay returns 409;
- a wrong signature returns 401 and never reaches the handler;
- the live HMAC path accepts a fresh signature and rejects a stale timestamp;
- unsupported operations and direct charges fail with `UnsupportedOperation`.

Add the gateway's published webhook examples as fixtures, and validate the
live adapter against a sandbox account before enabling it in production.
Tests with mocks establish local contracts, not provider interoperability.
