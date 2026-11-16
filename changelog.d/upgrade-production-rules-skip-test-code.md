### Fixed

- `cargo rullst upgrade` no longer reports the review rules about how the
  deployed application behaves in test code: an inline `#[cfg(test)]`
  module, a `#[test]` or `#[tokio::test]` function or another
  `#[cfg(test)]` item of a production file, and files below `tests/` or
  named `tests.rs`, `*_tests.rs` or `*_test.rs`. The rules are
  `V13-CSRF-COOKIE-IN-PAGE`, `V13-CSRF-STATIC-PREFIXES`,
  `V13-ROUTER-BASELINE-LAYERS`, `V13-CREDENTIAL-RATE-LIMIT`,
  `V13-LMS-RECORD-PROGRESS`, `V13-ERP-DASHBOARD-ACCESS`,
  `V13-ERP-STORE-ORDER`, `V13-PAGE-CSP-NONCE`, `V13-PAGE-BOUNDS` and
  `V13-PORTFOLIO-PROFILE`; before, a controller's test module that read the
  CSRF cookie, for example, was reported as a page embedding it.
  Must-change rules and the review rules whose change a test also observes
  still report test code.
