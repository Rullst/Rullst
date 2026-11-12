# Provider conformance kits

An application that adds its own payment gateway, mail transport or video
service implements the same trait as Rullst's built-in adapters. Since
13.0.0-alpha.5 each of those crates ships a **conformance kit**: a test suite,
behind the Cargo feature `conformance`, that runs the contract the built-in
adapters follow against your implementation and returns a structured report.
Passing it shows, in your own test suite, that the adapter refuses forged
notifications, header injection or impossible grants the way the built-in ones
do. No network is needed: the kits run against the adapter's offline mode
(empty or `mock_*` credentials) or a local fixture you control.

| Crate | Trait | Kit | Built-in adapters it passes |
| --- | --- | --- | --- |
| `rullst-capital` | `BillingProvider` | `conformance::check_billing_provider` | Stripe (mock and signed-webhook modes), InfinitePay (offline fixtures and the local Checkout API fixture) |
| `rullst-mail` | `MailDriver` | `conformance::check_mail_driver` | Resend, AWS SES, SendPulse, SMTP (offline mock) |
| `rullst-media` | `VideoProvider` | `conformance::check_video_provider` | Bunny Stream (offline mode, complete and upload-only) |

Enable the feature in your development dependencies only, with the same exact
version as the rest of your Rullst crates:

```toml
[dev-dependencies]
rullst-capital = { version = "=13.0.0-alpha.5", features = ["conformance"] }
```

## Reading a report

Every kit returns a `ConformanceReport` with one `ConformanceCase` per case, in
a fixed order. A case is `Passed`, `Failed` or `Skipped` (the harness did not
supply a fixture the case needs, or the capability does not apply), and its
`detail()` names every failed expectation, the reason for a skip, or what a
passed case could not exercise. `passed()` is true when no case failed;
`assert_passed()` fails a test with the whole report:

```text
conformance of examplepay: 8 passed, 1 failed, 0 skipped
  ok mode.declared
  ok checkout.inputs_validated
  ok checkout.server_side_amount
  ok offline.deterministic
  ok notification.forged_refused
  ok notification.replay_idempotent
  ok confirmation.required
  FAILED confirmation.amount_currency_bound: a source payment of a higher amount is never paid
  ok errors.typed_no_panic
```

Each case runs under `catch_unwind`, so a panicking adapter fails that case
with `panicked: <message>` and the run continues. The kits themselves never
panic, except `assert_passed`. Case identifiers are stable within 13.x; a
later 13.x release may add cases, so read the report rather than counting
cases.

## Billing providers

`check_billing_provider(&provider, &harness)` takes your `BillingProvider` and
a `BillingHarness`, which supplies what only the implementer knows:

- `one_off_request()`: a server-side `OneOffCheckoutRequest` the provider
  accepts (its currency and minimum amount), or `None` when it has no one-off
  checkout; the kit then expects `UnsupportedOperation`.
- `authentic_delivery(request)`: an authentic webhook delivery, signed for the
  `Real` and `Mock` verification modes, or the unsigned body naming the request
  for `ConfirmedAtSource`. The kit derives forgeries from it: the body without
  its headers, every header value zeroed and, in `Real` mode, the authentic
  headers on a tampered body. A `mock_*` signature is not bound to the body,
  so the tamper check does not apply in `Mock` mode.
- `set_source_payment(request, payment)`: makes the provider's source of truth
  (its API, or the local fake standing in for it) report a `SourcePayment`.
  When the harness cannot simulate it, the kit notes the paths it could not
  exercise; offline fixtures that never report paid still pass.

| Case | What a conforming provider does |
| --- | --- |
| `mode.declared` | `webhook_verification_mode()` returns a mode |
| `checkout.inputs_validated` | plan checkout refuses blank e-mail addresses and plans |
| `checkout.server_side_amount` | the one-off checkout is bound to the server-side request (`request_digest`) and names the provider |
| `offline.deterministic` | the same checkout input gives the same result |
| `notification.forged_refused` | `handle_webhook` refuses invented, stripped, zeroed and tampered deliveries, and every body of a provider that confirms at the source |
| `notification.replay_idempotent` | reading the same authentic delivery twice gives the same event or notification |
| `confirmation.required` | nothing is paid until the source confirms it; a confirmed payment with the request's amount and currency is paid, twice the same, and bound to the request |
| `confirmation.amount_currency_bound` | a source payment of a lower or higher amount, another currency or another order reference is never paid |
| `errors.typed_no_panic` | empty, non-UTF-8 and 3 MiB bodies and empty identifiers return a `CapitalError` |

Rejecting a replayed delivery is the webhook middleware's job
(`verify_webhook_with_state` and its replay store), not the provider's, so the
kit checks that a replay gives the same answer.

### Worked example: an application-owned provider

`ExamplePay` is a fictitious gateway: it signs each webhook with an HMAC of the
body and reports payments through its API. The adapter keeps the protocol
rules (signature, request binding, amount comparison) and takes the API as a
generic `ExamplePayApi`, so production code passes an HTTP client and the
test passes a local fake. The code is included from
[`rullst-capital/tests/conformance_example.rs`](https://github.com/Rullst/Rullst/blob/main/rullst-capital/tests/conformance_example.rs),
which CI runs:

```rust,ignore
{{#include ../../rullst-capital/tests/conformance_example.rs:provider}}
```

The harness shares the fake API's payment with the provider, so the kit can
make the "source" report a payment of any amount and check that only the
request's amount and currency count:

```rust,ignore
{{#include ../../rullst-capital/tests/conformance_example.rs:harness}}
```

Run it with `cargo test -p rullst-capital --features conformance --test
conformance_example`. Points the kit enforced while the adapter was written:

- `OneOffCheckout::from_verified_provider_response` binds the gateway's URL to
  the server-side request; building it from another request fails
  `checkout.server_side_amount`.
- `OneOffPayment::paid` refuses a charged amount or currency other than the
  request's, so the adapter passes the provider's figures to it instead of
  comparing them itself.
- `hmac::verify` checks the exact body bytes; trimming or re-serializing the
  body before verifying fails `notification.forged_refused`.
- Operations the gateway does not offer return `UnsupportedOperation`, which
  satisfies `errors.typed_no_panic`; fabricating success would not.

For a gateway whose webhooks are unsigned, declare
`WebhookVerificationMode::ConfirmedAtSource`, implement
`parse_payment_notification`, return the unsigned body from
`authentic_delivery`, and confirm with the gateway's API; the InfinitePay
fixture test in `rullst-capital/src/conformance/tests.rs` shows the harness.
[Writing your own payment provider](capital-custom-provider.md) covers the rest
of the adapter and the webhook middleware.

## Mail transports

`check_mail_driver(name, &driver, &capture)` sends messages through your
`MailDriver` configured for its offline mode. The `DeliveryCapture` returns
what that mode captured instead of sending; the kit tells its own messages
apart by subject, so a capture shared with other tests works. The built-in
drivers record into the shared `OfflineMailMock`, read with
`OfflineMockCapture::new("resend")` (or `aws_ses`, `sendpulse`, `smtp`).

| Case | What a conforming driver does |
| --- | --- |
| `delivery.accepted` | a valid message is delivered once through `send`, `send_with_delivery_id` and `send_for_tenant`, with its recipient |
| `headers.crlf_refused` | CR or LF in the subject, recipient, sender or Reply-To is a `ValidationError` and nothing is delivered |
| `attachments.limits_enforced` | 33 attachments, one over 20 MiB, two over 25 MiB together, a CR/LF or path filename are refused; one small attachment is delivered intact |
| `suppression.respected` | behind a `SuppressionGuard`, a suppressed recipient gets `SuppressedRecipient` on every path and nothing is delivered; another recipient is delivered |
| `offline.deterministic` | the same message sent twice is captured twice, identically |
| `errors.typed_no_panic` | empty or invalid recipients, a subject over 2 KiB, a malformed delivery ID or tenant ID are a `ValidationError` and nothing is delivered |

A transport passes the refusal cases by running `DeliveryPipeline::prepare` (or
`prepare_for_tenant`) before anything else, as
[Writing your own mail transport](mail-custom-transport.md) shows. For that
guide's `AcmeMailDriver`, whose offline fixture keeps messages in
`offline_outbox()`, the capture is three lines:

```rust,ignore
struct Outbox(OfflineOutbox);

impl DeliveryCapture for Outbox {
    fn captured(&self) -> Result<Vec<Message>, MailError> {
        Ok(self.0.lock().map_err(|_| MailError::DriverError("outbox".into()))?.clone())
    }
}

let driver = AcmeMailDriver::try_new("mock_acmemail")?;
let capture = Outbox(driver.offline_outbox());
check_mail_driver("acmemail", &driver, &capture).await.assert_passed();
```

## Video providers

`check_video_provider(name, &provider, expectations)` creates, renames and
deletes videos, so run it against the offline mode or a disposable library.
`VideoExpectations::new(now)` expects upload grants, playback grants and
listing, checked against the trusted UTC second `now`;
`without_upload()`, `without_playback()` and `without_listing()` declare a
capability absent, which the provider must then refuse with
`MediaError::Unsupported`. Bunny's upload-only credentials pass with
`without_playback()`.

| Case | What a conforming provider does |
| --- | --- |
| `binding.stable` | `binding()` returns the same binding every time |
| `create.marker_validated` | `create` and `find_created` refuse a malformed marker with `InvalidInput` |
| `lifecycle.create_get_update_delete` | a created video awaits its upload, is found by marker and ID, renamed, deleted, and deleting it again succeeds; updating it then is `NotFound` |
| `upload.grant` | the grant names the video, library and mode, expires after its TTL, is signed, uses HTTPS and refuses a zero or 136-year TTL and clocks at or before 1970; a declared upload binds its type and length or is `Unsupported` |
| `playback.grant` | each kind is granted (expiring after its TTL, over HTTPS, refusing a zero TTL) or `Unsupported`; at least one kind is granted, or none without playback |
| `list.bounded` | a page echoes its number and size, holds at most `per_page` videos and a total at least that many |
| `offline.deterministic` | in `ProviderMode::Offline`, the same grant and listing requests give the same answers (skipped in other modes) |
| `errors.typed_no_panic` | an unknown video reads as `None`, deletes successfully and updates as `NotFound`; impossible clocks are refused |

### Worked example: an application-owned video provider

An adapter creates its own grants with `UploadGrant::new` and
`PlaybackGrant::new` (since 13.0.0-alpha.5). Both take the adapter's
`ProviderBinding`, whose library and mode the grant records, and validate
their input: the endpoint or URL is an absolute `https://` URL of at most
2 KiB without credentials, fragment, spaces or control characters (plain
`http://` only in `ProviderMode::ProtocolFixture`), the upload signature is
1 to 4096 visible ASCII characters, and the expiry is a UTC second between
1970 and 9999; anything else is `MediaError::InvalidInput`.
`UploadGrant::with_declaration` adds the content type and length a declared
upload binds. Both keep the URL or signature out of `Debug`. For `list`,
`ListedVideo::new` (a 1 to 1024-byte title and a duration of at most a week,
with optional `with_created_at` and an HTTPS `with_thumbnail_url`) and
`VideoPage::new(&query, items, total)` build the page; the page echoes the
query's number and size and refuses more than `per_page` items or a total
lower than their number.

`ExampleVideo` is a fictitious service that signs its own upload and HLS
URLs and lists its library. The code is included from
[`rullst-media/tests/conformance_example.rs`](https://github.com/Rullst/Rullst/blob/main/rullst-media/tests/conformance_example.rs),
which CI runs:

```rust,ignore
{{#include ../../rullst-media/tests/conformance_example.rs:provider}}
```

ExampleVideo grants only HLS playback, so every other kind returns
`Unsupported`, which the `playback.grant` case accepts. The test expects every
capability:

```rust,ignore
{{#include ../../rullst-media/tests/conformance_example.rs:check}}
```

A wrapper that adds instrumentation or policy around `BunnyStream` or the S3
adapter passes the kit the same way. It must forward `list` and
`upload_declared` explicitly: their default implementations return
`Unsupported`, which `list.bounded` reports.
