# Vendored browser client

HTML scaffolds (`cargo rullst new`, Blank and SaaS) write these files to
`static/`. Since 13.0.0-alpha.5 they are htmx 2 and its separately packaged
Server-Sent Events extension, taken from the npm registry tarballs (not a CDN)
and checked against the registry's `integrity`:

| File | Package | Tarball file | Registry `integrity` of the tarball |
| --- | --- | --- | --- |
| `htmx-2.0.11.min.js` | [`htmx.org` 2.0.11](https://registry.npmjs.org/htmx.org/-/htmx.org-2.0.11.tgz), published 2026-09-22 | `package/dist/htmx.min.js` | `sha512-Thx/WtpeOQqSrqBCw/A1cwGJGg4UrVa3+sW0GmrM3p4gJgO89ecH4qtbnyzDDWFvBTqjnIMCgELTNt636dtamA==` |
| `htmx-ext-sse-2.2.4.js` | [`htmx-ext-sse` 2.2.4](https://registry.npmjs.org/htmx-ext-sse/-/htmx-ext-sse-2.2.4.tgz), published 2025-10-18 | `package/dist/sse.js` | `sha512-LJmxVhykyflBWgh5PvbRidcyuqMHlgfajmmzumvKctv9puvsufeH6OaejSMZTNTnEI8O2wXXn4ZZtBdpEMqmEQ==` |
| `HTMX-LICENSE` | `htmx.org` 2.0.11 | `package/LICENSE` (Zero-Clause BSD) | |
| `HTMX-EXT-SSE-LICENSE` | `htmx-ext-sse` 2.2.4 | `package/LICENSE` (Zero-Clause BSD) | |

Both are the `latest` dist-tag of their package, checked on 2026-11-12 with
`https://registry.npmjs.org/htmx.org` and `https://registry.npmjs.org/htmx-ext-sse`
(`htmx.org` 4.x is published only under the `next` tag and is not used). They
follow the 14-day cooldown of the other pinned frontend tools
(`cargo-rullst/src/generators/frontend/tailwind_release.rs`): 2.0.11 was 51
days old when it was pinned. Sources:
[htmx](https://github.com/bigskysoftware/htmx) (`gitHead`
`fa978b24e75fb03c137bf2cdae4fef0e711cf8a1`) and
[htmx-extensions](https://github.com/bigskysoftware/htmx-extensions) (`gitHead`
`5f10b3bf036455162009df5394ba82837e034085`).

`htmx-2.0.11.min.js` adds one terminal LF to the upstream minified text and
changes no behavior. `htmx-ext-sse-2.2.4.js` is the upstream file unchanged.

## The SSE reconnect patch is gone

Until 13.0.0-alpha.5 Rullst shipped a patched copy of htmx 1.9.12's
`dist/ext/sse.js`, whose reconnect delay after a failed connection used
`2 ^ retryCount`, a bitwise XOR in JavaScript, so it never grew past about 2.5
seconds. The htmx 2 extension 2.2.4 fixes this upstream: after the browser
gives up on a stream (an error status or a wrong content type, for example
from a proxy) it waits 0.5, 1, 2, … up to 64 seconds
(`retryCount = Math.max(Math.min(retryCount * 2, 128), 1)`, `timeout =
retryCount * 500`) and resets the count when a connection opens. Rullst
therefore ships it unmodified. `rullst::realtime::RealtimeSse` itself never
answers with an error: it ends a refused stream with a `retry:` delay, which
the browser's own `EventSource` reconnection honours without the extension.

## Digests

| Artifact | SHA-256 | SRI |
| --- | --- | --- |
| Upstream `dist/htmx.min.js` bytes | `d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717` | |
| Vendored `htmx-2.0.11.min.js` including final LF | `7f14135fe8ee3ed5f1a85fea21ba4dfba8236d697c4f95ab575d1c9091bb06ca` | `sha384-0rfIGBBgDyW0JtoZiNfcOM2jtv0x8QjJvQv7XDKYD8IbMa5Zz13HgPszfLfX2Sa8` |
| Vendored `htmx-ext-sse-2.2.4.js` (upstream `dist/sse.js`) | `3b5992a541619babefc4c169505af474df5c3039da51e59b96ccf9241ecd61d2` | `sha384-QA9wXqexhwzXTuTvuF5QP82pddm3R2hy81UzXi7ioNTqNF2b75hlkkSGjafohhL3` |

`browser_assets.rs` checks these digests, the version string, the extension's
backoff lines and that a new project's `static/` receives these exact bytes.

## Use

Core's page helpers (`rullst::htmx::render_page*`) load
`/static/htmx-2.0.11.min.js` with the request's CSP nonce. An application
created before 13.0.0-alpha.5 keeps its `static/htmx-1.9.12.min.js`, which
the helpers keep loading while it is the only htmx file the server publishes;
`cargo rullst upgrade` reports `V13-HTMX-2` with the steps to move it to htmx 2.
A page that uses `hx-ext="sse"` loads `/static/htmx-ext-sse-2.2.4.js` itself,
after htmx and with the request's CSP nonce, and connects to a route that
returns `rullst::realtime::RealtimeSse`. Applications using `render_page`
without CLI-generated assets must provide the files at those paths
themselves. This removes a runtime CDN dependency; it is not a
browser-security certification.

Updates require upstream provenance (the registry tarball and its
`integrity`), the cooldown, a license and digest review, and the htmx/CSP
browser regressions (the "Generated starters in browsers" CI job). The
vendored JavaScript is outside Cargo dependency scanning and therefore needs
explicit review when upstream security changes are published.
