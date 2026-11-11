# Vendored browser client

`htmx-1.9.12.min.js` and `htmx-ext-sse-1.9.12.js` come from the official
HTMX repository, version 1.9.12, commit
`f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d`:

- [Upstream script](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/htmx.min.js)
- [Upstream Server-Sent Events extension](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/ext/sse.js)
- [Upstream Zero-Clause BSD license](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/LICENSE), retained as `HTMX-LICENSE`.

The vendored `htmx-1.9.12.min.js` adds one terminal LF to the upstream
minified text and changes no behavior. `htmx-ext-sse-1.9.12.js` is the
upstream file with one fix, so Rullst ships a **patched copy** under the
upstream name. Upstream computes the reconnect delay after a failed
connection with `2 ^ retryCount`, which in JavaScript is a bitwise XOR: the
delay never grows past about 2.5 seconds and is sometimes zero, so a server
that keeps refusing is retried at full speed. The copy grows the delay as a
real power of two (a random delay below 0.5, 1, 2, … up to 64 seconds, the
existing cap of seven doublings) and resets it when a connection opens, as
the later htmx 2 extension does. A header comment in the file records the
patch:

```diff
@@ -3,6 +3,9 @@
 ============================
 This extension adds support for Server Sent Events to htmx.  See /www/extensions/sse.md for usage instructions.
 
+Patched by Rullst: the reconnect delay grows as 2^n (upstream computes the
+bitwise XOR `2 ^ n`) and a successful connection resets it.
+
 */
 
 (function() {
@@ -252,7 +255,7 @@
 			// Otherwise, try to reconnect the EventSource
 			if (source.readyState === EventSource.CLOSED) {
 				retryCount = retryCount || 0;
-				var timeout = Math.random() * (2 ^ retryCount) * 500;
+				var timeout = Math.random() * Math.pow(2, retryCount) * 500;
 				window.setTimeout(function() {
 					ensureEventSourceOnElement(elt, Math.min(7, retryCount + 1));
 				}, timeout);
@@ -260,6 +263,7 @@
 		};
 
 		source.onopen = function(evt) {
+			retryCount = 0;
 			api.triggerEvent(elt, "htmx:sseOpen", { source: source });
 		}
 
```

This path runs when the browser gives up on a stream (an error status or a
wrong content type, for example from a proxy). `rullst::realtime::RealtimeSse`
itself never answers with an error: it ends a refused stream with a `retry:`
delay, which the browser's own reconnection honours.

SHA-256:

| Artifact | Digest |
| --- | --- |
| Upstream `htmx.min.js` bytes | `449317ade7881e949510db614991e195c3a099c4c791c24dacec55f9f4a2a452` |
| Vendored `htmx-1.9.12.min.js` including final LF | `73eabc44d978b226a667c62ca3c40e99236d11aa6f8fc8a27be6f0b36a73b42d` |
| Upstream `dist/ext/sse.js` | `be05b2e2265279f035271adbea0b72a356f20ce4dfa5870481bfe9c51b822fc1` |
| Vendored, patched `htmx-ext-sse-1.9.12.js` | `afb588b88863ec49fe7d9f958b788ab1e1df1f45016466d2ea573f374c89e5d4` |

`browser_assets.rs` checks these digests, the patched lines, and that a new
project's `static/` receives these exact bytes.

HTML scaffolds emit both scripts and the license into `static/`; the standard
page helper and ERP load the same-origin htmx script. A page that uses
`hx-ext="sse"` loads `/static/htmx-ext-sse-1.9.12.js` itself, after htmx and
with the request's CSP nonce, and connects to a route that returns
`rullst::realtime::RealtimeSse`. Applications using `render_page` without
CLI-generated assets must provide the files at those paths themselves.
This removes a runtime CDN dependency; it is not a browser-security
certification or a claim that every separate Studio/Nexus asset is vendored.

Updates require upstream provenance, license/digest review, re-applying (or
dropping, once upstream fixes it) the SSE backoff patch, and HTMX/CSP browser
regressions. The vendored JavaScript is outside Cargo dependency scanning and
therefore needs explicit review when upstream security changes are published.
