### Added

- Capital: a confirmed `OneOffPayment` now says how the buyer paid, when the
  provider reports it: `payment_method()` returns a `#[non_exhaustive]`
  `PaymentMethod` (`Pix`, `CreditCard`, or `Other` with the provider's own
  name, 1 to 64 ASCII letters, digits, `_` or `-`), and `installments()` the
  number of instalments. InfinitePay maps `capture_method` and `installments`
  from `POST /payment_check`, leaving out a value it cannot read; Stripe's
  session reports neither, so both are `None`. Neither changes `is_paid()`.
  Custom adapters add them with `with_payment_method` and
  `with_installments`.
- Capital: `OneOffCheckoutRequest::with_notification_url(url)` sends one
  checkout's payment notification to its own URL instead of the provider's
  default, for example a URL with a per-checkout token, since InfinitePay's
  webhook is unsigned. The URL must be HTTPS, without credentials or a
  fragment, and at most 2048 bytes. InfinitePay sends it as the link's
  `webhook_url`; Stripe, whose webhooks go to the endpoints configured on the
  account, refuses the request with `UnsupportedOperation`, also with a
  `mock_*` key. A request without a URL keeps its 13.0.0-alpha.5
  `request_digest()`. An adapter that sends the URL marks the checkout with
  `OneOffCheckout::with_notification_url_sent`, and the billing conformance
  kit's new `checkout.notification_url` case fails an adapter that neither
  sends nor refuses it. Reported by the vene.win application.
