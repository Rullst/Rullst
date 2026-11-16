### Added

- `examples/mail-postmark-driver` shows how an application writes its own
  mail transport for a provider Rullst does not ship, with Postmark: a
  `MailDriver` that maps Reply-To, attachments, inline images and
  `List-Unsubscribe` to Postmark's fields, captures mail offline for an
  empty or `mock_*` token, and keeps only the HTTP status of a failure. Its
  `PostmarkWebhook` checks the webhook's Basic credentials in constant time
  and records Postmark's suppressions (hard bounces, spam complaints,
  unsubscriptions) for `SuppressionGuard`. The offline tests run the
  `rullst-mail` conformance kit. The mail guides link it and say that
  SendGrid, Mailjet and the other removed providers follow the same shape.
  ([#638](https://github.com/Rullst/Rullst/issues/638))
