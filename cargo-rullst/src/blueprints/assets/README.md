# Vendored browser client

`htmx-1.9.12.min.js` and `htmx-ext-sse-1.9.12.js` come from the official
HTMX repository, version 1.9.12, commit
`f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d`:

- [Upstream script](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/htmx.min.js)
- [Upstream Server-Sent Events extension](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/ext/sse.js)
- [Upstream Zero-Clause BSD license](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/LICENSE), retained as `HTMX-LICENSE`.

The vendored `htmx-1.9.12.min.js` adds one terminal LF to the upstream
minified text; `htmx-ext-sse-1.9.12.js` is the upstream file unchanged. No
JavaScript behavior is patched. SHA-256:

| Artifact | Digest |
| --- | --- |
| Upstream `htmx.min.js` bytes | `449317ade7881e949510db614991e195c3a099c4c791c24dacec55f9f4a2a452` |
| Vendored `htmx-1.9.12.min.js` including final LF | `73eabc44d978b226a667c62ca3c40e99236d11aa6f8fc8a27be6f0b36a73b42d` |
| Upstream and vendored `htmx-ext-sse-1.9.12.js` | `be05b2e2265279f035271adbea0b72a356f20ce4dfa5870481bfe9c51b822fc1` |

`browser_assets.rs` checks both vendored digests and that a new project's
`static/` receives these exact bytes.

HTML scaffolds emit both scripts and the license into `static/`; the standard
page helper and ERP load the same-origin htmx script. A page that uses
`hx-ext="sse"` loads `/static/htmx-ext-sse-1.9.12.js` itself, after htmx and
with the request's CSP nonce, and connects to a route that returns
`rullst::realtime::RealtimeSse`. Applications using `render_page` without
CLI-generated assets must provide the files at those paths themselves.
This removes a runtime CDN dependency; it is not a browser-security
certification or a claim that every separate Studio/Nexus asset is vendored.

Updates require upstream provenance, license/digest review and HTMX/CSP browser
regressions. The vendored JavaScript is outside Cargo dependency scanning and
therefore needs explicit review when upstream security changes are published.
