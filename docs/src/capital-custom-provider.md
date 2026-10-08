# Writing your own payment provider

Rullst Capital ships two built-in adapters: **Stripe** (supported) and
**InfinitePay** (experimental until it is validated against a live account).
Every other gateway is integrated by the application through the same
provider-neutral contracts the built-in adapters use. Version 13 removed the
Paddle, Lemon Squeezy, Polar, Razorpay, Mercado Pago, Alipay, Coinbase
Commerce, PicPay and Wise adapters; see the
[migration guide](migration-v13.md#changes-from-the-published-1210-source)
row "Capital providers and NFS-e removed".

This guide walks through a complete adapter for a fictitious gateway,
**AcmePay**. The code below is included from
[`rullst-capital/examples/custom_provider.rs`](https://github.com/Rullst/Rullst/blob/main/rullst-capital/examples/custom_provider.rs),
which CI compiles and which runs offline:

```text
cargo run -p rullst-capital --example custom_provider
```

In an application, the same types are available through the facade as
`rullst::capital::*` with the `capital` feature. The example's HMAC check uses
`ring`, `hex` and `subtle`; add equivalent crates to your own `Cargo.toml`.

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

## 4. Test the adapter

The example's `main` exercises the contract offline:

- a signed mock delivery returns 200 and its identical replay returns 409;
- a wrong signature returns 401 and never reaches the handler;
- the live HMAC path accepts a fresh signature and rejects a stale timestamp;
- unsupported operations and direct charges fail with `UnsupportedOperation`.

Add the gateway's published webhook examples as fixtures, and validate the
live adapter against a sandbox account before enabling it in production.
Tests with mocks establish local contracts, not provider interoperability.
