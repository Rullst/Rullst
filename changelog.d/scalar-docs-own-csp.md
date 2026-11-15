### Fixed

- `/docs` from `rullst::scalar::scalar_docs_router` (and the controller that
  `cargo rullst make:scalar` generates) was blank behind the security
  baseline, whose strict default Content Security Policy blocked the page's
  scripts; since 13.0.0-alpha.6 that applies in development too. The page now
  answers with a policy of its own, which the baseline keeps: the request's
  nonce on its inline style and scripts (and in a `csp-nonce` meta element,
  from which Scalar takes the nonce for the styles it inserts), the one pinned
  file `@scalar/api-reference@1.67.0/dist/browser/standalone.js`, loaded with
  Subresource Integrity, and the hashes of the style elements Scalar inserts
  without the nonce. It has no `'unsafe-eval'`, and `'unsafe-inline'` only in
  `style-src-attr`, for the computed `style` attributes Scalar sets. Scalar's
  remote fonts and its "Ask AI" agent, which calls `api.scalar.com`, are
  turned off. In Chromium the page renders with no policy violation and no
  console error.
  `render_scalar_html` returns the page without a nonce, loading the same
  pinned file with its integrity.
