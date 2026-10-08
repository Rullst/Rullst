# Zero to a complete app

**What you will build:** *Notes*, a small web app where people register, sign
in and keep private notes. Along the way you will add a model and its
migration, CRUD pages with HTMX and form validation, routes that only a note's
owner can open, tests, a security report, a development loop and a production
build. Plan for about an hour, most of it waiting for the first build.

This page is one path from an empty folder to a working app. Each step links
to the tutorial that explains it in depth.

> **How this page was recorded.** Every command after the installation, and
> every output, comes from a run on 8 October 2026 with the 13.0 development
> CLI built from `main` (`13.0.0-alpha.1`), SQLite and Linux. Output is shortened with `…`, and your
> timestamps will differ. Steps marked **New in 13.0** need that release; each
> says what to do on 12.x.

## 1. Install Rust and the CLI

Install Rust from [rustup.rs](https://rustup.rs/), then the stable CLI:

```bash
cargo install cargo-rullst --version '^12' --locked
```

To follow the **New in 13.0** steps before that release is published, install
the CLI from a `main` checkout instead, as described in
[Getting Started](../1-getting-started.md#1-installation).

## 2. Create the app from the SaaS starter

The SaaS starter already has registration, sign-in, a session middleware,
CSRF protection, security headers and Stripe billing that runs offline until
you add real keys, so you can focus on your own feature. Create it with SQLite, which needs no database
server:

```bash
cargo rullst new notes --default --blueprint saas --database sqlite
cd notes
```

The first build compiles every dependency and takes several minutes (3 min 48 s
on the recording machine). It ends with:

```text
📦 Bootstrapping Database...
  ✅ Database tables created successfully.
✨ Project 'notes' created successfully!
  Application profile: Zero-Bundle HTMX (html! SSR)
  ORM profile: Active Record

Next steps
  1  cd notes
  2  cargo rullst dev             builds, applies migrations and serves with live reload
  3  open http://127.0.0.1:3000   your generated welcome page
  Prefer a live dashboard? cargo rullst dash · new to Rullst? cargo rullst tour
```

The 12.x CLI prints a shorter summary; the generated project works the same
way. Put the project under version control now. The security report in step 7
reads the files Git tracks, and the generated `.gitignore` already leaves out
`.env` and the SQLite database:

```bash
git init
git add -A
git commit -m "Generated SaaS starter"
```

## 3. Add a model with a migration

```bash
cargo rullst make:model Note --migration
```

```text
🛠️ Generating Rullst model: Note...
✨ Model 'Note' successfully created at 'src/models/note.rs'!
✨ Rust migration successfully created at 'src/migrations/m20261008071503_create_notes.rs'!
…
Next steps
  → cargo rullst db:migrate             apply the new migration
  → cargo rullst make:controller Note   serve the model over HTTP
```

Give the model its fields in `src/models/note.rs`. `user_id` records who wrote
the note; it always comes from the session, never from the form:

```rust,ignore
{{#include zero-to-complete-app/note.rs}}
```

Add the same columns to the generated migration,
`src/migrations/m<timestamp>_create_notes.rs`:

```rust,ignore
{{#include zero-to-complete-app/create_notes.rs}}
```

Apply it:

```bash
cargo rullst db:migrate
```

```text
⏳ Running 'cargo run -- db:migrate'...
…
Migrating: m20261008071503_create_notes
Migrated:  m20261008071503_create_notes
```

More on models and schema changes:
[Active Record CRUD](03-active-record-crud.md) and
[migrations](05-migrations-and-seeds.md).

## 4. CRUD pages with HTMX and validation

Generate the controller, and add the `validator` crate, whose derive macro
checks form fields. Use the version Rullst uses, 0.21:

```bash
cargo rullst make:controller Notes
cargo add validator@0.21 --features derive
```

Replace the generated placeholder in `src/controllers/notes_controller.rs`
with the complete controller:

```rust,ignore
{{#include zero-to-complete-app/notes_controller.rs}}
```

What it does:

- `index` lists only the signed-in user's notes and renders a form. `store`,
  `update` and `destroy` save, change and delete notes; `show` opens one.
- `ValidatedForm<NoteForm>` rejects an empty or overlong title before the
  handler runs. For an HTMX request it answers with an error fragment, which
  `hx-target="#errors"` places under the form; the REST status travels in
  the `X-Rullst-Validation-Status` header.
- After a successful write, HTMX receives an `HX-Redirect` back to the list,
  and a plain form post a `303` redirect, so the forms work without
  JavaScript too.
- `html!` escapes the values it interpolates: a note titled `Buy <milk>` is
  rendered as `Buy &lt;milk&gt;`.

Now mount the routes in `src/main.rs`. Add them **before** the
`csrf_middleware` layer, so every form post needs the CSRF token the pages
include, and wrap each one in the starter's `auth_middleware`:

```rust,ignore
{{#include zero-to-complete-app/main.rs:43:54}}
```

The whole file is in [`zero-to-complete-app/main.rs`](zero-to-complete-app/main.rs).
More on [HTMX rendering](06-htmx-zero-bundle.md),
[forms and validation](07-forms-and-validation.md) and
[routing](08-routing-and-middlewares.md).

## 5. Sign-in and owner-only routes

Sign-in comes from the starter: `/register` and `/login` set an encrypted
session cookie, and `auth_middleware` turns it into the user id that the
handlers read as `Extension<i32>`. Anyone else is redirected to `/login`.

Being signed in is not enough to open a note, though: `/notes/{id}` must
check that the note belongs to that user. Otherwise anyone could read another
user's note by changing the number in the URL (an IDOR). `find_owned` loads
the note and calls `RbacGuard::authorize_owner_or_role`; another user's note
answers `404`, so it does not even reveal that the note exists.

The `// rullst-access: owner — reason` comment above each route with an `{id}`
records that decision. `cargo rullst audit --idor` and the security report in
step 7 fail when a route with a path parameter has no such comment, or when a
non-public classification has no matching guard in the code.

With two accounts, Alice and Bob, the recorded run observed:

| Request | Result |
| --- | --- |
| `GET /notes` without signing in | `303` to `/login` |
| Alice: `GET /notes/1` (her note) | `200` |
| Bob: `GET /notes/1` | `404` |
| Bob: `POST /notes/1/delete` | `404`, and the note is kept |
| `POST /notes` without the CSRF token | `403` |
| Alice: create with an empty title (HTMX) | `200` with `X-Rullst-Validation-Status: 422` and the message *Give the note a title (1 to 120 characters).* |

More on [authentication](11-authentication-system.md) and
[ownership, RBAC and IDOR](13-rbac-authorization.md).

## 6. Run the tests

The controller ends with two unit tests: one proves that only the owner passes
`authorize`, the other that an empty title fails validation. Neither needs a
database. Run them with the starter's own tests:

```bash
cargo test
```

```text
running 4 tests
test controllers::auth_controller::tests::registration_timestamps_use_the_current_timestamp_text ... ok
test controllers::notes_controller::tests::an_empty_title_is_rejected ... ok
test controllers::auth_controller::tests::length_limits_count_what_the_form_counts ... ok
test controllers::notes_controller::tests::only_the_owner_may_open_a_note ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## 7. Check the app with the security report (New in 13.0)

```bash
cargo rullst audit --report
```

```text
Rullst security report (OWASP ASVS 5.0.0 Level 1 mapping)
Evidence for a reviewer; it does not replace a manual review or a penetration test.
Security checks
  ✓ Security headers and CSP — no findings
  ✓ CSRF — no findings
  ✓ Session and auth cookies — no findings
  ✓ Rate limiting on authentication routes — no findings
  ✓ Committed secrets — no findings
  ✓ Vulnerable dependencies — no findings
  ✓ IDOR / BOLA route classification — no findings
Personal data
  ! Inventory — 2 field(s) listed, 2 need review (name heuristic)
Accessibility checks
  ✓ Images have a text alternative — no findings
  ✓ Form controls have a label — no findings
  ✓ Pages declare their language — no findings
NOT EVALUATED: 63 of 70 ASVS Level 1 requirements are outside these static checks.
Report written to SECURITY_REPORT.md
```

`SECURITY_REPORT.md` lists the two fields to review: the `email` columns of
the starter's `User` and `BillingCustomer` models. The vulnerable-dependency
check needs `cargo-audit` installed. To see the IDOR check work, delete one
`rullst-access` comment and run `cargo rullst audit --idor`: it fails and
names the route.

**On 12.x**, run `cargo rullst audit --idor` for the route check, and
`cargo rullst audit --compliance` for an evidence report. See
[the security report guide](../security-report.md).

## 8. Develop with live reload

```bash
cargo rullst dev
```

```text
Tip: run `cargo rullst dash` for live requests/s, latency, errors and database metrics.
Building the application...
Running initial db:migrate...
Nothing to migrate.
Auto-reload: watching source, assets and configuration; successful builds restart the application.
📊 Rullst Studio running on http://127.0.0.1:5555
🚀 SaaS server starting on port 3000...
…
Rullst framework serving on http://127.0.0.1:3000
```

Open `http://127.0.0.1:3000/register`, create an account, then go to
`http://127.0.0.1:3000/notes`. Saving a source file rebuilds and restarts the
app; a failed build keeps the previous one running. `cargo rullst dash` runs
the same loop inside a terminal dashboard with live requests, latency, errors
and database metrics. Stop either with `Ctrl+C`. See
[supervised auto-reload](51-authenticated-hot-reload.md) and the
[`dash` reference](../cli_reference.md#cargo-rullst-dash).

## 9. Build for production and deploy

```bash
cargo rullst build
```

```text
🚀 Starting Rullst production build pipeline (Release Mode: true)...
⚙️ Executing cargo build --release...
    Finished `release` profile [optimized] target(s) in 7m 14s
📦 Pre-compressing static assets in static/ directory...
…
✨ Pre-compression finished: processed 2 files, generated 2 .br files and 2 .zst files.
🎉 Rullst production build completed successfully!
```

The release binary is `target/release/notes` (25 MiB in the recorded run).
The build also writes Brotli and Zstandard copies of the files in `static/`,
which the server prefers; run it again after changing an asset. Started with
the development `.env`, the binary answered `200` on `/health` and redirected
`/notes` to `/login`.

Before it serves real users:

- create a production environment file with `RULLST_ENV=production`, a new
  `APP_KEY` and the billing settings described in the generated `BILLING.md`;
  never reuse the development `.env`;
- **New in 13.0:** check that file with
  `cargo rullst deploy:doctor --env-file .env.production`. Run against the
  development `.env`, it reports `[FAIL] environment_target: Set RULLST_ENV
  to the requested --target (production or staging).` On 12.x, review the
  [deployment preparation checklist](31-end-to-end-saas-aws-gcp.md) by hand;
- choose a platform: [guided PaaS deployment](26-one-click-paas-deploy.md)
  (`cargo rullst deploy` writes Fly.io, Railway, Render or VPS files),
  [Kubernetes](25-kubernetes-deployment.md), or a container from
  `cargo rullst dockerize`. Run migrations once per release, before starting
  the new version.

## Where next

- Read the generated `AGENTS.md` before asking an AI assistant to change the
  app, and see [the CLI reference](../cli_reference.md) for every command.
- Add [background jobs](20-background-jobs-queues.md),
  [email](../crates/mail.md) or [real billing](19-saas-billing-capital.md).
- Before launch, read the [security architecture](../security-architecture.md)
  and check each crate's [maturity tier](../maturity.md).
