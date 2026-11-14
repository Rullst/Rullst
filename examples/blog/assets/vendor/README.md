# Vendored browser assets

The blog showcase serves these files from its own origin (`/assets/vendor/...`)
so its pages render under the production Content Security Policy
(`script-src 'self' 'nonce-…'; style-src 'self' 'nonce-…'`) without a CDN or a
relaxed policy. No JavaScript or CSS behavior is patched.

| File | Upstream | License |
| --- | --- | --- |
| `htmx-1.9.12.min.js` | [htmx `dist/htmx.min.js`](https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/htmx.min.js), tag `v1.9.12`, commit `f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d` | Zero-Clause BSD, `HTMX-LICENSE` |
| `pico-2.1.1.slate.min.css` | [Pico CSS `css/pico.slate.min.css`](https://github.com/picocss/pico/blob/1039a4788d6abc368d5485ae6bac84a8f0e3096f/css/pico.slate.min.css), tag `v2.1.1`, commit `1039a4788d6abc368d5485ae6bac84a8f0e3096f` (identical to the npm `@picocss/pico@2.1.1` file) | MIT, `PICO-LICENSE.md` |

The two minified files add one terminal LF to the upstream text. SHA-256:

| Artifact | Upstream bytes | Vendored file |
| --- | --- | --- |
| `htmx-1.9.12.min.js` | `449317ade7881e949510db614991e195c3a099c4c791c24dacec55f9f4a2a452` | `73eabc44d978b226a667c62ca3c40e99236d11aa6f8fc8a27be6f0b36a73b42d` |
| `pico-2.1.1.slate.min.css` | `2830e93a11683edcd55d128ec84d842337ae041e59ea395a2f9150df80a67a68` | `fb2390b6835da75dbfc49e142a781a70b759d6e0b50163cbbaf9569328881ae0` |

`htmx-1.9.12.min.js` is byte-identical to the copy the CLI wrote into new
projects before 13.0.0-alpha.5 (they now get htmx 2.0.11). These files are outside Cargo
dependency scanning: an update needs upstream provenance, a license and digest
review and a browser check of the affected showcase pages.
