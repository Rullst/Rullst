# ERP example

A small inventory back office: products, stock movements and orders, with a
dashboard of totals and recent orders, and Nexus for the records.

This is an example to copy from, not a supported starter.

## What it shows

- `/`: the dashboard (totals over every row, bounded and paged lists).
- `POST /products`, `POST /products/{id}/add-stock`, `POST /orders`: CSRF-protected
  forms. An order reserves stock atomically and reports failures instead of
  overselling.
- `/nexus`: the product and order records.
- `/health` and `/ready`, outside the administrator policy.

The dashboard, the forms and Nexus share one administrator policy
(`NexusAuthPolicy::local_development_or_basic_from_env`): loopback-only in debug
builds, Basic Auth over verified TLS in release builds with
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` (at least 16 characters).
Without them a release build serves only `/health` and `/ready`.

The migration seeds three sample products and two orders.

## Run it

From a checkout of this repository:

```bash
cd examples/erp
cp .env.example .env   # then set APP_KEY to 32 random characters, e.g. `openssl rand -hex 16`
cargo run -p rullst-erp-example -- db:migrate
cargo run -p rullst-erp-example
```

Run it from this directory so the server finds `.env`, `Rullst.toml` and
`static/`. The SQLite database is `db.sqlite` here. Open
<http://127.0.0.1:3000>; in debug builds Rullst Studio runs on
<http://127.0.0.1:5555>.

## Tests

```bash
cargo test -p rullst-erp-example
```

`src/security_tests.rs` checks the production security baseline (secure headers,
the WAF and CSRF) and that the dashboard refuses a non-loopback client. A unit
test checks that text limits count characters as the form does.

They run offline (no database or network) as part of the workspace test suite
in CI.

## Where it comes from

Until 13.0.0-alpha.4, `cargo rullst new --blueprint erp` generated this
application. The CLI now offers only the Blank, API and SaaS starters, and this
code lives here as a complete application. It was generated with the
13.0.0-alpha.3 development CLI of this repository (commit `1126f6558`):

```bash
cargo rullst new erp --default --blueprint erp --database sqlite \
  --skip-initial-migration --vcs none
```

and then adapted to the workspace: the package is `rullst-erp-example`
(`publish = false`) with workspace dependencies instead of crates.io versions,
`src/main.rs` was split into a library (`src/lib.rs`: modules, `router()` and
the tests) and a thin binary, the code was formatted with `rustfmt`, and the
host-local or regenerable files (`.env`, `.cargo/config.toml`, `AGENTS.md`,
`.llms.txt`, `.rullst/`) were left out. The test that the dashboard is not
public came from the old hot-reload variant of the blueprint.

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
