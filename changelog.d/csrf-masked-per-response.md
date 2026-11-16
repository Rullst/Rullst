### Security

- **Breaking:** the CSRF token that pages embed is masked per response, a
  defence against BREACH. `CsrfToken::as_str()` now returns
  `mask || (token XOR mask)` in URL-safe base64, with a fresh random mask
  for every request from a ChaCha12 generator per thread, seeded and
  reseeded from the operating system (86 characters for the 32-character
  token the middleware mints, which comes from the operating system's
  random source), instead of the `rullst_csrf`
  cookie value, which used to be the same in every response and could be
  recovered from a compressed page that also reflected attacker-controlled
  input. The middleware unmasks the `_token` field or `X-CSRF-Token` header
  and compares it with the cookie in constant time; it still accepts the
  cookie value itself, for clients that read the cookie, and nothing else.
  The cookie is unchanged. Code or tests that compared the embedded value
  with the cookie must submit it instead. The new
  `rullst::security::mask_csrf_token` masks a value read from the cookie;
  the `get_csrf_token` helpers that `make:auth`, the SaaS starter and
  `make:oauth` generate use it, and `cargo rullst upgrade` reports the older
  helper that embeds the raw cookie value (`V13-CSRF-COOKIE-IN-PAGE`). A
  multipart `_token` field may be up to 342 bytes (was 128). Masking covers
  the CSRF token only: an application should not show other secrets in a
  compressed page next to text from the request.
