# Writing your own mail transport

Rullst Mail ships four delivery transports: **Resend**, **AWS SES**,
**SendPulse** and **SMTP**, plus the `log`, `memory` and offline `mock_*`
drivers for development and tests. Every other provider is integrated by the
application through the same `MailDriver` trait the built-in transports
implement. Version 13 removed the SendGrid, Postmark, Mailjet, Mailtrap and
Azure Communication Services transports; see the
[migration guide](migration-v13.md#changes-from-the-published-1210-source)
row "Mail providers removed".

This guide walks through a complete transport for a fictitious HTTP email API,
**AcmeMail**. The code below is included from
[`rullst-mail/examples/custom_transport.rs`](https://github.com/Rullst/Rullst/blob/main/rullst-mail/examples/custom_transport.rs),
which CI compiles and which runs offline:

```text
cargo run -p rullst-mail --example custom_transport
```

In an application, the same types are available through the facade as
`rullst::mail::*` with the `mail` feature. The example also uses `reqwest`,
`secrecy`, `serde_json`, `chrono` and `async-trait`; add equivalent crates to
your own `Cargo.toml`.

## What the framework does and what the transport does

| Responsibility | Owner |
| :--- | :--- |
| Header-injection, recipient, link and homograph checks, outbound secret redaction, and attachment count, size and CID bounds | `DeliveryPipeline::prepare`, which the transport calls first |
| Default sender, queueing and scheduling, tenant routing, failover, suppression, attachment inspection and observations | The `Mail` facade, `TenantMailResolver`, `FailoverDriver`, `SuppressionGuard`, `AttachmentInspectionGuard` and `ObservedMailDriver`, which wrap any `MailDriver` |
| Authenticating to the provider, mapping the message to its API, rejecting what the API cannot express, bounding and checking the receipt, and classifying failures | Your `MailDriver` implementation |
| Sender-domain verification, provider limits, bounce and complaint handling, retention and the provider contract | Your application and provider account |

The transport is trusted server code. A transport that cannot express part of
a message (a schedule, an inline image, an unsubscribe header) must return an
error instead of sending the message without it.

## 1. Implement `MailDriver`

```rust,ignore
{{#include ../../rullst-mail/examples/custom_transport.rs:transport}}
```

Points that matter for security and testing:

- **Run the pipeline first.** `DeliveryPipeline::prepare` applies the same
  pre-flight checks as the built-in transports; send `prepared.message()`,
  never the original.
- **Never invent a sender.** A message without `from` fails with
  `MailError::ConfigError` before any request. The `Mail` facade fills it from
  `MAIL_FROM` or `[mail] from` before the transport sees it.
- **Reject what the API cannot express.** A future `send_at` without provider
  scheduling, inline CID images or headers the API has no field for are errors,
  not silent drops. Durable scheduling belongs to `Mail::enqueue`.
- **Offline fallback.** An empty or `mock_*` key selects
  `DeliveryMode::OfflineMock` through `credential_mode`, and the transport
  captures the message without network I/O, as AGENTS.md 3.5 requires. Validate
  the payload before that check so the fixture enforces the live contract.
- **Bound the HTTP exchange.** Disable redirects (they would forward the key
  and the message), set connect and request deadlines, require HTTPS except for
  a loopback test fixture and read at most a bounded receipt.
- **Classify failures.** `MailError::from_provider_response` turns HTTP 429
  into `RateLimited` with its `Retry-After` and 5xx into a transient error;
  `MailError::transport` marks a failure before any response. `FailoverDriver`
  moves only transient and rate-limited failures to a fallback. Keep provider
  error bodies out of the error: they can echo addresses and content.
- **Accept only a real receipt.** Success requires the provider's message ID;
  a 2xx without one is a `SendError`. Acceptance is not proof of inbox
  delivery.
- **Idempotency.** `send_with_delivery_id` defaults to at-least-once. Override
  it only when the provider deduplicates by a key you forward.

## 2. Install it

```rust,ignore
{{#include ../../rullst-mail/examples/custom_transport.rs:install}}
```

`Mail::set_driver` replaces the facade's configured driver for every send and
for the queue worker (`register_mail_handler`) in that process, so install it
at start-up in every process that sends or works the mail queue.
`Mail::reset_driver` returns to `MAIL_DRIVER`. A `TenantMailResolver` selects a
transport per tenant, with a default for the others. Suppression, attachment
inspection and observation wrap the transport with static dispatch, as they
wrap the built-in ones.

A `MAIL_DRIVER` value cannot name a custom transport: the facade builds only
the kept drivers from configuration, and a removed driver name fails with
`MailError::ConfigError` instead of falling back.

## 3. Test the transport

The example's `main` exercises the contract offline:

- a `mock_*` key captures the message and never reaches the network;
- a message without a sender, and one with an inline image, fail before any
  request;
- against a loopback HTTP fixture, a 202 receipt succeeds, a 429 becomes
  `RateLimited` with a seven-second `Retry-After`, and a 503 fails over to the
  fallback driver;
- the facade, a tenant resolver and a suppression guard use the transport.

Add the provider's documented request and response examples as fixtures, and
validate the live transport against a sandbox account and a verified sender
before enabling it in production. Tests with mocks establish local contracts,
not provider interoperability.
