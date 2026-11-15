### Changed

- **Breaking:** `Server` applies the CSRF check and the secure headers in every
  environment, development and test included, and the hot-reload server of
  `cargo rullst dev` wraps each router generation in the same baseline. Until
  13.0.0-alpha.5 only staging and production had them, so development
  accepted a `POST`, `PUT`, `PATCH` or `DELETE` without the double-submit
  token in an application whose router did not add `csrf_middleware` itself
  (the API starter, a custom router); it now refuses it with `403`, as
  production does. Outside staging and production `Strict-Transport-Security`
  is sent only over HTTPS, the CSRF cookie carries `Secure` over HTTPS as
  before, and the WAF still only reports. `apply_security_baseline` behaves
  the same way for `Environment::Development` and `Environment::Test`.
- Projects created by `cargo rullst new` no longer add `csrf_middleware` and
  `headers_middleware` to their router, which `Server` now applies in every
  environment; a request paid for both twice. `cargo rullst upgrade` reports
  these layers in an existing router (`V13-ROUTER-BASELINE-LAYERS`), and
  `make:privacy` and `make:age-gate` work with a router without a CSRF layer.
  The generated contract tests that call `router()` directly wrap it in
  `apply_security_baseline`.
