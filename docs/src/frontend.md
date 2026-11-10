# Frontend: htmx, native HTML, Tailwind and Alpine

Rullst pages are rendered on the server with `html!` and sent as HTML. The
framework ships no JavaScript bundle, CSS compiler or bundler, and every page
runs under a strict Content-Security-Policy with a per-request nonce. Since
13.0.0-alpha.5 three opt-in pieces cover what most applications add on top:

| Piece | What it is | Maintenance |
| :--- | :--- | :--- |
| `rullst::assets::url` | Content-hashed URLs for files in `static/`, served `immutable` | Part of `rullst-core` |
| `cargo rullst add tailwind` | Tailwind CSS v4 through its official standalone CLI (one binary, no Node.js) | A pinned version and one SHA-256 per platform |
| `cargo rullst add alpine` | Alpine.js's CSP build vendored into `static/vendor/` | A pinned version and its npm integrity |

## Which tool for which job

1. **The server first, with htmx.** Anything that reads or changes data goes
   to the server: `hx-get`/`hx-post` and a fragment returned by a handler
   ([tutorial 06](tutorials/06-htmx-zero-bundle.md)). The state stays where the
   authorization checks are. Updates the server pushes (a dashboard, a
   notification) arrive over Server-Sent Events: `rullst::realtime::RealtimeSse`
   streams a channel to the htmx SSE extension that new projects ship as
   `static/htmx-ext-sse-1.9.12.js`
   ([tutorial 34](tutorials/34-realtime-dashboard-sse.md)).
2. **Native HTML before script.** Disclosure, dialogs and menus need no
   JavaScript in current browsers:
   `<details>`/`<summary>` for disclosure and accordions, `<dialog>` with a
   `<form method="dialog">` button for modals, and the `popover` attribute
   with `popovertarget` for menus, tooltips and toasts. They are accessible
   and keyboard-ready by default and work under the strict CSP.
3. **Alpine's CSP build for client-only state.** A character counter, a
   client-side filter of rows already on the page, a "copy" button: state
   that never needs the server. Logic lives in components registered with
   `Alpine.data` in a same-origin `.js` file.
4. **Tailwind when you want utility classes.** It is optional: a plain
   stylesheet in `static/` works the same way with `rullst::assets::url`.

## Content-hashed asset URLs

Link static files with `rullst::assets::url`, directly inside `html!`:

```rust
use rullst::html;
use rullst::html::Markup;

fn head() -> Markup {
    html! {
        <link rel="stylesheet" href={rullst::assets::url("app.css")} />
        <script defer src={rullst::assets::url("js/app.js")}></script>
    }
}

// Outside a running server the URL is the plain one.
assert!(head().as_str().contains("href=\"/static/app.css\""));
```

The path is relative to the project's `static/` directory. While the server
runs, `url("app.css")` returns `/static/app.<hash>.css`, where `<hash>` is
the first 16 hexadecimal characters of the SHA-256 of the file. The static
mount resolves that name to `static/app.css` and answers with
`Cache-Control: public, max-age=31536000, immutable`, so a browser or CDN
keeps the file until its content changes, and with it the URL. This replaces
hand-made `?v=<hash>` query strings: a CDN that lengthens the browser TTL of
`no-cache` responses (Cloudflare's default browser TTL is four hours) can no
longer keep an old stylesheet after a deploy.

- **When files are hashed.** When `Server` mounts `static/`: up to 10,000
  files of up to 32 MiB each, 16 directory levels deep. In **debug builds**
  each call compares the file's size and modification time with the recorded
  ones and hashes it again when they differ, and a file created after the
  start is hashed on its first use, so `cargo rullst dev` (and the Tailwind
  watcher) never leaves a page on an old stylesheet. **Release builds** keep
  the start-up hashes for the links they write.
- **An outdated hash.** A request whose hash no longer matches the file, for
  example from a page cached before a deploy, gets `307 Temporary Redirect`
  to the current URL with `Cache-Control: no-store`. The new content is never
  served as immutable under the old name. The server checks size and
  modification time on every hashed request, in every build. During a rolling
  deploy an old replica can still answer a new hash with a redirect to its own
  file; finish the rollout before purging a CDN.
- **Plain URLs.** `url` returns `/static/<path>` unchanged (served with
  `no-cache` as before) when no server has mounted `static/` in the process
  (unit tests, a function called outside a server), the file does not exist,
  is a symbolic link or lies below one, exceeds the limits above, has no
  extension, is a `.br`/`.zst` sibling, already carries a content hash
  (`app.3f9a2c1b.css`), or its path has a segment that is hidden (`.well-known`)
  or uses characters other than ASCII letters, digits, `-`, `_`, `.` and `~`.
- **Unchanged rules.** The static service still serves the real file:
  Brotli and Zstandard siblings (`app.css.br`, `app.css.zst`), `ETag`
  validators, path-traversal checks and the handling of symbolic links are
  those of [the `/static` mount](tutorials/10-static-assets-and-compression.md).
  A real file whose name looks hashed is served as it is.

The helper reads the manifest of the last `static/` directory mounted in the
process. An application that serves static files itself (its own `ServeDir`)
gets plain URLs.

## `cargo rullst add tailwind`

```bash
cargo rullst add tailwind --dry-run   # the planned diff; downloads nothing
cargo rullst add tailwind
```

The command, at the root of a Rullst project:

1. **Downloads the pinned CLI** — Tailwind CSS **v4.3.3**, the
   `tailwindcss-<platform>` asset of the
   [official release](https://github.com/tailwindlabs/tailwindcss/releases/tag/v4.3.3)
   (Linux x64/arm64 with glibc or musl, macOS x64/arm64, Windows x64; Windows
   on Arm runs the x64 build) — into the per-user cache:
   `$XDG_CACHE_HOME/rullst/tailwindcss/v4.3.3/` or
   `~/.cache/rullst/tailwindcss/v4.3.3/` on Linux and macOS,
   `%LOCALAPPDATA%\rullst\tailwindcss\v4.3.3\` on Windows. The download is
   written to a temporary file, checked against the SHA-256 embedded in the
   CLI, and only then made executable and moved into place. The cached file
   is checked again **before every use** (`add`, `dev`, `build`); a file that
   no longer matches is refused and never run. Nothing is written to the
   project when the download fails.
2. **Writes the input** `assets/css/app.css`:

   ```css
   @import "tailwindcss" source(none);
   @source "../../src/**/*.rs";
   ```

   `source(none)` turns off Tailwind's automatic detection so only your Rust
   sources are scanned (not `target/` or `static/vendor/`). Add an `@source`
   line for templates kept elsewhere. Tailwind finds class names by reading
   the source text, so **a class built at run time is not detected**:
   `format!("text-{color}-600")` produces no CSS. Write each complete name
   (`text-red-600`, `text-green-600`) somewhere in the source, for example in
   a `match` that returns string literals.
3. **Wires the output to `static/app.css`** and runs a first build.
4. **Links it in the layout** when the layout is recognisable: exactly one
   `html!` invocation in `src/` closes `<head>` (on its own line). The line
   added before `</head>` is
   `<link rel="stylesheet" href={rullst::assets::url("app.css")} />`.
   Otherwise the command prints that line and where it goes. Pages rendered
   with `render_page*` have a framework-owned `<head>`: pass the line as the
   `head` argument of `rullst::htmx::render_page_with_head`. A project still
   locked to a `rullst` older than 13.0.0-alpha.5 gets the line printed, with
   a reminder to run `cargo rullst upgrade` first.
5. **`.gitignore`**: `static/app.css` is a build output, so `/static/app.css`
   is added to an existing `.gitignore`. A project that already tracks
   `static/app.css` in Git commits its built assets and keeps doing so; the
   file is then left alone.
6. **Dockerfile**: a Dockerfile generated by `cargo rullst new`, `deploy` or
   `dockerize` gains two stages, and the generators add them from then on.
   BuildKit downloads the CLI for the build machine's architecture (amd64 or
   arm64) with `ADD --checksum=sha256:…`, which refuses the file unless it
   matches; a stage on the build platform runs
   `tailwindcss --input assets/css/app.css --output /css/app.css --minify`,
   and the runtime stage copies the result to `/app/static/app.css`. It needs
   BuildKit (Docker 25 or later, `docker buildx`, or Podman 4.8 or later). A
   hand-written Dockerfile gets the stages printed instead.

It stops before downloading or writing anything when `assets/css/app.css`
exists without importing `tailwindcss`, or when `static/app.css` exists and was
not built by Tailwind (the build would overwrite it). Running it again
changes nothing.

**Development and release builds.** `cargo rullst dev` builds the stylesheet
once before the application starts, then runs `tailwindcss --watch` next to
it and stops it with the session (it also exits when `dev` dies, because it
watches its standard input). Tailwind writes the file only when the CSS
changes; each write reloads the page like any other change in `static/`. If
the CLI cannot be prepared (offline without a cached copy), `dev` reports it
and runs the server without rebuilding the stylesheet.
`cargo rullst build` writes the minified stylesheet before pre-compressing
`static/`, and fails if the CLI cannot run, so a release never ships a stale
file.

**Updating Tailwind** is a change to one file of the CLI,
`cargo-rullst/src/generators/frontend/tailwind_release.rs`: the version and
the SHA-256 of each asset. The v4.3.3 checksums come from the release's
`sha256sums.txt` asset, are identical to the asset digests the GitHub API
reports, and were confirmed by downloading `tailwindcss-linux-x64` and hashing
it. A unit test checks that every platform the CLI runs on has one entry.
Pins follow a **14-day cooldown**: the newest release published at least 14
days before the pin is made, so a compromised or broken release has time to
be noticed. On 2026-11-10 that was v4.3.3 (published 2026-07-16); v4.3.4 and
v4.3.5 were younger. The Alpine pin follows the same rule.

## `cargo rullst add alpine`

```bash
cargo rullst add alpine --dry-run
cargo rullst add alpine
```

Standard Alpine.js evaluates directive values with `new Function`, which
needs `'unsafe-eval'` in `script-src`; Rullst's policy never allows it.
Alpine's [CSP build](https://alpinejs.dev/advanced/csp) (`@alpinejs/csp`)
parses expressions itself instead. The command:

1. **Vendors `@alpinejs/csp` 3.17.5** (published 2026-10-20, past the 14-day
   cooldown) into
   `static/vendor/alpine-csp.min.js`: it downloads the npm registry tarball
   (`https://registry.npmjs.org/@alpinejs/csp/-/csp-3.17.5.tgz`), checks it
   against the registry's `dist.integrity` (SHA-512) embedded in the CLI, and
   reads `package/dist/cdn.min.js` from it in memory. Nothing in the tarball
   is unpacked or run. The file starts with a one-line banner naming the
   version and Alpine's MIT licence; running the command again keeps a file
   with that banner and replaces an older one.
2. **Writes an example component**, `static/js/alpine-components.js`
   (never overwritten), registered with `Alpine.data` on `alpine:init`.
3. **Loads both** in the recognisable layout (or prints the lines), as
   deferred same-origin scripts, components first so their `alpine:init`
   listener exists when Alpine starts:

   ```text
   <script defer src={rullst::assets::url("js/alpine-components.js")}></script>
   <script defer src={rullst::assets::url("vendor/alpine-csp.min.js")}></script>
   ```

   Same-origin files are allowed by `script-src 'self'`; they need no nonce.
4. **Prints the example markup** for a page:

   ```text
   <div x-data="charCounter(160)">
       <label for="message">"Message"</label>
       <textarea id="message" name="message" x-model="text"></textarea>
       <p aria-live="polite" x-text="remaining + ' characters left'"></p>
       <button type="button" x-bind="clear">"Clear"</button>
   </div>
   ```

**Events and bindings in `html!`.** `html!` attribute names are identifiers
joined by `-`, so Alpine's `x-on:click`, `@click`, `x-bind:disabled` and
`:disabled` cannot be written there. Put them in a directive object of the
component and bind it with `x-bind="name"`, as `clear` does:

```javascript
clear: {
  ["@click"]() { this.text = ""; },
  [":disabled"]() { return this.text.length === 0; },
},
```

### What the CSP build does not support

From [Alpine's CSP page](https://alpinejs.dev/advanced/csp) and the parser of
the vendored version (`src/parser.js` in `@alpinejs/csp` 3.17.5). Directive
values may use property access (`user.name`, `items[0]`), calls of component
methods (`items.push('c')`, `charCounter(160)`), arithmetic, comparisons,
`&&`, `||`, `!`, the ternary `?:`, string concatenation, `=` and `++`/`--`,
and object and array literals. They may **not** use:

- arrow functions (`() => …`) or `function` expressions;
- template literals (`` `Hello ${name}` ``), destructuring
  (`{ name } = user`) or spread (`{ ...defaults }`);
- several statements (`a = 1; b = 2` — one expression only, with an optional
  final `;`), `+=` and the other compound assignments, `?.`, `??`, `**`,
  bitwise operators, `typeof`, `instanceof`, `in`, `new`, `delete` and
  regular-expression literals;
- globals: `window`, `document`, `console`, `Math`, `JSON`, `parseInt` and
  every other global object or function; a name that is not in the
  component's scope is an error;
- HTML injection: `x-html` is disabled and `insertAdjacentHTML`,
  `setAttribute`, `appendChild` and similar DOM methods are refused;
- assignments to properties of DOM objects, and expressions on `<script>` or
  `<iframe>` elements. Alpine's page also lists assignments to data
  properties (`user.name = 'John'`) as unsupported; 3.17.5 accepts them on
  plain data objects, but keep them in methods so an update cannot break them.

Anything else belongs in a component method or getter in the components
file, which is ordinary JavaScript.

**Hide before start.** `x-cloak` needs the rule
`[x-cloak] { display: none !important; }` in a stylesheet file (with
Tailwind, after the `@import` in `assets/css/app.css`); an inline `<style>`
without the nonce is blocked.

**With htmx.** Alpine initialises components in content htmx swaps in, so a
fragment can carry `x-data`. Keep server state on the server: a component
should not duplicate what an htmx request returns.

## The CSP rules these tools follow

Rullst's default policy is
`script-src 'self' 'nonce-…'; style-src 'self' 'nonce-…'` (see the
[security layers](security-layers.md)). What works under it:

- `<script src="/static/…">` and `<link rel="stylesheet" href="/static/…">`:
  same-origin files, the form every generated tag uses;
- inline `<script>` or `<style>` only with `nonce={nonce}` from the request's
  `CspNonce`;
- no `style="…"` attributes and no `onclick="…"` handlers in markup; Alpine's
  `x-show` and transitions set styles through the CSSOM, which the policy
  allows;
- no `eval`, `new Function` or string timers: the Alpine CSP build needs
  none, and the components file must not either.

A unit test of the CLI renders a page with the generated tags through the real
security baseline and checks that the policy has no `'unsafe-eval'` or
`'unsafe-inline'` and that the page has no inline script or style without the
nonce.
