### Removed

- **Breaking:** the Mail APIs deprecated in 12.0.0 were removed:
  `ResendDriver::new` and `AwsSesDriver::new` (use `try_new`, which returns a
  `MailError` for invalid configuration instead of failing later in `send`),
  `Message::with_open_tracking` and `Message::with_click_tracking` (use
  `try_with_open_tracking` and `try_with_click_tracking`), and
  `TrackingEngine::generate_open_token`, `generate_click_token`,
  `inject_open_pixel` and `rewrite_links` (use their `try_` forms, which
  return a `TrackingError` instead of an empty token or the unchanged HTML).
  `cargo rullst upgrade` reports each use (`V13-MAIL-DEPRECATED-REMOVED`).
