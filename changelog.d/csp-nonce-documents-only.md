### Changed

- **Breaking:** the Core security baseline (`headers_middleware`, and
  `Server` in staging and production) puts the request's CSP nonce only into
  the policy of a response that can be an HTML document: `Content-Type`
  `text/html` or `application/xhtml+xml`, or none. Every other response
  (JSON, plain text, images, files) gets the policy without a nonce, computed
  once instead of rendered per response: the configured `csp` when it has no
  `{NONCE}`, otherwise the default directives with `script-src 'self'` and
  `style-src 'self'`. That policy is stricter, since no inline script or style
  can match it. `Extension<CspNonce>` is unchanged; a page served with another
  `Content-Type` loses its nonce-carrying inline scripts and styles.
