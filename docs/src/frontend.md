# Frontend: htmx, native HTML, Tailwind and Alpine

Rullst pages are rendered on the server with `html!` and sent as HTML. The
framework ships no JavaScript bundle, CSS compiler or bundler, and every page
runs under a strict Content-Security-Policy with a per-request nonce. Since
13.0.0-alpha.5 four opt-in pieces cover what most applications add on top:

| Piece | What it is | Maintenance |
| :--- | :--- | :--- |
| `rullst::assets::url` | Content-hashed URLs for files in `static/`, served `immutable` | Part of `rullst-core` |
| `cargo rullst add tailwind` | Tailwind CSS v4 through its official standalone CLI (one binary, no Node.js) | A pinned version and one SHA-256 per platform |
| `cargo rullst add alpine` | Alpine.js's CSP build vendored into `static/vendor/` | A pinned version and its npm integrity |
| `cargo rullst ui add` | Accessible `html!` components styled with Tailwind, copied into `src/ui/` | Yours once copied |

## Which tool for which job

1. **The server first, with htmx.** Anything that reads or changes data goes
   to the server: `hx-get`/`hx-post` and a fragment returned by a handler
   ([tutorial 06](tutorials/06-htmx-zero-bundle.md)). The state stays where the
   authorization checks are. Updates the server pushes (a dashboard, a
   notification) arrive over Server-Sent Events: `rullst::realtime::RealtimeSse`
   streams a channel to the htmx SSE extension that new projects ship as
   `static/htmx-ext-sse-2.2.4.js`
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

## The htmx client

`cargo rullst new` (Blank and SaaS) writes htmx **2.0.11** to
`static/htmx-2.0.11.min.js` and the htmx 2 Server-Sent Events extension
**2.2.4** to `static/htmx-ext-sse-2.2.4.js`, with their Zero-Clause BSD
licenses. Both are the unmodified files of the npm packages `htmx.org` and
`htmx-ext-sse`, checked against the registry's integrity;
`cargo-rullst/src/blueprints/assets/README.md` records their provenance and
digests. Core's `render_page*` load htmx with the request's CSP nonce; a page
that uses `hx-ext="sse"` loads the extension itself, after htmx.

htmx needs neither `'unsafe-inline'` nor `'unsafe-eval'` for what Rullst
pages use. Under Rullst's security headers `render_page*` turn off htmx's
nonce-less indicator `<style>` (`includeIndicatorStyles: false`) and send the
same rules with the nonce. What htmx evaluates as JavaScript (`hx-on*`,
`hx-vars`, `js:` values in `hx-vals`, trigger filters such as
`click[ctrlKey]`) is refused by the strict CSP, in htmx 1 and 2 alike: use a
same-origin script that listens to htmx's events instead.

**Applications created before 13.0.0-alpha.5** vendor htmx 1.9.12 in
`static/htmx-1.9.12.min.js`. They keep working without a change:
`render_page*` load that file, with htmx 1.9.12's indicator rules, while the
`static/` directory the server publishes holds it and not
`htmx-2.0.11.min.js`. `cargo rullst upgrade` reports `V13-HTMX-2` (review)
until the application moves. To move, copy `htmx-2.0.11.min.js`,
`htmx-ext-sse-2.2.4.js`, `HTMX-LICENSE` and `HTMX-EXT-SSE-LICENSE` from the
`static/` of a new project, change layouts of your own that name the old
file, delete `static/htmx-1.9.12.min.js` and check the htmx 2 changes that
can reach a Rullst page (the full list is htmx's
[1.x → 2.x migration guide](https://htmx.org/migration-guide-htmx-1/)):

- `hx-delete` sends its parameters in the query string, like `hx-get`, so a
  hidden `_token` field no longer reaches the CSRF middleware: send the token
  in the `X-CSRF-Token` header (`hx-headers`), or use `hx-post` to a delete
  route.
- `hx-on="click: …"` is written `hx-on:click="…"` (`hx-on::after-request`
  for htmx events).
- Requests to another origin are refused (`htmx.config.selfRequestsOnly`).
- The `hx-sse` and `hx-ws` attributes are gone: use `hx-ext="sse"` with
  `sse-connect`/`sse-swap`, and the htmx 1.9.12 SSE extension does not work
  with htmx 2.
- The history cache is kept in `sessionStorage`; a cache miss still sends
  `HX-History-Restore-Request: true` with `HX-Request: true`, which
  `render_page*` answer with the whole page.

The `HX-*` headers that `HtmxRequest` reads (`HX-Request`, `HX-Trigger`,
`HX-Target`, `HX-Prompt`, `HX-Current-URL`, `HX-Boosted`,
`HX-History-Restore-Request`) and that `HtmxResponse` writes (`HX-Trigger`,
`HX-Redirect`, `HX-Refresh`) mean the same in htmx 2.

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

Core's own page (`rullst::htmx::render_page`, `render_page_with_lang` and
`render_page_with_head`) links its stylesheet and the htmx client this way
since 13.0.0-alpha.5: `/static/rullst.<hash>.css` and
`/static/htmx-2.0.11.min.<hash>.js` while the server runs, so an edited
`static/rullst.css` reaches every visitor on the next deploy. The favicon
keeps its plain `/static/rullst.png`. The service worker `cargo rullst make:pwa`
writes stores the static files its offline page links when it installs, so
the hashed names work offline; a worker generated before 13.0.0-alpha.5
stores the hashed files the first time a page it controls loads them.

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
   and the result becomes `static/app.css`: in a Dockerfile generated since
   13.0.0-alpha.5 it is copied into the `static-assets` stage, which writes
   its `.br`/`.zst` siblings with the other static files; in an older one the
   runtime stage copies it to `/app/static/app.css`. It needs
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
file. A plain `cargo build`, `cargo run` or a binary started by a script does
**not** run Tailwind: classes added to the sources since the last build are
missing from `static/app.css` and the pages look unstyled. Use `cargo rullst
dev` or `cargo rullst build`, or run the pinned CLI yourself
(`tailwindcss -i assets/css/app.css -o static/app.css --minify`) before
starting the binary; `cargo rullst doctor` warns when `static/app.css` is
missing or older than `assets/css/app.css` or a file its `@source` lines scan
(since 13.0.0-alpha.5).

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

**Events and bindings in `html!`.** Since 13.0.0-alpha.5 `html!` accepts
Alpine's attribute names, so `@click`, `x-on:keydown.enter.prevent`,
`:disabled` and `x-bind:class` can be written in the markup
([text and attribute names](tutorials/06-htmx-zero-bundle.md#text-and-attribute-names)):

```text
<button type="button" @click="text = ''" :disabled="text.length === 0">"Clear"</button>
```

Their values run as JavaScript, so they must be string literals; a `{…}`
value is a compile error (pass server data through a `data-*` attribute).
To keep several directives together, put them in a directive object of the
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

## Components

```bash
cargo rullst ui list
cargo rullst ui add button dialog --dry-run   # the planned files and diff
cargo rullst ui add button dialog
cargo rullst ui add --all
```

`ui add` copies components into the application as Rust source, the way
shadcn/ui and Topcoat copy theirs: after the copy the code is yours to change.
Each component is a function returning `rullst::html::Markup`, with a small
options struct whose variants and sizes are enums, styled with Tailwind
utility classes written in full (so the Tailwind CLI finds them in
`src/ui/*.rs`). They follow the order of [which tool for which
job](#which-tool-for-which-job): native HTML first (`<dialog>`, `popover`,
`<table>`), htmx for anything that talks to the server, and a small
same-origin ES module only for keyboard behaviour HTML does not have yet. No
component needs Alpine.js, an inline script, a `style` attribute or `eval`.

The command, at the root of a Rullst project:

1. writes `src/ui/<name>.rs` for each component, plus the ones it uses
   (`dialog` and `menu` call `ui::button`);
2. for `dialog`, `menu`, `tabs` and `toast`, writes `static/ui/<name>.js`, which
   the component's markup loads with
   `<script type="module" src={rullst::assets::url("ui/<name>.js")}>`
   (a browser runs a module once per page however many components load it, and
   the modules use event delegation, so content htmx swaps in works);
3. creates or extends `src/ui/mod.rs` with a `pub mod <name>;` line per
   component (a created file has `#![allow(dead_code)]`, as an application
   rarely uses every helper);
4. declares the module in the crate root: `pub mod ui;` in `src/lib.rs` when
   the project has one, else `mod ui;` in `src/main.rs`. Use the components as
   `crate::ui::button::button(…)` (or `<crate>::ui::…` from the binary of a
   library project).

Running it again changes nothing. A copied file that differs from the copy
this CLI ships (you changed it, or another CLI version copied it) stops the
command before anything is written; `--dry-run` shows the difference and
`--force` replaces it. `--all` copies every component.

**Tailwind.** The classes need the project's Tailwind build. When
`uses_tailwind` finds none (no `assets/css/app.css` importing `tailwindcss`,
`tailwind.config.*` or Tailwind in `package.json`), an interactive run asks
whether to run [`cargo rullst add tailwind`](#cargo-rullst-add-tailwind)
first, answering Yes by default; scripts, CI and `--dry-run` never ask (and never download): they copy the
components and print the command. Without Tailwind the components still work
and stay accessible, unstyled. `add tailwind`'s `@source "../../src/**/*.rs"`
already covers `src/ui/`; a project that builds Tailwind itself must include
`src/ui/*.rs` in its sources.

**Colours.** Each component has light and dark styles (Tailwind's `dark:`
variant, which follows `prefers-color-scheme`), with text contrast of at least
4.5:1 in both. Surfaces (cards, tables, dialogs, menus, alerts, toasts) set
their own background; labels, help text, tabs and pagination links sit on the
page, so the page background must follow the colour scheme too. Core's
`render_page*` gives `<html>` a dark background (`bg-slate-950`), so put the
content on a surface:

```rust,ignore
let content = html! {
    <div class="min-h-full bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100">
        {page_body}
    </div>
};
render_page_with_head(&htmx, "en", "Orders", html! {
    <link rel="stylesheet" href={rullst::assets::url("app.css")} />
}, content)
```

**Your own layout.** `render_page*` already handles htmx's indicator styles
under the CSP. A layout of your own that loads htmx must do the same: add
`<meta name="htmx-config" content="{&quot;includeIndicatorStyles&quot;:false}" />`
(in `html!`: `content="{\"includeIndicatorStyles\":false}"`) and, if you use
the `htmx-indicator` class, its rules in your stylesheet. The button's spinner
needs neither: Tailwind classes show it.

Every component passes the checks of the "Generated starters in browsers" CI
job: a gallery page that renders all of them (copied by `ui add --all` into a
fixture application with Tailwind) is driven in Chromium, Firefox and WebKit,
in both colour schemes, with no console error or CSP violation, no axe-core
`serious` or `critical` finding, every control reachable with Tab and visibly
focused, and the keyboard paths described below.

### Button (`button`)

```rust,ignore
use crate::ui::button::{self, ButtonOptions, Kind, Size, Variant};

button::button("Save", &ButtonOptions { kind: Kind::Submit, ..Default::default() })
button::link("Docs", "/docs", Variant::Outline, Size::Sm)
// Other attributes (hx-post, name, value, form): write the element with the classes.
html! {
    <button type="button" hx-post="/archive" class={button::classes(Variant::Ghost, Size::Sm)}>
        {button::spinner()} "Archive"
    </button>
}
```

```html
<button type="submit" class="inline-flex … bg-sky-700 text-white … h-10 px-4 text-sm"><span>Save</span></button>
```

- **Options:** `variant` (`Primary`, `Secondary`, `Outline`, `Ghost`,
  `Destructive`, `Link`), `size` (`Sm`, `Md`, `Lg`, `Icon`), `kind`
  (`Button`, `Submit`, `Reset`), `id`, `disabled`, `loading`.
- **Loading state:** `loading: true` adds a spinner shown while htmx gives
  the button, or an element around it, its `htmx-request` class: the form
  sending the request by default, or the element its `hx-indicator` names.
  `hx-disabled-elt` disables the button meanwhile, so a screen reader
  announces it as unavailable and it cannot be pressed twice:
  `<form id="order" hx-post="/save" hx-indicator="#order" hx-disabled-elt="#save">`
  with `id: Some("save".into())`. With htmx 1.9.12 (applications created
  before 13.0.0-alpha.5 that still load it) name different elements in the
  two attributes: it counts requests per element for both, so an element
  named in both keeps its `htmx-request` class after the response. htmx 2
  releases both.
- **Accessibility:** a real `<button>` (or `<a>` for navigation), so Enter and
  Space work; a visible focus outline on keyboard focus. An `Icon` button
  needs an `aria-label`. The spinner is `aria-hidden`.
- **CSP:** classes only. The spinner keeps htmx's `htmx-indicator` class, so
  on `render_page*` pages htmx's own (nonce'd) indicator rules apply as well.

### Badge (`badge`)

```rust,ignore
use crate::ui::badge::{badge, Tone};
badge("Paid", Tone::Success)
```

```html
<span class="inline-flex items-center rounded-full border … text-emerald-800 …">Paid</span>
```

- **Options:** `Tone::{Neutral, Info, Success, Warning, Danger}`;
  `badge::classes(tone)` for other elements.
- **Accessibility:** the text carries the meaning; the colour only repeats it.

### Alert (`alert`)

```rust,ignore
use crate::ui::alert::{alert, AlertOptions, Tone};
alert(&AlertOptions { tone: Tone::Danger, title: Some("Not saved") },
      html! { <p>"Fix the highlighted fields."</p> })
```

```html
<div role="alert" class="flex gap-3 rounded-lg border p-4 …"><svg aria-hidden="true">…</svg><div><p class="font-semibold">Not saved</p>…</div></div>
```

- **Options:** `tone` (`Info`, `Success`, `Warning`, `Danger`), `title`.
- **Accessibility:** `Warning` and `Danger` are `role="alert"` (announced
  when they appear, also when htmx swaps them in); `Info` and `Success` are
  `role="status"`. The icon is decorative.

### Card (`card`)

```rust,ignore
use crate::ui::card::{card, CardOptions, Heading};
card(&CardOptions { title: Some("Pro plan"), description: Some("Billed monthly"),
                    id: Some("plan"), heading: Heading::H2 },
     html! { <p>"20 seats"</p> },
     Some(html! { <a href="/billing">"Change plan"</a> }))
```

```html
<section id="plan" aria-labelledby="plan-title" class="rounded-xl border …"><div …><h2 id="plan-title" …>Pro plan</h2><p …>Billed monthly</p></div><div …>…</div><div class="… border-t …">…</div></section>
```

- **Options:** `title`, `description`, `heading` (`H2`, `H3`, `H4`), `id`;
  the footer is the third argument.
- **Accessibility:** a titled card is a `<section>` named by its heading;
  choose the level that continues the page's outline, and give each card an
  `id` when a page has several (the heading's id is `<id>-title`).

### Form and fields (`form`)

```rust,ignore
use crate::ui::form::{self, Control, Field, FormOptions, InputType};

async fn page(Extension(csrf): Extension<rullst::security::CsrfToken>) -> Markup {
    form::form("/contacts", csrf.as_str(), &FormOptions::default(), html! {
        {form::field(&Field { id: "email", label: "Email",
            control: Control::Input(InputType::Email), autocomplete: Some("email"),
            required: true, error: Some("Enter an address."), ..Default::default() })}
        {form::checkbox(&Field { id: "terms", label: "I accept the terms", ..Default::default() }, false)}
        <button type="submit">"Send"</button>
    })
}
```

```html
<form method="post" action="/contacts" class="space-y-6">
  <input type="hidden" name="_token" value="…" />
  <div class="space-y-2">
    <label for="email" …>Email<span aria-hidden="true"> *</span></label>
    <input type="email" id="email" name="email" autocomplete="email" required aria-describedby="email-error" aria-invalid="true" class="… border-red-600 …" />
    <p id="email-error" …>Enter an address.</p>
  </div>
  …
</form>
```

- **`form(action, csrf_token, options, body)`:** `method` (`Post`, `Get`),
  `boost` (`hx-boost="true"`: htmx submits and swaps the body, a plain submit
  without JavaScript), `multipart` (`enctype="multipart/form-data"`), `id`.
  A `POST` form starts with the `_token` field of Rullst's double-submit CSRF
  check (first, as a multipart body must send it before any file); pass
  `CsrfToken::as_str()` from `Extension<CsrfToken>`. `GET` forms carry none.
- **`field(&Field)`:** `id`, `name` (the `id` when empty), `label`, `control`
  (`Input(InputType::{Text, Email, Password, Number, Tel, Url, Search, Date})`,
  `Textarea { rows }`, `Select(&[(value, label)])`), `value`, `placeholder`,
  `autocomplete`, `help`, `error`, `required`, `disabled`.
  `checkbox(&Field, checked)` puts the label on the right.
- **Accessibility:** every control has a `<label for>`; help text
  (`<id>-help`) and the error (`<id>-error`) are linked with
  `aria-describedby`, and an error sets `aria-invalid="true"` and a red
  border, so the message is announced with the field and not shown by colour
  alone. Required fields carry `required` and a visible `*` hidden from
  screen readers (which announce `required` themselves).
- **CSP:** classes only. WebKit 27.2, as Playwright 1.64.1 ships it, reports
  a blocked style attribute for every `<select>` under the strict
  `style-src`: it comes from the engine's own select internals (a page with
  only a `<select>` gets the same report; Chromium and Firefox report
  nothing), the select still renders and works, and the CI check records it
  as a known WebKit defect.

### Table (`table`)

```rust,ignore
use crate::ui::table::{table, Align, Column, TableOptions};
let rows = invoices.iter().map(|invoice| vec![
    html! { {invoice.number.as_str()} },
    html! { {invoice.total.to_string()} },
]).collect();
table(&TableOptions { id: "invoices", caption: "Invoices", ..Default::default() },
      &[Column::new("Number"), Column { label: "Total", align: Align::End }], rows)
```

```html
<div role="region" aria-labelledby="invoices-caption" tabindex="0" class="w-full overflow-x-auto …">
  <table …><caption id="invoices-caption" …>Invoices</caption>
    <thead …><tr><th scope="col" …>Number</th><th scope="col" class="… text-right tabular-nums">Total</th></tr></thead>
    <tbody>…</tbody></table></div>
```

- **Options:** `id` (prefix of the ids), `caption`, `show_caption` (false keeps
  it for screen readers only), `empty` (the text of the single row shown
  without rows, spanning every column); `Column { label, align }`.
- **Accessibility:** a real table with a caption and `scope="col"` headers.
  The scroll container is a named region with `tabindex="0"`, so a table wider
  than the screen can be scrolled with the arrow keys.

### Pagination (`pagination`)

```rust,ignore
use crate::ui::pagination::{pagination, PaginationOptions};
pagination(&PaginationOptions { page: 3, pages: 12, ..Default::default() },
           |page| format!("/invoices?page={page}"))
```

```html
<nav aria-label="Pagination"><ul class="flex …">
  <li><a href="/invoices?page=2" rel="prev" …>←Previous</a></li>
  <li><a href="/invoices?page=1" aria-label="Page 1" …>1</a></li> …
  <li><a href="/invoices?page=3" aria-current="page" aria-label="Page 3" …>3</a></li> …
  <li><span aria-hidden="true" …>…</span></li> …
</ul></nav>
```

- **Options:** `page` (from 1), `pages`, and the texts `label`, `previous`,
  `next`, `page_label`; nothing is rendered for one page. `window(page,
  pages)` is the pure function choosing the numbers (all up to 7, else the
  first, the last and the current page with its neighbours).
- **htmx:** plain links, so paging works without JavaScript; put
  `hx-boost="true"` on an element around it to swap the page instead.
- **Accessibility:** `aria-current="page"` on the current page; Previous and
  Next are left out on the first and last page rather than shown disabled.

### Dialog (`dialog`)

```rust,ignore
use crate::ui::button::Variant;
use crate::ui::dialog::{self, DialogOptions};
html! {
    {dialog::trigger("delete", "Delete project", Variant::Destructive)}
    {dialog::dialog(
        &DialogOptions { id: "delete", title: "Delete the project?",
                         description: Some("Its invoices are kept."), ..Default::default() },
        html! { <p>"This cannot be undone."</p> },
        Some(html! {
            {dialog::close("delete", "Cancel", Variant::Outline)}
            <form method="post" action="/projects/7/delete">
                <input type="hidden" name="_token" value={csrf.as_str()} />
                <button type="submit" class={crate::ui::button::classes(Variant::Destructive, crate::ui::button::Size::Md)}>"Delete"</button>
            </form>
        }),
    )}
}
```

```html
<button type="button" commandfor="delete" command="show-modal" aria-haspopup="dialog" …>Delete project</button>
<dialog id="delete" aria-labelledby="delete-title" aria-describedby="delete-description" class="m-auto … backdrop:bg-slate-950/60 …">
  <div …><h2 id="delete-title" …>Delete the project?</h2>
    <button type="button" commandfor="delete" command="close" aria-label="Close" …><svg aria-hidden="true">…</svg></button></div>
  <p id="delete-description" …>Its invoices are kept.</p>
  <div …>…</div><div class="… border-t …">…</div>
</dialog>
<script type="module" src="/static/ui/dialog.<hash>.js"></script>
```

- **Options:** `id`, `title`, `description`, `width` (`Sm`, `Md`, `Lg`),
  `close_label`; `trigger(id, label, variant)` and `close(id, label,
  variant)` make the buttons.
- **How it opens:** the HTML invoker commands, `commandfor` with
  `command="show-modal"` or `"close"`, built into Chrome 135, Firefox 144 and
  Safari 26.2 (Baseline since December 2025) and working without JavaScript
  there. `static/ui/dialog.js` runs the same commands in older browsers (and
  the popover commands), so the dialog does not depend on the newest engines.
- **Accessibility:** a native modal `<dialog>`: the browser makes the rest of
  the page inert, keeps focus inside, closes it on Escape and returns focus to
  the button that opened it; `dialog.js` returns focus where a browser loses
  it. It is named by its title and described by `description`. Focus starts on
  the first control (the close button in the corner) or on an element marked
  `autofocus`. A `<form method="dialog">` button also closes it.
- **CSP:** same-origin module; the backdrop and position are classes.

### Dropdown menu (`menu`)

```rust,ignore
use crate::ui::menu::{menu, Item, MenuOptions};
html! {
    {menu(&MenuOptions { id: "account", label: "Account", ..Default::default() }, &[
        Item::Link { label: "Profile", href: "/profile" },
        Item::Separator,
        Item::Submit { label: "Sign out", form: "logout" },
    ])}
    <form id="logout" method="post" action="/logout">
        <input type="hidden" name="_token" value={csrf.as_str()} />
    </form>
}
```

```html
<button type="button" id="account-button" popovertarget="account" aria-haspopup="menu" aria-controls="account" aria-expanded="false" …>Account<svg aria-hidden="true">…</svg></button>
<div id="account" popover role="menu" aria-labelledby="account-button" class="m-0 min-w-48 …">
  <a role="menuitem" tabindex="-1" href="/profile" …>Profile</a>
  <div role="separator" …></div>
  <button role="menuitem" tabindex="-1" type="submit" form="logout" …>Sign out</button>
</div>
<script type="module" src="/static/ui/menu.<hash>.js"></script>
```

- **Options:** `id` (the button is `<id>-button`), `label`, `variant`;
  items are `Link { label, href }`, `Submit { label, form }` (submits the
  form with that `id`, which carries the CSRF token; an htmx form works too)
  and `Separator`.
- **How it opens:** the `popover` attribute with `popovertarget`, in every
  current browser: the button toggles it without script, and the browser
  closes it on Escape or a click elsewhere. `static/ui/menu.js` places it
  under the button (above when there is no room), keeps `aria-expanded` in
  step and adds the keyboard of the ARIA menu button pattern.
- **Keyboard:** Enter or Space on the button opens the menu on its first
  item, ArrowDown on the first and ArrowUp on the last; ArrowDown and ArrowUp
  move (wrapping), Home and End jump; Enter chooses; Escape closes and
  returns focus to the button; Tab closes it and moves on.
- **CSP:** the module positions the menu through the CSSOM
  (`element.style.top`), which the policy allows; no `style` attribute.

### Tabs (`tabs`)

```rust,ignore
use crate::ui::tabs::{tabs, Tab, TabsOptions};
tabs(&TabsOptions { id: "settings", label: "Settings", selected: 0 }, vec![
    Tab { key: "profile", label: "Profile", panel: html! { <p>"…"</p> } },
    Tab { key: "billing", label: "Billing",
          panel: html! { <div hx-get="/settings/billing" hx-trigger="load"></div> } },
])
```

```html
<div class="space-y-4">
  <div role="tablist" aria-label="Settings" …>
    <button type="button" role="tab" id="settings-tab-profile" aria-controls="settings-panel-profile" aria-selected="true" tabindex="0" …>Profile</button>
    <button type="button" role="tab" id="settings-tab-billing" aria-controls="settings-panel-billing" aria-selected="false" tabindex="-1" …>Billing</button>
  </div>
  <div role="tabpanel" id="settings-panel-profile" aria-labelledby="settings-tab-profile" tabindex="0" …>…</div>
  <div role="tabpanel" id="settings-panel-billing" aria-labelledby="settings-tab-billing" tabindex="0" hidden …>…</div>
</div>
<script type="module" src="/static/ui/tabs.<hash>.js"></script>
```

- **Options:** `id` (prefix of the ids), `label` (names the tab list),
  `selected` (index rendered selected); each `Tab { key, label, panel }`.
- **Accessibility:** the ARIA tabs pattern with automatic activation:
  ArrowRight and ArrowLeft move and select (wrapping; reversed for
  right-to-left, ArrowDown/ArrowUp with `aria-orientation="vertical"`), Home
  and End jump; only the selected tab is in the Tab order, and Tab then moves
  into its panel. The server renders the selected state, so the first paint is
  right. Without JavaScript only the selected panel shows: when content must
  work without it, make each tab a link (`?tab=billing`) and render the
  selected one on the server.

### Toasts (`toast`)

```rust,ignore
use crate::ui::toast::{self, RegionOptions, Tone};
use rullst::htmx::HtmxResponse;

// Once per page, in the layout:
html! { {page_body} {toast::region(&RegionOptions::default(), &[])} }

// A handler:
async fn save() -> HtmxResponse {
    HtmxResponse::new(fragment.into_string())
        .trigger(toast::trigger("Saved", Tone::Success))
}
```

```html
<section aria-label="Notifications" class="pointer-events-none fixed … bottom-4 z-50 …">
  <div data-ui-toasts aria-live="polite" aria-relevant="additions" class="flex flex-col gap-2"></div>
  <template data-ui-toast-template="info">…</template> … (one per tone)
</section>
<script type="module" src="/static/ui/toast.<hash>.js"></script>
```

- **Server side:** `trigger(message, tone)` is the `HX-Trigger` header value
  `{"ui:toast":{"message":"Saved","tone":"success"}}`, with characters outside
  ASCII written as `\u` escapes so the header stays valid. htmx fires the
  `ui:toast` event when the response arrives; `{"ui:toast":"Saved"}` works too.
  A toast for the next full page (after a redirect) goes in `region`'s second
  argument. Any script can show one with
  `document.dispatchEvent(new CustomEvent("ui:toast", { detail: { message, tone } }))`.
- **Options:** `RegionOptions { label, dismiss }` (the texts), `Tone::{Info,
  Success, Warning, Error}`.
- **Accessibility:** toasts are added to an `aria-live="polite"` list, so
  screen readers announce them without moving focus; each has a close button.
  Info and success toasts leave after six seconds unless the pointer or the
  focus is on them; warnings and errors stay until dismissed.
- **CSP:** `toast.js` clones the region's `<template>` for the tone (so its
  Tailwind classes live in Rust source) and sets the message with
  `textContent`, never as HTML.

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
