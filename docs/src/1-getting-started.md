# Getting Started

Welcome to the **Rullst** Getting Started guide!

**Your goal:** install the matching v13 CLI, generate a small application,
open it locally and make your first change. Prefer to write the first route
yourself? Use [Zero to Hello Rullst](tutorials/01-hello-world.md).

Rullst is a strictly typed Rust framework suite for full-stack applications,
designed around explicit APIs, measurable performance, and defense-in-depth
defaults.

## 1. Installation

First, ensure you have Rust installed. The official and recommended way is to visit [rustup.rs](https://rustup.rs/).

**For macOS and Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**For Windows:**
Download and run `rustup-init.exe` from the website.

Next, install the **Rullst CLI** from the same release train as the
framework. This book is built from `main`, where v13 is in development. The
v12 line reached end of life on 16 October 2026 (12.3.0 was its last release,
and a new 12.x project no longer resolves its dependencies after crates.io
yanked the `zip` releases its Studio needs), so install the v13 prerelease. Name its exact version: Cargo never selects a
prerelease on its own, so neither a plain `cargo install cargo-rullst` nor
`^13` installs it:

<!-- first-hour: install -->
```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
```

Pin the framework the same way in `Cargo.toml` (`"=13.0.0-alpha.4"`); the
[v13 migration guide](migration-v13.md) explains why and what changed.

Source reviewers who intentionally need checkout-local changes can instead
clone this repository and install the CLI from that exact checkout. On `main`
this installs the v13 development CLI:

```bash
git clone --branch main https://github.com/Rullst/Rullst.git
cd Rullst
cargo install --locked --path cargo-rullst
cd ..
```

When installed from source, the CLI reuses the exact checkout
from which it was compiled, even when project creation is invoked from another
directory, provided that checkout has not been moved or deleted. Generated
manifests therefore contain absolute path dependencies and are not portable.
Running from the repository root remains an explicit fallback. Prefer the
published prerelease and its matching registry packages for ordinary
applications.

Cargo installs the `cargo-rullst` and `rullst` executables into
`$CARGO_HOME/bin` (`~/.cargo/bin` by default). To install elsewhere, set
`CARGO_INSTALL_ROOT` or pass `--root <dir>`. To remove the CLI, run:

```bash
cargo uninstall cargo-rullst
```

Add the same `--root <dir>` if you installed with one.

## 2. Creating Your First Project

Run the create wizard with the name of your application (or run
`cargo rullst` alone and choose **Create New Project** on its home screen):

<!-- first-hour: create -->
```bash
cargo rullst new my_app
```

The wizard asks its questions in a terminal and writes nothing until you
confirm. Esc goes back one question.

1. **Starter**: **Blank**, **API** or **SaaS**, each with a one-line
   description and a preview of the files it generates (see
   [Rullst Starters](#3-rullst-starters)).
2. **Database**: the primary database. **SQLite** is the simplest local first
   run; PostgreSQL, MySQL and MariaDB need their database service running.
3. **Features**: optional features such as AI, Redis or a storage add-on.
   Selecting none is valid; a storage add-on is not another primary database.
4. **Review**: the files and the commands it will run. Choose
   **Create the project**, **Back** to change an answer, or **Cancel** to
   exit without creating anything.

For a database-backed app, the CLI then compiles the new application once and
runs its initial migration, and prints three numbered next steps: `cd my_app`,
`cargo rullst dev` and the local URL of the generated welcome page, by default
`http://127.0.0.1:3000`:

<!-- first-hour: run -->
```bash
cd my_app
cargo rullst dev
```

Open the local URL reported by the command. Find the generated page code,
change a heading and save. Confirm that the changed page appears after the
successful rebuild/restart. Stop development with `Ctrl+C` before moving or
removing the project directory.

`cargo rullst new my_app --default --dry-run` prints the plan, its file tree
and the commands without creating anything, and `cargo rullst tour` walks
through the main commands without changing a project. Without a terminal
(in CI or a script), `new` needs `--default`; flags such as `--blueprint` and
`--database` answer their questions. See the
[CLI reference](cli_reference.md#cargo-rullst-new-name).

Prefer a small deterministic starter without navigating the wizard?

<!-- first-hour: first-app new-terminal -->
```bash
cargo rullst new first_app --default --blueprint blank --database sqlite \
  --skip-initial-migration
cd first_app
cargo rullst dev
```

The first build compiles every dependency and can take several minutes
(about two and a half minutes for the Blank starter on the machine measured
in [Build times](build-times.md), which also has tips). `--skip-initial-migration`
defers the generator's bootstrap work; the development command runs the
initial migration before starting a generated database-backed app. Never point
this learning project at a production database.

> [!TIP]
> The `cargo rullst dev` command compiles the project and starts the local
> server. Saving changes triggers a real rebuild and supervised process restart;
> a failed build leaves the previous application serving. The same-origin
> browser client refreshes after the new process responds with its generation
> marker. In-memory state resets. No hot-reload scaffold option is required.
> See the [CLI reference](cli_reference.md#cargo-rullst-dev).
> 
> The CLI deliberately generates one audited application profile: Active
> Record with server-rendered `html!` views and HTMX enhancement. The framework
> still exposes lower-level repository and client-runtime foundations for
> application-owned integration. See the [Architecture Choices Guide](architecture-decisions.md).


## 3. Rullst Starters

`cargo rullst new` generates one of three starters. The wizard asks which one
(or pass `--blueprint blank`, `api` or `saas`); each uses the Rullst color
scheme and server-rendered HTML/HTMX patterns, and allocation and latency
depend on the generated page and runtime.

### Blank
**Use Case:** Custom, from-scratch development.
This is the minimal template powered by server-rendered `html!` views and HTMX
without a project-local JavaScript bundle. It includes a simple reactive counter
to demonstrate server-driven communication. Other frontend foundations remain
available as application APIs, but are not advertised as equivalent generated
starters.

### API
**Use Case:** Headless JSON services.
The Blank starter without HTML (`--blueprint api`, or `--api`): JSON routes, an
example write route protected by a bearer `API_TOKEN` written to `.env`, and
the same production security baseline. It can run without a database
(`--no-database`) or with Turso as its primary.

### SaaS
**Use Case:** Subscription-based products and billing.
An opinionated SaaS starting point, pre-wired with:
- User authentication (login, signup, session management, sign out
  everywhere, password change and password reset by email with links built
  from `RULLST_PUBLIC_ORIGIN`).
- Stripe pricing panels and subscription checkout views.
- Secure user dashboard and a Nexus admin for signed-in administrators.

### Larger example applications

Up to 13.0.0-alpha.3 the CLI also generated LMS, Blog, Portfolio and ERP
starters. Since 13.0.0-alpha.4 they are complete applications in the repository's
[`examples/`](examples.md) directory — `examples/lms` (courses, lessons, an
accessible player, enrollment and owner-bound progress), `examples/erp`
(products, stock and orders), `examples/portfolio` (a developer showcase edited
in Nexus) and the `examples/blog` showcase — tested in the workspace. Copy one
to start a bigger application; `cargo rullst new --blueprint lms` (or `erp`,
`portfolio`, `blog`) creates nothing and names the example instead.

---

> [!TIP]
> **Starter evolution:** Starters are continuously checked by generator
> smoke tests. Treat generated code as an application starting point: inspect its
> configuration and rerun `cargo check` and security tests after customization.

### Local Studio and Nexus access

The SaaS starter (and the LMS, ERP, Portfolio and Blog examples) expose
visible buttons for both control surfaces during local development:

- **Nexus** is mounted at `/nexus` and accepts only a verified loopback peer in a
  debug build, so the first local click needs no placeholder password.
- **Studio** starts as a separate debug-only service at
  `http://127.0.0.1:5555`.

This convenience cannot be enabled in a release binary through `RULLST_ENV` or
the legacy `APP_ENV` alias. A
release build does not start the generated Studio task, and Nexus needs a
real sign-in:

- **SaaS** (and the LMS example) admit the user signed in to the application whose
  `users.is_admin` column is true, checked on every request; no extra
  credentials. Grant the first administrator with your database client
  (`UPDATE users SET is_admin = TRUE WHERE email = 'you@example.com';`); that
  administrator can grant others from the Users model in Nexus. See
  [using the application's sign-in](4-rullst-nexus.md#using-the-applications-sign-in).
- **The Portfolio and ERP examples** require unique `NEXUS_ADMIN_USERNAME` and
  `NEXUS_ADMIN_PASSWORD` values (Basic Auth behind verified TLS). Without them
  the application starts without `/nexus` (the ERP without its back office)
  and prints one line naming the missing variable; see
  [serving the application without Nexus](4-rullst-nexus.md#serving-the-application-without-nexus).
  The blog example needs the same variables in a release build and does not
  start without them.

Put production Nexus behind a verified TLS boundary; do not expose Studio
publicly. Studio has no
production mode: it runs only in debug builds and on loopback.

## 4. Database configuration and specialized stores

The wizard deliberately makes two separate decisions:

1. Choose the primary **SQLx Active Record** backend: **SQLite**,
   **PostgreSQL**, **MySQL**, or **MariaDB**. MariaDB uses the MySQL wire
   protocol but has its own executable container contract. The Blank starter
   (web page or JSON API) can instead select **Turso/libSQL** as its primary
   database, through a separate adapter that has no Nexus, Studio database
   pages, `Schema` migrations, `artisan!` or SQLx query builder.
2. Optionally add side stores: **Turso/libSQL** edge SQL, **MongoDB**
   documents, **DuckDB** analytics, **SurrealDB** documents/graph reads,
   **Qdrant** vectors or **Redis** structures. They use their own APIs and do
   not replace the primary pool.

[Database Support](database-support.md) lists what each backend works with;
read it before you choose.

After generation, the default flow compiles the new application once and runs
its initial migration. A clean first build can take several minutes for the
larger SaaS starter. Use `--skip-initial-migration` when the network database is not yet
configured, then run `cargo rullst db:migrate` from the generated project.
`cargo rullst dev` applies pending migrations when it starts, but a deployed
server never does: run the binary's `db:migrate` command on each release, as
described in [Before the first deploy](tutorials/26-one-click-paas-deploy.md#before-the-first-deploy).

The non-interactive flags mirror the second step:

<!-- first-hour: stores new-terminal -->
```bash
cargo rullst new edge_app --default --database mariadb --turso --mongodb \
  --skip-initial-migration
```

Automation can pin the supported starter, database and optional runtime
capabilities. The application architecture is intentionally fixed to Active
Record plus server-rendered `html!`/HTMX:

<!-- first-hour: automation new-terminal -->
```bash
cargo rullst new subscriptions --default --blueprint saas \
  --database postgres --ai \
  --skip-initial-migration
```

For a blank application with no primary relational database, use the explicit
`--no-database` flag. It cannot be combined with `--database`.
Generated SQLx profiles disable the default features of Rullst's umbrella and
`rullst-orm` dependencies and select exactly one strict relational backend, so
a chosen PostgreSQL/MySQL/MariaDB profile is not accidentally compiled through
an implicit SQLite default.

To create a Turso-primary API using the current bounded blank starter:

<!-- first-hour: turso-api new-terminal -->
```bash
cargo rullst new edge_app --default --api --database turso \
  --skip-initial-migration
cd edge_app
cargo rullst db:migrate
```

The generated `.env` selects a persistent, real-SQL offline fallback and does
not invent a SQLx `DATABASE_URL`:

<!-- first-hour: turso-env expect-file=.env -->
```env
TURSO_DATABASE_URL=mock_local
TURSO_AUTH_TOKEN=
TURSO_OFFLINE_PATH=turso-development.db
```

To use Turso Cloud, put the token in its own variable rather than in the URL:

```bash
turso db create my-app-db
turso db tokens create my-app-db
```

```env
TURSO_DATABASE_URL=libsql://my-app-db-username.turso.io
TURSO_AUTH_TOKEN=replace-with-a-secret-token
```

The familiar derive explicitly selects the Turso backend:

```rust
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "users", backend = "turso")]
struct User {
    id: i64,
    name: String,
}
```

`User::all()`, `find`, `save`, `create`, `delete`, `count`, filtering,
ordering and pagination then execute through the primary `TursoOrm` store.
`make:model --migration`, `make:migration`, `db:migrate`, `db:status`, and
`db:rollback` preserve this backend. The current first-class scaffold contract
is deliberately limited to the Blank and API starters; the SQLx-specific SaaS
starter rejects `--database turso` until ported.
Transparent embedded-replica synchronization is not claimed.

`TursoStore` remains available for prepared SQL, bounded result
materialization, transactions and checksummed migrations in either primary or
additive configurations.
See [Polyglot Persistence](polyglot-persistence.md) for complete setup,
security constraints and examples for every optional store.
