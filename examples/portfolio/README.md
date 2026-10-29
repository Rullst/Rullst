# Portfolio example

A developer portfolio: a profile, featured projects, experience and skills read
from the database and edited in Nexus.

This is an example to copy from, not a supported starter.

## What it shows

- `/`: the portfolio page, rendered with `html!` under the production Content
  Security Policy (nonce-based styles, no remote fonts or images). Values from
  the CMS are escaped, and only `http(s)` URLs become links.
- `/nexus`: loopback-only in debug builds. In release builds it needs
  `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` (at least 16 characters);
  without them the site starts without `/nexus`.
- `/health` and `/ready`.

The migration seeds a sample profile, projects, experience and skills; edit
them in Nexus. The page reads them with Active Record queries and answers a
failed query with an error page instead of an empty portfolio.

## Run it

From a checkout of this repository:

```bash
cd examples/portfolio
cp .env.example .env   # then set APP_KEY to 32 random characters, e.g. `openssl rand -hex 16`
cargo run -p rullst-portfolio-example -- db:migrate
cargo run -p rullst-portfolio-example
```

Run it from this directory so the server finds `.env`, `Rullst.toml` and
`static/`. The SQLite database is `db.sqlite` here. Open
<http://127.0.0.1:3000>; in debug builds Rullst Studio runs on
<http://127.0.0.1:5555>.

## Tests

```bash
cargo test -p rullst-portfolio-example
```

`src/security_tests.rs` checks the production security baseline (secure headers,
the WAF and CSRF). `src/pages/home.rs` tests that CMS values render as text and
that only `http(s)` URLs become links, and `src/pages/home_mobile_tests.rs`
renders long content and, with `RULLST_UI_BROWSER_TESTS=1` and
`RULLST_UI_BROWSER_SCRIPT=.github/mobile-ui-browser-smoke.mjs` (Linux CI),
checks the page in a real mobile Chromium viewport.

They run offline (no database or network) as part of the workspace test suite
in CI.

## Where it comes from

Until 13.0.0-alpha.4, `cargo rullst new --blueprint portfolio` generated this
application. The CLI now offers only the Blank, API and SaaS starters, and this
code lives here as a complete application. It was generated with the
13.0.0-alpha.3 development CLI of this repository (commit `1126f6558`):

```bash
cargo rullst new portfolio --default --blueprint portfolio --database sqlite \
  --skip-initial-migration --vcs none
```

and then adapted to the workspace: the package is `rullst-portfolio-example`
(`publish = false`) with workspace dependencies instead of crates.io versions,
`src/main.rs` was split into a library (`src/lib.rs`: modules, `router()` and
the tests) and a thin binary, the code was formatted with `rustfmt`, and the
host-local or regenerable files (`.env`, `.cargo/config.toml`, `AGENTS.md`,
`.llms.txt`, `.rullst/`) were left out. The mobile browser test came from the
CLI's generated-project suite.

Applications generated from the old blueprint keep working: their code is
theirs.

## Copying it

Copy the directory into your own project and replace the `workspace = true`
dependencies with the published crates, for example:

```toml
rullst = { version = "=13.0.0-alpha.4", features = ["orm", "strict-sqlite", "studio", "nexus"] }
```

and the same version for `rullst-orm`. It is tested in this
repository, but there is no generator or upgrade path behind it. To start a new
application, use `cargo rullst new` (Blank, API or SaaS).
