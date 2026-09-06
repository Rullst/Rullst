# Vendored browser client

`htmx-1.9.12.min.js` comes from the official HTMX repository, version 1.9.12,
commit `f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d`:

- [Upstream script](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/htmx.min.js)
- [Upstream Zero-Clause BSD license](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/LICENSE), retained as `HTMX-LICENSE`.

The vendored file adds one terminal LF to the upstream minified text. No
JavaScript behavior is patched. SHA-256:

| Artifact | Digest |
| --- | --- |
| Upstream bytes | `449317ade7881e949510db614991e195c3a099c4c791c24dacec55f9f4a2a452` |
| Vendored file including final LF | `73eabc44d978b226a667c62ca3c40e99236d11aa6f8fc8a27be6f0b36a73b42d` |

HTML scaffolds emit the script and license into `static/`; the standard page
helper and ERP load the same-origin script. Applications using `render_page`
without CLI-generated assets must provide the file at that path themselves.
This removes a runtime CDN dependency; it is not a browser-security
certification or a claim that every separate Studio/Nexus asset is vendored.

Updates require upstream provenance, license/digest review and HTMX/CSP browser
regressions. The vendored JavaScript is outside Cargo dependency scanning and
therefore needs explicit review when upstream security changes are published.
