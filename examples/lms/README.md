# LMS example

A small learning platform: a public course catalog with categories, courses,
modules and lessons; an accessible lesson player with captions; enrollment;
owner-bound lesson progress; sign-up, sign-in, password change and reset, "sign
out everywhere"; and a Nexus admin for signed-in users whose `is_admin` is true.

This is an example to copy from, not a supported starter.

## What it shows

- `/` and `/courses/{id}`: the public catalog (bounded, parameterized queries).
- `/register`, `/login`, `/logout`, `/logout/everywhere`, `/password/forgot`,
  `/password/reset`, `/password/change`: Argon2id accounts with encrypted session
  cookies revoked by a per-account session epoch; credential submissions are
  rate limited per client.
- `/dashboard`, `/courses/{id}/enroll`, `/lessons/{id}/play`,
  `/lessons/{id}/progress`: signed-in learning. A lesson opens only for the
  learner's active enrollment, and progress is recorded with idempotency keys
  scoped to the learner.
- `/nexus`: loopback-only in debug builds; in release builds, signed-in users
  whose `is_admin` is true (`src/middlewares/nexus_access.rs`). Grant the first
  one with `UPDATE users SET is_admin = TRUE WHERE email = 'you@example.com';`.
- `/health` and `/ready`.

The migrations seed a sample category, course, modules and lessons whose
captions are in `static/media/`. Password reset links are sent through the
Mail facade (`MAIL_FROM`, `MAIL_DRIVER`; see `.env.example`).

## Run it

From a checkout of this repository:

```bash
cd examples/lms
cp .env.example .env   # then set APP_KEY to 32 random characters, e.g. `openssl rand -hex 16`
cargo run -p rullst-lms-example -- db:migrate
cargo run -p rullst-lms-example
```

Run it from this directory so the server finds `.env`, `Rullst.toml` and
`static/`. The SQLite database is `db.sqlite` here. Open
<http://127.0.0.1:3000>; in debug builds Rullst Studio runs on
<http://127.0.0.1:5555>.

## Tests

```bash
cargo test -p rullst-lms-example
```

`src/security_tests.rs` sends requests through the router wrapped in the
production security baseline: secure headers, the WAF, CSRF, the sign-in rate
limit and the refusal of another learner's lessons (the evidence for
threat-model case `ACADEMY-02`). Unit tests cover the owner boundary of the
learning service, progress idempotency keys, catalog bounds, form length limits
and the Nexus authorizer.

They run offline (no database or network) as part of the workspace test suite
in CI.

## Where it comes from

Until 13.0.0-alpha.4, `cargo rullst new --blueprint lms` generated this
application. The CLI now offers only the Blank, API and SaaS starters, and this
code lives here as a complete application. It was generated with the
13.0.0-alpha.3 development CLI of this repository (commit `1126f6558`):

```bash
cargo rullst new lms --default --blueprint lms --database sqlite \
  --skip-initial-migration --vcs none
```

and then adapted to the workspace: the package is `rullst-lms-example`
(`publish = false`) with workspace dependencies instead of crates.io versions,
`src/main.rs` was split into a library (`src/lib.rs`: modules, `router()` and
the tests) and a thin binary, the code was formatted with `rustfmt`, and the
host-local or regenerable files (`.env`, `.cargo/config.toml`, `AGENTS.md`,
`.llms.txt`, `.rullst/`) were left out.

Applications generated from the old blueprint keep working: their code is
theirs.

## Copying it

Copy the directory into your own project and replace the `workspace = true`
dependencies with the published crates, for example:

```toml
rullst = { version = "=13.0.0-alpha.4", features = ["orm", "strict-sqlite", "studio", "nexus", "auth", "mail"] }
```

and the same version for `rullst-orm`, `rullst-auth` and `rullst-security`. It is tested in this
repository, but there is no generator or upgrade path behind it. To start a new
application, use `cargo rullst new` (Blank, API or SaaS).
