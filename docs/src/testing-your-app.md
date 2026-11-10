# Testing your application

Three applications built on Rullst wrote their own testing layers by hand
before 13.0.0-alpha.5. Between them they needed the same list, and their
browser and end-to-end layers caught defects that Rust unit tests did not:

- a static `html!` attribute containing `\"`, which ended the attribute early
  and broke the rest of the tag;
- `name={option}` writing `disabled=""` for `None`, which disabled a button;
- content escaped twice, so the page showed `&lt;span` as text;
- an inline `style` attribute that the strict Content Security Policy blocked;
- a bare fragment served when htmx restored a page from its history.

Since 13.0.0-alpha.5, `cargo rullst new` generates the first layers of that
list for every starter (Blank, API and SaaS), and `cargo rullst add
browser-tests` adds the browser layer. The rest stays the application's job;
this page says which is which.

| Layer | Generated | Where |
| :--- | :--- | :--- |
| Unit tests of business rules | No: they are yours | `#[cfg(test)]` modules, `tests/` |
| Clippy with `-D warnings`, formatting, tests on every push and pull request | Yes | `.github/workflows/ci.yml` |
| Security headers, WAF and CSRF behind the production baseline | Yes | `src/security_tests.rs` |
| Every page: status, security headers, HTML hygiene | Yes | `PAGES` in `src/security_tests.rs` |
| Daily `cargo audit` and a secret scan (TruffleHog) | Yes | `.github/workflows/ci.yml` |
| Monthly grouped dependency updates | Yes | `.github/dependabot.yml` |
| Every page in Chromium, Firefox and WebKit with axe-core, both colour schemes | With `cargo rullst add browser-tests` | `tests/browser/` and the `browser-tests.yml` workflow |
| Routes with `{id}` classified with `// rullst-access:` | Checked by `cargo rullst audit --idor` | your CI, see below |
| A route × role permission table | No | see below |
| Restore a backup, test migrations on a copy, monitor the site, OWASP ZAP, published-figure freshness | No | see below |

## What `cargo rullst new` generates

### `src/security_tests.rs`

The starter's router is tested offline (no database, no network) wrapped in
the baseline that `Server` adds in staging and production: secure headers,
WAF and CSRF. Run it with `cargo test`. Since 13.0.0-alpha.5 it also has a
smoke test of every page:

```rust,ignore
/// Every page a signed-out visitor can open without parameters, with the
/// status it answers. ...
const PAGES: &[(&str, u16)] = &[
    ("/", 200),
    ("/pricing", 200),
    ("/login", 200),
    ("/dashboard", 303),
    ("/health", 200),
];
```

For each entry, `every_page_answers_with_safe_headers_and_sound_html` sends a
GET with [`TestApp`](crates/core.md) and checks:

- the status (pages behind the sign-in answer `303` to `/login`, so the list
  also proves they stay protected);
- `TestResponse::assert_security_headers`: a `Content-Security-Policy`,
  `X-Content-Type-Options: nosniff` and frame protection (`X-Frame-Options`
  `DENY`/`SAMEORIGIN` or a CSP `frame-ancestors` directive);
- `TestResponse::assert_html_hygiene` on HTML responses (JSON passes
  unchecked), which reports:
  - an escaped element tag or comment in text (`&lt;span class`,
    `&lt;/div&gt;`) or an entity escaped again (`&amp;lt;`) anywhere;
  - a broken attribute: a quote in a name, a quoted value followed directly by
    more characters (the `\"` defect above), a quote, `<`, `=` or backtick in
    an unquoted value, `name=` without a value, or a tag, comment or
    `<script>`/`<style>` that never ends;
  - a boolean attribute set to `"false"` (`disabled="false"` disables:
    write `disabled?={flag}`);
  - an inline `<script>` (without `src`) or `<style>` without the response's
    CSP nonce;
  - a `style` or `on*` attribute the response's policy blocks (nonces do not
    apply to attributes).

**Add each new page to `PAGES`.** Use the path a signed-out visitor opens and
the status they receive; pages that need a signed-in user can be checked in
your own tests by sending the session cookie with `TestApp::get(path)
.header("cookie", …)`. Routes with parameters (`/posts/{id}`) are not listed:
test them with fixtures in your own tests.

The checks are a tokenizer, not an HTML parser or a browser. Text inside
`code`, `pre`, `kbd`, `samp` and `textarea` may show escaped markup; anywhere
else a page that shows an HTML tag on purpose (`<p>Use &lt;b&gt; for
bold</p>`) is reported, so keep such pages out of `PAGES` or check them
yourself. The same functions are available to any test:

```rust
use rullst::testing::{HtmlIssueKind, html_issues};

let csp = "default-src 'self'; script-src 'self' 'nonce-abc'";
let issues = html_issues("<p>&lt;span&gt;</p><script>run()</script>", Some(csp));
assert_eq!(issues[0].kind(), HtmlIssueKind::DoubleEscaped);
assert_eq!(issues[1].kind(), HtmlIssueKind::MissingNonce);
```

`rullst::testing::assert_html_snapshot!` compares a page with a reviewed
snapshot when you want to pin its whole markup.

### `.github/workflows/ci.yml` and `.github/dependabot.yml`

`new` writes them unless you pass `--no-ci`. They belong to the application:
edit them freely.

- **Formatting:** `cargo fmt --all -- --check`. A new project is not
  rustfmt-formatted yet, so run `cargo fmt --all` once and commit before the
  first push.
- **Clippy and tests:** `cargo clippy --all-targets -- -D warnings` and
  `cargo test --all-targets`, with a dependency cache.
- **Dependency advisories:** `cargo audit` on every push and pull request and
  daily at 06:17 UTC, so a new RustSec advisory for a locked crate fails the
  scheduled run. It reads `Cargo.lock`: commit it.
- **Secret scan:** TruffleHog, verified findings only, over the commits of the
  push or pull request.

Every action is pinned to a full commit SHA with its version in a comment,
the same pins Rullst's own workflows use, and the token is read-only
(`permissions: contents: read`). Dependabot opens one grouped pull request a
month for Cargo and one for GitHub Actions. Enable Dependabot security updates
in the repository settings to receive advisory fixes at once.

The workflow sits at the root of the new project. When the project lives in a
subdirectory of a larger repository, move the file to that repository's
`.github/workflows/` and set `working-directory`.

## Browser tests

```bash
cargo rullst add browser-tests
```

writes:

| File | Purpose |
| :--- | :--- |
| `tests/browser/package.json`, `package-lock.json` | Playwright 1.64.1 and axe-core 4.14.0, pinned with `sha512` integrity |
| `tests/browser/pages.mjs` | Opens each HTML page of `PAGES` in Chromium, Firefox and WebKit |
| `tests/browser/.gitignore` | Keeps `node_modules/` and `results/` out of Git |
| `browser-tests.yml` (in `.github/workflows/`) | Builds and starts the application, then runs `pages.mjs` |

`pages.mjs` reads the `PAGES` list from `src/security_tests.rs`, so one list
drives both layers. It fails on a console error, an uncaught page error, a
failed same-origin request, a Content-Security-Policy violation (the
application's own policy applies) or an axe-core violation of `serious` or
`critical` impact; `moderate` and `minor` findings are printed as notes. When
a stylesheet has a `prefers-color-scheme: dark` rule, axe-core runs again in
the dark scheme. Pages that redirect (`303`) are checked where they land.

Locally:

```bash
npm ci --prefix tests/browser
node tests/browser/node_modules/playwright/cli.js install --with-deps chromium firefox webkit
cargo run                      # in another terminal
node tests/browser/pages.mjs   # --base-url, --browsers chromium,firefox,webkit
```

The workflow copies `.env.example` to `.env`, sets a random `APP_KEY`, runs
`db:migrate` when the project has `src/migrations/`, starts the application on
port 3000 and waits for `/health`. Running the command again changes nothing;
a file you changed is kept. A project generated before 13.0.0-alpha.5 has no
`PAGES` list: the script then visits `/` only until you add one.

## Access classification of `{id}` routes

`cargo rullst audit --idor` lists every route with a path parameter that has
no adjacent `// rullst-access: public|owner|role|admin — reason` comment, or
whose classification has no matching guard in the crate, and exits non-zero.
Add it to your CI once the CLI is installed there (`cargo install cargo-rullst
--version <the version you generated with> --locked`). The audit points at
the routes; a person still has to read each handler and decide whether it
really checks ownership. Rullst does not generate this check yet: the audit
reports findings as text and its JSON summary carries counts, not a
machine-readable route list a generated test could read.

## What stays the application's job

- **Business rules:** unit tests next to the code, and integration tests that
  boot the application with a real database and walk the main screens
  (`#[rullst_orm::test]` runs a test inside an ORM transaction that is always
  rolled back, on `DATABASE_URL` or an in-memory SQLite database).
- **Roles:** with several user roles, a table of route × role and the status
  each role must receive, as a test that signs in as each role.
- **Production:** restore a backup now and then, run migrations on a copy of
  production data before releasing them, and monitor the site (see the
  [production monitoring endpoint](production-monitoring.md)).
- **Scans:** a weekly OWASP ZAP baseline scan of a deployed or locally started
  copy; Rullst's own `.github/workflows/dast-zap.yml` is a model.
- **Published figures:** when you publish numbers such as page weight or
  response times, re-measure them in CI and fail when the published figure is
  stale.
- **Hygiene:** coverage and badges, `actionlint` for the workflows and
  `cargo machete` for unused dependencies.

The htmx history-restore defect in the list above is not checked by the
generated smoke test yet: it needs the Core fix that renders the whole page
for `HX-History-Restore-Request`, after which a check can be added to `PAGES`
testing.
