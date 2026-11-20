### Fixed

- `cargo rullst upgrade` did not report browser and smoke tests that compare
  the `rullst_csrf` cookie with the form's `_token`, which fail since
  13.0.0-alpha.6 masks the token per response; `cargo test` does not run such
  scripts, so the failure first appeared in CI. `V13-CSRF-TOKEN-COMPARED` now
  also reads the application's JavaScript, TypeScript and Python files that
  mention CSRF (outside `target`, `node_modules` and similar folders) and
  reports a line that expects the cookie and the token to be equal: an
  `==`/`===`, a `!=`/`!==` in the condition of an `if` that fails the test, an
  equality assertion such as `expect(token).toBe(cookie.value)` or
  `assertEqual`, or a check that the page contains the cookie's value. The
  fix is to assert that the token is present and differs from the raw cookie.
  Sending the cookie's value as `_token` or `X-CSRF-Token`, which the
  middleware still accepts, is not reported.
