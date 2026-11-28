### Added

- `WebhookMiddlewareState::with_retry_after_server_error()`: with it,
  `verify_webhook_with_state` and `verify_webhook_actix_with_state` remove a
  delivery's replay claim when the handler answers with a server error (5xx),
  so the provider's retry of the same payload reaches the handler again
  instead of getting `409 Conflict` for the replay TTL (24 hours by default)
  and the event being lost. It works with the in-memory and the SQL replay
  store; the SQL store removes the claim in a task of its own. It is off by
  default: while the handler keeps failing, anyone holding a captured signed
  body can deliver it again, and each delivery runs the handler, so enable it
  only when a 5xx answer means nothing was applied. The claim is still kept
  after a `2xx` or `4xx` answer, when its removal fails, and when the request
  is dropped before the handler answers; `verify_webhook`,
  `verify_webhook_mock_local` and their Actix equivalents never remove it
  ([#837](https://github.com/Rullst/Rullst/issues/837)).
