# 💡 Rullst CLI - Full Command Reference

The Command Line Interface (`cargo-rullst`) scaffolds projects, invokes build
tools, and provides bounded static-analysis and deployment helpers.

The CLI's `--help` output is authoritative for the installed version. This page
documents the principal commands of the version 13 prerelease CLI and their
security boundaries. Cargo never installs a prerelease unless you name it, so
`cargo install cargo-rullst` installs the end-of-life 12.x CLI; install this one
with an exact version:

```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
```

`cargo-rullst` is a Core package. A few generators depend on outside toolchains
or platforms and carry their own **Maturity** label below the command name:
*Supported* or *Experimental*. The [maturity tiers](maturity.md#cli-generators) page
explains what each label promises.

## Extras (`cargo-rullst-extras`)

The terminal AI assistant ([`cargo rullst ai`](#cargo-rullst-ai-extras)) and
the efficiency report ([`cargo rullst green`](#cargo-rullst-green-extras))
live in the optional, Experimental `cargo-rullst-extras` package, so the main
CLI stays small. Install it with the same version as the CLI:

```bash
cargo install cargo-rullst-extras --version =13.0.0-alpha.4 --locked
```

`cargo rullst ai …` and `cargo rullst green …` run `cargo-rullst-extras ai …`
and `cargo-rullst-extras green …`, found on `PATH` the way Cargo finds its own
`cargo-<name>` plugins: every argument, `--help` included, passes through
unchanged, and the extras' exit status becomes the command's exit status. Only
absolute `PATH` entries are searched, so an executable in the current
directory is never run. Without the extras, the command prints the exact
install command above and exits with status 1; the home menu marks both
entries `extras` and shows the install command instead of failing. The extras
also run on their own (`cargo-rullst-extras ai`, or `cargo rullst-extras ai`);
the assistant's allowlisted `cargo rullst` actions then run the
`cargo-rullst` executable from `PATH`.

## Machine-readable output

A command whose result is data accepts `--json`. The contract is the same for
all of them:

* Standard output carries exactly one JSON document and nothing else: no
  progress lines, banners, tables or ANSI escape codes, whether or not the
  terminal supports colour.
* The document has a top-level `schema` field naming its shape and version,
  `rullst.<name>.v1`. A new field is added without a new version; a removed or
  renamed field, or a changed meaning, gets `v2`. Documents that predate this
  contract also keep their `schema_version` field, with the same value.
* Human messages, progress and failures go to standard error. Exit statuses are
  the same with and without `--json`: a failed check or a refused request still
  exits non-zero, and the document is still printed when the command could
  compute one (`doctor`, `deploy:doctor` and `academy:doctor` print their
  report and then exit with status 1 when a check fails).
* A request that has no JSON form (for example `pkg add --json`) is refused on
  standard error with a non-zero exit status and prints nothing on standard
  output.

| Command | `schema` |
| --- | --- |
| `cargo rullst info --json` | `rullst.cli-info.v1` |
| `cargo rullst doctor --json` | `rullst.cli-doctor.v1` |
| `cargo rullst audit --json` | `rullst.cli-audit.v1` |
| `cargo rullst inspect routes --json` | `rullst.cli-routes.v1` |
| `cargo rullst inspect models --json` | `rullst.cli-models.v1` |
| `cargo rullst inspect schema --json` | `rullst.cli-schema.v1` |
| `cargo rullst pkg list --json` | `rullst.cli-pkg.v1` |
| `cargo rullst upgrade --dry-run --json` | `rullst.upgrade-plan.v1` |
| `cargo rullst deploy:doctor --json` | `rullst.deployment-diagnostic.v1` |
| `cargo rullst academy:doctor --json` | `rullst.academy-diagnostic.v1` |
| `cargo rullst update project prepare --json` | `rullst.project-preparation-result.v1` |
| `cargo rullst update project verify --dry-run` or `--json` | `rullst.project-verification-plan.v1` (dry run), `rullst.project-verification.v1` |
| `cargo rullst update project review --json` | `rullst.project-review-report.v1` |
| `cargo rullst update project apply --json` and `recover --json` | `rullst.project-application.v1` |
| `cargo rullst ai status --json` (extras) | `rullst.ai-status.v1` |
| `cargo rullst ai review --json` (extras) | `rullst.ai-review.v1` |
| `cargo rullst green --json` (extras) | `rullst.green.v1` |

`pkg list --json` prints `{"schema": "rullst.cli-pkg.v1", "table":
"dependencies", "packages": ["rullst-auth"]}`; `table` is
`workspace.dependencies` for a virtual workspace manifest. `audit --report json`
is a different interface: it writes `SECURITY_REPORT.json`
(`rullst.cli-audit-report.v1`) and cannot be combined with `--json`. The
files `cargo rullst generate:ai-context` writes (`rullst.project-context.v1`)
are described in [project context](project-context.md). `db:status` is
printed by your application, not by the CLI, and has no `--json` form yet.

---

## 🏗️ 1. Project Initialization & Maintenance

### `cargo rullst` (no arguments): home screen
Running the CLI without a subcommand opens the v13 home: the RULLST wordmark in
a blue → green → orange gradient, the slogan line with the installed version,
and a summary of where you are.

* **Inside a Rullst project** (the current directory or a parent has a
  `Cargo.toml` whose package depends on `rullst`): the package name, the
  enabled `rullst` features (`default` when default features are on), the
  database family and where it is configured (the process `DATABASE_URL`, then
  `.env`, then `[database].url` in `Rullst.toml`; `TURSO_DATABASE_URL` for a
  Turso-primary project), the number of `src/migrations` files and the latest
  one, and the Git branch. Connection URLs are never displayed. The menu leads
  with dev, dash, scaffolding, database, `doctor` and deploy, keeps every
  project operation of earlier releases, and offers **AI Assistant** and
  **Green Report**, marked `extras`: without
  [`cargo-rullst-extras`](#extras-cargo-rullst-extras) they print its install
  command instead of failing. Started from a subdirectory, menu
  commands run at the project root (except `new`). The migration count is the
  files on disk; run `cargo rullst db:status` for the applied state.
* **Outside a project**: the summary links the docs ([start here](start-here.md)
  and this reference) and the menu leads with project creation, followed by
  the operations for an existing project.

Both menus include **Search All Commands**, a fuzzy command palette over every
command, including the ones attached at runtime (`update ...`, `make:privacy`,
`deploy:doctor`, `completions`, ...). Type to filter by name, alias or
description; every word must match, names rank above descriptions, and ties go
to the shorter name, then alphabetical order. ↑/↓ (or Tab, Ctrl+P/N) move,
Enter runs the selection (asking first for its required arguments, as a menu
when the values are fixed), Esc returns to the home menu and Ctrl+C exits.
`cargo rullst --help` is the non-interactive equivalent.

The opening animates for about 0.7 s on every run; any key skips the
animation. `RULLST_OPENING=daily` animates only the first run of each day
(later runs draw the final frame instantly) and `RULLST_OPENING=static` never
animates. For `daily`, the day of the last animation is kept in the user cache
directory
(`$XDG_CACHE_HOME/rullst-ui-v1` or `~/.cache/rullst-ui-v1`;
`%LOCALAPPDATA%\rullst-ui-v1` on Windows), never in the project. If that file
cannot be read or written, the opening is static.

Colours use 24-bit RGB when `COLORTERM` is `truecolor` or `24bit` and the
nearest xterm 256-colour entry otherwise. `RULLST_REDUCED_MOTION=1` (or
`true`/`yes`) keeps the colours without animation. `NO_COLOR` prints the plain
line `RULLST v<version> · SECURE, FAST AND AI-NATIVE RUST FRAMEWORK` while the
menu stays interactive. When standard input, output or error is not a
terminal, `CI` is set or `TERM=dumb`, the CLI prints the plain line, the
summary and the equivalent commands, then exits successfully without
prompting.

In a terminal, at most once a day, the home (or any command, on standard
error after it finishes) prints `A newer cargo-rullst is available: cargo
install cargo-rullst --version =X --locked` when crates.io has a newer release
of the same major; the catalog is fetched in the background at most once a day
and cached privately, and nothing is installed. `RULLST_DISABLE_UPDATE_CHECK=1`,
`CARGO_NET_OFFLINE=true` or `CI=true` turns it off. See
[Update notice and installing a newer CLI](#cargo-rullst-update-check-1210-working-source-unreleased).

### Errors and exit codes
Every failure is printed on standard error by one shared report:

```text
error: Not inside a Rullst project
  What happened  `cargo rullst make:model` must run at the root of a Rullst project; ...
  How to fix     The project root is `../..`: run `cd ../..` and try again.
  Docs           https://rullst.github.io/Rullst/book/start-here.html
```

The common failures have their own title and fix: not inside a Rullst project
(pointing to an enclosing project root when there is one), a missing tool
(`cargo`, `rustup`, `git`, `docker`, `flyctl`, ...), a missing Rust target
(`rustup target add <target>`), an unreachable database, a port already in
use, permission denied, invalid input, network failures, an interrupted
prompt and a starter that moved to the repository's examples (`new --blueprint
lms|blog|portfolio|erp`). Anything else gets a generic report naming the failed command.
Connection-string passwords (up to the host, even when they contain an
unencoded `@`), `*_KEY`/`*_SECRET`/`*_TOKEN`/`password` values (also under the
quoted keys of JSON or `Debug` maps), bearer tokens and well-known credential
formats (including Stripe `sk_live_`/`whsec_` secrets) are masked, and
terminal control characters are replaced, in every line of the report. The
whole message is redacted before a long one is cut to 4,096 characters.

A command started from the home menu or the command palette prints its own
report, and `cargo rullst` then exits with that command's status (for example
`2` for a usage error) without a second report.

`-v`/`--verbose` (accepted by every command) adds the underlying causes and
the error's `Debug` form, still redacted. A panic prints a short "Internal
error" report instead of the default panic message and backtrace unless `-v`
or `RUST_BACKTRACE` is set.

| Exit status | Meaning |
| :--- | :--- |
| `0` | Success (`doctor`: no failed check; warnings are allowed). |
| `1` | The command failed, including `doctor` with a failed (✗) check. |
| `2` | Usage error: an unknown command, invalid arguments or an argument that is not valid UTF-8. |

An unknown command lists the closest commands at the level where it was typed
(`cargo rullst make:modek` suggests `make:model`; `cargo rullst update projetc`
suggests `update project`; `cargo rullst db` lists the `db:*` group). Ranking
ignores case and separators (`make-model`), then uses edit distance with
transpositions, an exact `group:name` segment (`migrate`) and prefixes.

Colour follows the home screen rules on each stream: none for `NO_COLOR`, `CI`,
`TERM=dumb` or a stream that is not a terminal. When standard output is not
coloured, generator output is plain as well. Long steps (the first build of
`new`, the Cargo gates of `upgrade`) animate a spinner only when standard
output is a terminal without `NO_COLOR`, `CI`, `TERM=dumb` or
`RULLST_REDUCED_MOTION`; otherwise, as in CI logs and pipes, they print one
plain line without escape sequences or carriage returns.

After a generator succeeds, the CLI prints up to three concrete **Next steps**
(for example `cargo rullst db:migrate` after `make:model -m`). Generators that
already print their own instructions (`new`, `auth`, `deploy`,
`make:k8s`) add none, and `--json` invocations never print them.

### `cargo rullst completions <shell>`
Prints a completion script for `bash`, `zsh`, `fish`, `powershell` or `elvish`
generated from the complete command tree, runtime-only commands included.
Scripts complete the `rullst` executable that `cargo install cargo-rullst`
installs beside `cargo-rullst` (Cargo's own completion does not delegate the
arguments of external subcommands); `--bin cargo-rullst` targets the other
executable.

```bash
cargo rullst completions bash > ~/.local/share/bash-completion/completions/rullst
cargo rullst completions zsh > ~/.zfunc/_rullst      # with fpath+=(~/.zfunc)
cargo rullst completions fish > ~/.config/fish/completions/rullst.fish
cargo rullst completions powershell >> $PROFILE
```

### `cargo rullst info [--json]` (alias `version`)
Prints the CLI version, platform, `rustc`/`cargo` versions and the detected
project (name, root, `rullst` requirement, features, database family and
source, migration files, Git branch). It reads local files and runs
`rustc --version`/`cargo --version` only; connection URLs are never printed.
`--json` prints:

```json
{
  "schema": "rullst.cli-info.v1",
  "schema_version": "rullst.cli-info.v1",
  "cli": { "name": "cargo-rullst", "version": "13.0.0" },
  "platform": { "os": "linux", "arch": "x86_64" },
  "toolchain": { "rustc": "1.98.1", "cargo": "1.98.1" },
  "project": {
    "name": "shop",
    "root": "/work/shop",
    "relative_root": null,
    "rullst": "13",
    "features": ["default", "orm"],
    "database": { "status": "configured", "kind": "SQLite", "source": ".env" },
    "migrations": { "count": 2, "latest": "m20260101000000_create_users" },
    "git_branch": "main"
  }
}
```

`project` is `null` outside a project; `toolchain` values are `null` when the
tool is missing. `database.status` is `configured`, `not_configured` or
`unreadable` (with `reason`); `rullst` is the version requirement, or `path`,
`git` or `workspace`.

### `cargo rullst new <name>`
Creates a Rullst project from scratch. Since version 12, `new` intentionally
generates one audited application architecture: Active Record for
database-backed code and server-rendered `html!` views enhanced with HTMX for
full-stack pages.

In a terminal, `cargo rullst new` (with or without a name) runs the v13
create wizard; the home screen's **Create New Project** entry runs the same
wizard. It asks, one screen at a time:

1. the project name, when none was given (letters, digits, `_` and `-`,
   starting with a letter; Rust keywords and existing paths are refused). A
   name given on the command line is checked by the same package-name rules
   before the first question;
2. the **starter** — **Blank** (a server-rendered web page with HTMX),
   **API** (the Blank starter as a headless JSON API) or **SaaS**
   (authentication, Stripe billing and a pricing page) — each with a one-line
   description and a compact preview of the file tree it generates. The
   preview is rendered by the real project writers into a private temporary
   directory that is removed immediately; nothing is built and no network is
   used;
3. the **primary database** (Turso and *No database* only for Blank and API);
4. **optional features** (zero or more): AI, Redis adapters, Dockerfile
   and the Turso, MongoDB, DuckDB, SurrealDB and Qdrant add-ons;
5. a **review** of the answers, the file tree and the commands `new` will run
   (the initial migration, unless skipped or database-free), with *Create
   the project*, *Back* and *Cancel*.

↑/↓ (or `j`/`k`, or a digit) moves, Space toggles a feature, Enter confirms
and Esc, Backspace or ← returns to the previous question. On a terminal too
short for every choice, the list scrolls with the highlighted entry and counts
the hidden ones. Cancel and Ctrl+C
create nothing. Colours follow the home screen: `NO_COLOR` removes them and
24-bit colour needs `COLORTERM=truecolor`. Every question has a flag; a
question answered by a flag is skipped. When standard input, output or error
is not a terminal, `CI` is set or `TERM=dumb`, `new` never prompts: without
`--default` it exits with an error that names the flags instead.

After creation, `new` prints numbered next steps: `cd <name>`,
`cargo rullst dev` (which applies pending migrations) and the URL of the
generated welcome page (`http://127.0.0.1:<port>`, where the port comes from
`PORT`, the project's `.env`, `[app].port` in `Rullst.toml`, then 3000; JSON
APIs show a `curl` command instead).

The choices that materially change the generated application:
* **Starter:** Blank, API and SaaS (`--blueprint` values `blank`, `api` and
  `saas`; `api` is the Blank blueprint with `--api`). The LMS, Blog, Portfolio
  and ERP blueprints left the CLI in 13.0.0-alpha.4 and are now complete
  applications in the repository's `examples/` directory; see
  [Starters that moved to the examples](#starters-that-moved-to-the-examples).
* **Persistence:** a primary relational backend (SQLite, PostgreSQL, MySQL,
  MariaDB, or bounded Turso-primary for blank/API) plus optional Turso/libSQL,
  MongoDB, DuckDB, SurrealDB, and Qdrant capabilities. The optional selector
  accepts zero or more choices and omits capabilities already selected by the
  primary profile or flags. Specialized adapters remain separate from SQLx
  Active Record.
* **Application profile:** the Blank and SaaS starters use the audited `html!`
  SSR/HTMX path; API (`--api`) uses the headless JSON path. Repository, LiveView, Wasm Island,
  Pico.css and Tera foundations remain application-owned APIs and are not
  presented as equivalent generated profiles.
* **Arguments:**
  * `<name>`: The folder and package name (e.g., `my_startup`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API from the Blank starter (no HTML view rendering), the same as `--blueprint api`; `--blueprint saas` rejects it instead of ignoring it. Its example write route, `POST /api/messages`, is a bearer machine endpoint: clients send `Authorization: Bearer <API_TOKEN>` with the random `API_TOKEN` written to `.env`, and the server does not start without it (see [which security layer to use](security-layers.md#what-generated-applications-already-have)). The interactive wizard then skips its starter question instead of letting its Blank default replace the flag.
  * `--docker`: Adds a multi-stage `Dockerfile` and `.dockerignore`. The
    `.dockerignore` mirrors the generated `.gitignore`: it excludes `.env` and
    `.env.*` (except `.env.example`), SQLite and DuckDB files
    (`*.db`, `*.sqlite`, `*.sqlite3`, `*.duckdb` and their journals, one list
    shared by both files) and the
    host-local `.cargo/config.toml` described below, so the builder's
    `COPY . .` never sends them to a (possibly remote) builder. An existing
    `.dockerignore` is kept unchanged. Since 13.0.0-alpha.3 the dependencies
    are built in their own cached layer without an extra tool: a `planner`
    stage copies only the manifests, `Cargo.lock` and `rust-toolchain(.toml)`,
    with `fn main() {}` standing in for every target (`src/main.rs`,
    `src/lib.rs`, `src/bin/*.rs`, explicit `path = "*.rs"` targets) and build
    script, and the builder runs `cargo build --release --locked` on that
    skeleton before `COPY . .`, touches the real `.rs` files and builds again
    with `--locked`. Docker reuses the dependency layer until a manifest, the
    lockfile or the set of targets changes; workspace members keep their
    paths. Since v13 the runtime
    stage is `gcr.io/distroless/cc-debian12:nonroot` (glibc, libgcc, libstdc++,
    CA certificates and tzdata; no shell or package manager). It runs as
    UID/GID 10001 with `HOME=/app`, sets the production bind address and copies
    local static/config assets when present. The builder stage prepares the
    `/app` tree, so an explicit SQLite selection still uses the writable
    `/app/data` directory. Without a shell the image has no `HEALTHCHECK`;
    let the platform probe `/health` and `/ready` over HTTP. The migration job
    runs the binary directly (`/app/<name> db:migrate`). The image size of this
    template has not been measured yet; measure it with `docker image ls` in CI
    or on a machine with Docker. Secrets are never
    embedded and no anonymous volume is declared. Run schema migrations as one
    deployment job before starting or rolling multiple replicas; the generated
    image deliberately does not race migrations from every application process.
    Compose services, persistent-volume ownership, backup/restore and platform
    deployment hardening remain explicit project work. Every starter mounts
    `rullst::health::health_router()`, so the `/health` and `/ready` probes
    used by `make:k8s` manifests and the `deploy` Caddyfile answer `200`
    without authentication.
  * `--turso`: Adds Turso/libSQL as a side store beside the primary database: the direct Hrana HTTP v3 adapter, checked migrations and its real-SQL offline development fallback. It does not imply transparent replication.
  * `--mongodb`: Adds MongoDB as a side store: typed document CRUD and its deterministic offline store.
  * `--duckdb`: Adds DuckDB as a side store for in-process analytics; the optional native dependency increases the first build time.
  * `--surrealdb`: Adds SurrealDB as a side store: HTTP document CRUD and bounded read-only graph queries.
  * `--qdrant`: Adds Qdrant as a side store for bounded dense-vector operations and generates empty/`mock_*`-compatible environment fields; it is not the SQL primary.
  * `--nix` and `--buildah` were removed in 13.0.0-alpha.4; see [Advanced deployment](advanced-deployment.md).
  * `--default`: Skips every question and uses deterministic defaults for anything no flag sets (name `app`, Blank, SQLite, no features), intended for CI and reproducible scaffolding.
  * `--blueprint <blank|api|saas>`: Selects the starter; the wizard skips that question. `lms`, `blog`, `portfolio` and `erp` still parse but fail before anything is written, naming the example that replaced them (exit status 1).
  * `--database <sqlite|postgres|mysql|mariadb|turso>`: Selects the primary database; the wizard skips that question. Network databases must be configured before migration bootstrap. The SQLx backends (`sqlite`, `postgres`, `mysql`, `mariadb`) work with every starter. `turso` is accepted only with the Blank starter (web page or `--api`) and has no Nexus, Studio database pages, `Schema` migrations or SQLx ORM; the wizard's choice, `--help` and the review/dry-run screen say so. See [Database Support](database-support.md).
  * `--no-database`: Generates the Blank or API starter without a primary relational database; it conflicts with `--database` and `--blueprint saas` rejects it.
  * `--ai`: Enables the umbrella AI facade in the generated manifest.
  * `--redis`: Enables the umbrella Redis queue and cache capabilities and the Redis-structures side store (the direct ORM Redis feature).
  * `--skip-initial-migration`: Generates the project without running the best-effort initial database migration. `cargo rullst dev` applies pending migrations when it starts; run `cargo rullst db:migrate` explicitly otherwise.
  * `--dry-run` (v13): Prints the plan (the answers, a compact file tree with the exact number of files and the commands it would run) and exits without creating anything. With `--default` it never prompts; in a terminal without `--default` the wizard's review ends with *Finish the dry run*.
  * `--vcs <git|none>` (v13, default `git`): Like `cargo new`, initializes a Git repository in the new project when `git` is installed and the destination is not already inside a Git work tree; `none` skips it. The generated `.gitignore` keeps `.env`, `.env.*` (except `.env.example`) and `/target` out. Without Git, `new` prints a warning and keeps the project; `cargo rullst audit` checks committed secrets through Git.

Since v13, `--blueprint`, `--database`, `--no-database`, `--ai` and `--redis`
no longer require `--default`: in a terminal they answer their questions and
the wizard asks the rest. `--api` selects the API starter; `--no-database` and
`--database turso` limit the starter question to Blank and API. The retired
`--hot-reload` still requires `--default` and is rejected.

When the generating Linux host has `mold` or `lld`, the project's
`.cargo/config.toml` selects it to speed up local linking. That file describes
the generating machine only: the generated `.gitignore` and `.dockerignore`
exclude it, so CI runners, teammates and the container builder (which has
neither linker) build with the toolchain default.

Since v13 the generated `Cargo.toml` also has a `[profile.release]` with
`lto = "thin"`, `codegen-units = 1` and `strip = "symbols"`; `panic` stays
`unwind` so one panicking handler cannot stop the server. Release builds take
longer and their backtraces lose symbol names; the [measured release defaults](green-software-roadmap.md#measured-release-defaults)
state the size and build-time effect on one starter. Since 13.0.0-alpha.4 it
has a `[profile.dev]` as well: the application keeps line tables (file:line in
panics and backtraces) and dependencies are built without debug information,
which shortens debug builds and rebuilds; see [Build times](build-times.md).

Without `--skip-initial-migration`, project creation performs the first Cargo
build before applying migrations. A clean first build can take several minutes,
especially for the larger SaaS profile; the animated status remains visible
while Cargo is working. Later migration and server runs reuse that project-local
build cache.

Preview a starter without writing anything:

```bash
cargo rullst new shop --default --blueprint saas --dry-run
```

For example, the release gate can generate a SaaS starter without prompts or
network-dependent bootstrap work:

```bash
cargo rullst new packaged-saas --default --blueprint saas --skip-initial-migration
```

A deterministic profile can pin the starter, database and optional features:

```bash
cargo rullst new operations-portal --default --blueprint saas \
  --database mariadb \
  --ai --redis --skip-initial-migration
```

Every generated manifest also declares `validator` 0.21 with its `derive`
feature. `#[derive(rullst::Validate)]` expands to `::validator::…` paths that
only a direct dependency resolves, so `ValidatedForm`/`ValidatedJson` DTOs
compile without adding it by hand; Rullst Core already builds that version.

Generated SQLx applications disable the default features of both the umbrella
`rullst` dependency and the direct `rullst-orm` dependency, and select exactly
one strict primary profile (`strict-sqlite`, `strict-postgres`, or
`strict-mysql`; MariaDB uses the MySQL protocol). This prevents an implicit
SQLite default from masking the chosen backend and keeps the other drivers,
including bundled SQLite, out of the build. Turso-primary and database-free
profiles keep `rullst-orm`'s default drivers for its `AnyPool`.

#### Generated-project verification boundary

The repository does not treat template rendering as sufficient evidence. A
structural contract materializes 10 internal starter/profile shapes (five
public directly linked layouts plus five legacy DLL layouts retained for
regression) and checks paths, Rust syntax and manifests. A slower four-case
set crosses every starter, hot and non-hot layouts, database/API boundaries and
a SaaS release build, runs every generated test target, and constructs the
public router of each hot-reload project using offline-safe defaults. A
separate seven-case test invokes the public `cargo rullst new` binary and
verifies exact feature selection for SQLite, PostgreSQL, MySQL, MariaDB, AI,
Redis, Turso, MongoDB, DuckDB, SurrealDB and Qdrant across the Blank, API and
SaaS starters plus a polyglot profile. The invocation starts outside the source checkout, proving
that an unpublished pre-release CLI retains its exact matching checkout as a
path source instead of requesting unavailable registry packages.

The public polyglot profile uses `cargo check` in that CLI-level set because a
second bundled-DuckDB test build adds no adapter behavior and can consume
several GiB on small machines. DuckDB, MongoDB, SurrealDB, Turso and Qdrant
runtime behavior is exercised by their dedicated ORM matrices instead. These
gates prove reproducible local generation and bounded offline construction;
they do not prove provider accounts, production deployment, browser behavior
or application-specific authorization.

#### Starters that moved to the examples

Since 13.0.0-alpha.4 `cargo rullst new` offers three starters: Blank, API and
SaaS. The other blueprints left the CLI and live on as complete applications
in the Rullst repository, tested in its workspace:

| Former `--blueprint` | Example | What it shows |
| :--- | :--- | :--- |
| `lms` | [`examples/lms`](https://github.com/Rullst/Rullst/tree/main/examples/lms) | Course catalog, modules and lessons, accessible player, enrollment, owner-bound progress, sign-in and a Nexus admin |
| `portfolio` | [`examples/portfolio`](https://github.com/Rullst/Rullst/tree/main/examples/portfolio) | A developer portfolio edited in Nexus |
| `erp` | [`examples/erp`](https://github.com/Rullst/Rullst/tree/main/examples/erp) | Products, stock movements and orders behind one administrator policy |
| `blog` | [`examples/blog`](https://github.com/Rullst/Rullst/tree/main/examples/blog) | The blog showcase (a larger application than the old starter) |

`--blueprint lms`, `blog`, `portfolio` or `erp` still parses but fails before
anything is written: the report names the example directory and its GitHub URL
and exits with status 1. The examples are applications to copy from, not
supported starters: there is no generator or upgrade path behind them. Each
README says how to run it and how to switch its workspace dependencies to the
published crates. Applications generated earlier from these blueprints keep
working: their code is theirs.

### `cargo rullst tour` (v13)
A short guided walkthrough of the main commands in seven skippable steps:
`new`, `dev`, `dash`, `make:*`, `db:*`, `doctor` and `ai`. Each step explains
what the command family does, lists commands to try and offers one read-only
example that runs only when you pick it: `cargo rullst new tour-demo --default
--dry-run` for `new` and `--help` for the others (the `ai` step falls back to
`generate:ai-context --help` in a build without `cargo rullst ai`). Next,
Previous (or Esc) and *Quit the tour* move between steps. The tour needs no
network and never creates or changes a project.

```bash
cargo rullst tour          # guided, in a terminal
cargo rullst tour --list   # every step and its example, without prompting
```

Outside an interactive terminal (pipes, `CI`, `TERM=dumb`) `cargo rullst tour`
prints the same list as `--list` and exits successfully.

### `cargo rullst upgrade`
Plans or applies a transactional application upgrade. The target defaults to
the exact installed `cargo-rullst` version; `--to <VERSION>` accepts an exact
version in the same major release train as that CLI. The command fails before
writing anything when the target is older than a managed requirement's lower
bound or than a Rullst package locked in `Cargo.lock`, so an older CLI never
downgrades the project; install a CLI that is not older than the project.

```bash
# Human-readable plan; no writes or dependency resolution
cargo rullst upgrade --dry-run

# Versioned machine-readable plan
cargo rullst upgrade --dry-run --json

# Apply every source rule, not only those introduced after the project's version
cargo rullst upgrade --dry-run --all-rules

# Backed-up apply + cargo fix + cargo check
cargo rullst upgrade

# Deliberately inspect a failed partial migration instead of auto-rollback
cargo rullst upgrade --keep-on-failure

# Recover a persisted snapshot, including after interruption
cargo rullst upgrade --restore target/rullst-upgrades/<run-id>
```

The CLI uses Cargo metadata to scope workspace manifests, preserves TOML
comments/order, updates normal, inline, workspace, target-specific and renamed
Rullst dependencies, and reports unversioned path/git entries. Before applying,
it snapshots workspace manifests, the root `Cargo.lock`, and Rust sources under
`target/rullst-upgrades/`; a failed Cargo gate restores them by default. The
reports use the `rullst.upgrade-plan.v1` schema and include version-selected
source findings.

The v13 CLI's `rullst-upgrade-rules-v4` catalog (107 rules) parses Rust sources
(with `syn`; comments, doc comments and strings never match an API rule), Cargo
manifests and generated project files, and reports each finding as
`MUST-CHANGE` (a compile-breaking or API-shape change) or `REVIEW` (changed
behaviour of an API, feature or configuration in use) with a stable `V13-*`
code, `file:line`, a one-line message and the title of its row in the
[v13 migration guide](migration-v13.md#changes-from-the-published-1210-source).
Findings do not stop the transaction; a failed Cargo gate names the
must-change findings and repeats the guidance of `V13-CAPITAL-REMOVED` and
`V13-MAIL-REMOVED` (removed APIs reported for review).
Each rule records the release that introduced its change. The plan reads the
project's Rullst version (the oldest version a Rullst requirement admits or
`Cargo.lock` holds); for a project already on a 13.0 prerelease it applies only
the rules introduced after that version and prints how many earlier rules it
skipped and how many findings they would have added. `--all-rules` applies
every rule; a v12 project, or one whose version cannot be read, gets every
rule anyway.
`update project verify`/`review`/`apply` refuse a preparation with must-change
findings; preparations apply every rule. In JSON, each
`source_findings` element keeps `path`, `line`, `code`, `severity` (`BLOCKER`
or `REVIEW`) and `message` and adds `kind`, `introduced_in`, `migration_row`
and `migration_url`; the root adds `finding_counts`, `unscanned_sources`,
`migration_guide` and `rule_selection` (`current_version`, `all_rules`,
`skipped_rules`, `hidden_findings`). `cargo rullst ai upgrade` proposes reviewed fixes for the
findings. See the [tutorial](tutorials/36-assisted-framework-upgrades.md#v12-to-v13-source-findings)
and its [rule classification](tutorials/36-assisted-framework-upgrades.md#v12--v13-rule-classification).

Since 12.1, managed requirements use exact `=VERSION` pins. The final
`cargo check --workspace --all-targets --locked` validates the lockfile produced
by `cargo fix` without resolving a different version. Broader dependency ranges
remain an application decision after reviewing the update.

Process-level fixtures upgrade v12 origins, reject retired pre-v12 origins with
v12-first guidance, verify restoration across multiple workspace
members, retain a deliberately failed edit only with `--keep-on-failure`, and
restore that retained snapshot on demand. Symlinked Rust sources are rejected
before a transaction begins. This is recovery evidence for the bounded file and
Cargo operation; it is not an automatic application, database or deployment
migration.

The command does not install the CLI globally, rewrite Axum/SQLx/Tokio imports,
run database migrations, modify secrets or authorization, validate live
providers, or replace the project's test suite. Follow the
[assisted upgrade tutorial](tutorials/36-assisted-framework-upgrades.md) and the
[v13 migration guide](migration-v13.md).

<a id="cargo-rullst-update-check-1210-working-source-unreleased"></a>

### Update notice and installing a newer CLI

The CLI never downloads, verifies or replaces its own binary. Install a newer
one with Cargo, using its exact version:

```bash
cargo install cargo-rullst --version =VERSION --locked
```

At most once a day, in an interactive terminal, `cargo rullst` tells you when a
newer release of its major is on crates.io: a newer stable release, or for a
prerelease CLI also a newer prerelease. The notice prints that exact command
(`A newer cargo-rullst is available: cargo install cargo-rullst --version =X
--locked`) on the home screen, or on standard error after a command finishes.
It never delays or fails a command:

- the crates.io catalog is fetched in a background thread at most once a day
  (one bounded HTTPS request, 4 seconds in total, no redirects) and kept in
  the private user cache (`$XDG_CACHE_HOME/rullst-update-v1`,
  `$HOME/.cache/rullst-update-v1` or `%LOCALAPPDATA%\rullst-update-v1`;
  unsafe owners, permissions or links make it unusable);
- the notice itself reads only that cache, and the day of the last notice and
  of the last request are small stamps in `rullst-ui-v1/` in the same cache
  base;
- `RULLST_DISABLE_UPDATE_CHECK=1`, `CARGO_NET_OFFLINE=true`, `CI`, and any
  standard stream that is not a terminal silence it completely: no request,
  no file and no output.

**Removed in 13.0.0-alpha.4:** the self-updater (`update check`,
`update verify`, `update stage` and `update install review|apply|recover`)
and the CLI half of `update guided` (`--scope`, `--root`, `--allow-major`,
`--prerelease`). The native CLI executables and their manifests stay
attached to each GitHub release and can be checked by hand as described in
[Verifying a release](https://github.com/Rullst/Rullst/blob/main/SECURITY.md#verifying-a-release).

<a id="cargo-rullst-update-guided-1210-working-source-unreleased"></a>

### `cargo rullst update guided` (since 12.1.0)

```bash
cargo rullst update guided --to VERSION --project ./my-app
cargo rullst update guided --to VERSION --project ./my-app --offline
```

This interactive entry point composes the isolated project commands described
below: prepare, verify, review and apply. `--to` must be in this CLI's major,
because the running CLI carries only its own major's migration rules; to move
to another major, install that CLI first and run its guided flow. Each complete
review appears before its own default-no confirmation. The flow reuses the
exact version, directories and review digest without shell commands or manual
copying. It reports elapsed milliseconds per executed stage, excluding time
spent answering prompts. Declining stops before the next operation; completed
steps and their recovery records remain available.

Project verification asks whether Cargo may use the network and requires
explicit consent to execute trusted build scripts, macros and tests.
`--offline` keeps it offline. Feature selection and command deadlines use the
same `--all-features`, `--features`, `--no-default-features` and
`--timeout-seconds` options as project verification. PATH, databases and
deployments are not changed.

Piped input/output rejects before I/O; use the explicit commands with JSON and
review digests for automation. This flow does not bypass ownership or recovery
checks.

<a id="cargo-rullst-update-project-prepare-1210-working-source-unreleased"></a>

### `cargo rullst update project prepare` (since 12.1.0)

Prepare dependency edits in a private source copy for review:

```bash
cargo rullst update project prepare --project ./my-app --to VERSION --json
```

The project must be a Git working directory containing `Cargo.toml`. The default
target is this CLI's exact version, within its major train. Tracked and
non-ignored untracked files are copied with their current contents, including
uncommitted edits and tracked deletions. The root `Cargo.lock` is retained even
when ignored by an older generator. Other ignored files are omitted; tracked
secrets are still tracked inputs. The original files and Git index are not
edited. Limits are 100,000 entries, 64 MiB/file and 512 MiB/source snapshot;
the two copies can consume about 1 GiB before any build.

The command rejects linked/special inputs, unsupported paths, unknown migration
origins, ambiguous/unversioned managed dependencies and version/lockfile
downgrades. It accepts source majors 12 and 13 and records the
`rullst-upgrade-rules-v4` source findings in the plan; verification, review and
application refuse a preparation with must-change findings (fix them in the
original project and prepare again), while review findings stay in the plan.
Older applications first upgrade to v12 with the v12 CLI. The
exact-version editor preserves TOML
comments. Offline, locked, dependency-free Cargo metadata enumerates workspace
members inside the copy. Rustup auto-installation is disabled using its
[documented environment setting](https://rust-lang.github.io/rustup/environment-variables.html);
Git and Cargo must already be installed on absolute trusted PATH entries.
Every Cargo/rustc invocation in a project copy (preparation, verification,
review and apply) runs with `RUSTUP_TOOLCHAIN` pinned to the caller's
toolchain: an inherited `RUSTUP_TOOLCHAIN` (rustup sets it for `cargo rullst`),
otherwise rustup's configured default. A `rust-toolchain`/`rust-toolchain.toml`
in the project, including a `path` toolchain, is therefore not honored. Run the
CLI from outside an untrusted project: invoking `cargo rullst` inside it lets
rustup select that project's toolchain for the CLI process itself.

The result points to `before/`, `candidate/` and `preparation.json` inside the
private update cache. JSON uses `rullst.project-preparation-result.v1`, with a
`rullst.project-preparation.v1` record of original file hashes and absences.
Compare the two trees and inspect the plan. Failed preparation removes its own
new staging directory. Successful preparations remain for review and can be
deleted by the caller after use.

Preparation executes no builds, procedural macros or tests, does not resolve a
new candidate lockfile, and authorizes neither execution nor application. The
copy is not a sandbox. Candidate verification, review, application and
recovery are described below. The existing `upgrade` command remains separate.

<a id="cargo-rullst-update-project-verify-1210-working-source-unreleased"></a>

### `cargo rullst update project verify` (since 12.1.0)

Review the commands and then explicitly authorize trusted project execution:

```bash
cargo rullst update project verify --prepared PATH --dry-run --all-features
cargo rullst update project verify --prepared PATH --allow-project-code --all-features --json
```

`PATH` is the private directory returned by preparation. The command validates
its records, before/candidate copies and current original files under an
exclusive operation lock. Unknown/stale inputs and unresolved migration findings
fail before project code runs. Fix findings in the original and prepare again.
Builds and tests use another fresh private copy, preserving the reviewed copy.

The sequence probes `rustc --version --verbose` and `cargo --version`, resolves
`Cargo.lock` with `cargo update --workspace`, checks all workspace targets, and
runs workspace tests with the resolved lockfile. That resolution keeps every
existing lock entry the edited manifests still accept and resolves only what
the dependency edits require; it does not upgrade unrelated pins (a missing
lockfile is resolved in full). Every managed Rullst package must resolve to the
exact target.
Default features apply unless `--all-features`, `--features names` or
`--no-default-features` selects another policy. This verifies that policy only;
application-specific service/browser/deployment tests remain separate.

Cargo is offline by default. `--allow-network` permits dependency retrieval but
cannot override `CARGO_NET_OFFLINE=true`. Rustup does not install missing
toolchains, and the project's toolchain file is ignored as described above; set
`RUSTUP_TOOLCHAIN` explicitly to verify with the project's pinned channel. `--timeout-seconds` bounds each command (default 900, range 1–3,600);
stdout/stderr are each capped at 8 MiB. Private logs retain failed Cargo output;
failure, cancellation or timeout never records acceptance. Builds/tests inherit
the caller's environment and can affect external files, databases or services:
this copy and process cleanup are not a sandbox. Use trusted projects only.

Success emits `rullst.project-verification.v1` and a private `verification.json`
with commands, log hashes and final file digests. Its verified candidate has the
resolved lockfile; original files are not edited. Source or baseline changes
during verification and unexpected candidate writes are rejected. Review the
complete diff and logs; compiler overrides/wrappers require separate review.
The report is not a reusable apply token and grants no application/deployment
authority. Final native application/recovery acceptance remains open.
Retained source copies can consume up to about 2 GiB, plus build outputs/logs.

### `cargo rullst update project review` (since 12.1.0)

```bash
cargo rullst update project review --verified PATH --json
```

Use `verified_directory` from the verification result. Review validates the
stored command policy, successful statuses, bounded logs, original/prepared
files and verified candidate again under operation locks. Changed evidence is
rejected. Git produces the full dependency diff with external helpers, text
conversion and paging disabled; output is bounded to 8 MiB. No build/test runs
and original files remain untouched.

JSON contains `rullst.project-review.v1`, before/after file hashes, the full
`diff` and `review_sha256` binding its evidence and contents. The digest is not
authorization to apply changes; review does not invoke the legacy in-place
upgrade command. The digest also binds the source access policies.

### `cargo rullst update project apply|recover` (since 12.1.0)

```bash
cargo rullst update project apply --verified PATH --approved-review SHA256 --json
cargo rullst update project recover --verified PATH --approved-review SHA256 --json
```

After reviewing the complete diff and logs, supply that exact `review_sha256`.
Stop editors/builds and other writers first. The CLI revalidates all evidence,
locks the canonical source within the configured private cache, stages every
replacement and persists an intent before changing any original. Only reviewed
workspace manifests and the root lockfile can change. Original hardlink aliases
are not truncated, and a newly created lockfile cannot overwrite a competing file.

Recovery checks every target before restoring any file. A changed file must match
this operation's before or after state and access policy; later conflicting edits
are refused. Unrelated files remain untouched. A root lockfile originally absent
is removed only if it still matches the recorded created file. Repeating recovery
is supported. Keep both private source copies and `application.json` until finished.

Replacement is atomic per file, not for the whole workspace. Partial failures
retain recovery evidence and report progress. Unix mode/owner/group and Windows
owner/group/DACL/integrity label are bound to the review (1 MiB total policy budget). Unix extended ACLs/xattrs and special
mode bits (except the SELinux `security.selinux` label, which the replacement
staged in the same directory receives like any new file there), Windows read-only/special attributes, alternate streams, resource/central-access
policies and policies that cannot be
recreated exactly require manual handling. Other updater cache configurations and
filesystem aliases do not share the lock. Forced termination during staging may
leave sibling `.rullst-update-stage-*` files. Timestamp and Windows audit-policy preservation are not implemented;
files that require them need manual handling. Native and process-interruption/power-loss acceptance remains pending.
Recovery does not undo application-code effects, databases or deployments.

### `cargo rullst pkg <action> [name]`
Adds or lists `rullst-*`/`rullst_*` crate dependencies in the local
`Cargo.toml`. It executes no package code, queries no registry and changes no
application routing; Cargo resolves the crate on the next build.
* **Subcommands:**
  * `add <package_name>`: Adds the dependency (e.g., `cargo rullst pkg add rullst-auth`) to `Cargo.toml` with this CLI's version as its requirement; an existing entry is left unchanged. In a virtual workspace manifest it adds the entry to `[workspace.dependencies]`, for members to use with `{ workspace = true }`.
  * `list`: Scans and lists all active `rullst-*` community extensions installed in your project (the workspace dependencies of a virtual workspace manifest).

An unknown action, or `add` without a package name, fails with a non-zero exit
status.

### `cargo rullst add`
`cargo rullst add <capability> [--dry-run]` (v13) enables a facade capability
in an existing project. Capabilities: `mail`, `auth`, `ai`, `nexus` and
`studio`.

It runs only at the root of a Rullst project (a `Cargo.toml` whose
`[dependencies]` declare `rullst`), asks nothing and:

1. enables the capability's `rullst` feature in `Cargo.toml`. The edit keeps
   comments, formatting and the other features; a feature that already
   implies it (for example `mailer` or `auth-sqlite`) counts as enabled;
2. appends the capability's variables to `.env.example` under a
   `# ── <Capability> (added by cargo rullst add …) ──` heading, creating the
   file when it is missing. Values are placeholders, empty or `mock_*` (the
   offline mocks of AGENTS.md 3.5), never real secrets; a variable that is
   already assigned or commented out is not repeated. `.env` is changed only
   when it lacks a variable the application needs to start in development,
   and then receives the same mock value and is reported; none of the current
   capabilities needs one, so `.env` stays as it is;
3. prints the wiring code to paste and where it goes. Generated projects have
   no marked insertion points, so `add` never edits application source;
4. prints one to three next steps.

| Capability | `rullst` feature | `.env.example` | Next steps |
| :--- | :--- | :--- | :--- |
| `mail` | `mail` | `MAIL_FROM`, `# MAIL_DRIVER`, `# RESEND_API_KEY` | `make:mail Welcome`, `dev` |
| `auth` | `auth` | `APP_KEY` placeholder | `auth`, `make:mfa` |
| `ai` | `ai` | `OPENAI_API_KEY=mock_openai_key`, commented Anthropic, Gemini and Ollama entries | `make:chat-session`, `ai connect` |
| `nexus` | `nexus` | empty `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` (required by release builds unless Nexus uses the application's sign-in, `NexusAuthPolicy::application`) | `dev`, then `/nexus` |
| `studio` | `studio` | none | `dev`, then `http://127.0.0.1:5555` |

Running it again changes nothing and reports that the capability is already
enabled. `--dry-run` prints the planned diff of every file and writes nothing.
Outside a project it fails with the shared error report (exit status 1); an
unknown capability is a usage error (exit status 2).

```bash
cargo rullst add mail --dry-run
cargo rullst add mail
```

---

## 🛠️ 2. Architecture Scaffolding (`make:*`)

Rullst generators write the files described under each command. Some commands
also register modules and refresh `.llms.txt`; this is command-specific, and a
failed best-effort context refresh does not roll back generated source.
`make:controller`, `make:model`, `make:middleware`, `make:worker`,
`make:island`, `auth`, `make:billing`, `make:cors` and `make:jwt` also refresh
the `generate:diagram` output `diagram.md`: a missing file is created, while a
`diagram.md` the generator did not write is kept and reported instead of
replaced. Review the diff and run `cargo check` after scaffolding.

### `cargo rullst make:resource <name>`
Scaffolds the bounded starting files for a CRUD resource in one command: a
Model (`src/models/<name>.rs`), Migration
(`src/migrations/m<timestamp>_create_<plural>.rs`, for example
`create_categories` for `Category`), Controller
(`src/controllers/<name>_controller.rs`), and HTML view placeholders
(`views/<name>/index.html` and `views/<name>/form.html`). It does not infer
application fields, register routes, establish ownership/RBAC, or turn the
placeholder handlers into a complete authorized CRUD implementation. Like
`make:model --migration`, it keeps an existing model and does not add a second
create-table migration for it. Mount the
routes behind the canonical security baseline, render request-scoped CSRF
tokens in state-changing forms, complete validation/persistence, and run the
application's authorization-negative tests.
* **Arguments:** `<name>` (e.g., `Product` or `product`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API resource controller instead of HTML views.

### `cargo rullst make:controller <name>`
Generates `src/controllers/<name>_controller.rs` (`posts` and
`PostsController` both produce `posts_controller.rs`) with placeholder CRUD
handlers (`index`, `show`, `store`, `update`, `delete`). It registers the
module in `src/controllers/mod.rs` and adds `pub mod controllers;` to
`src/main.rs` when that file exists; it does not add application routes
automatically.
* **Arguments:** `<name>` (e.g., `UsersController` or `users`). A name whose
  module or type would not be a non-keyword Rust identifier (for example
  `Bad.Name`) is rejected before any file is edited.
* **Optional Flags:**
  * `--api`: Instead of returning HTML Views via the `html!` macro, the generated methods will automatically extract/return `Json<T>`. The command prints the `routes!` lines (`/api/<name>` and `/api/<name>/{id}`) and the `MachineEndpoint::bearer` lines for `machine_endpoints()` in `src/main.rs`: in staging and production JSON clients reach the write routes only as machine endpoints, or the browser CSRF check refuses them with 403 (see [which security layer to use](security-layers.md#what-generated-applications-already-have)).

### `cargo rullst make:model <name>`
Creates a model struct in `src/models/` with the ORM annotations. SQLx projects
receive `FromRow` plus `Orm` and import only those two derives (`use
rullst::db::{FromRow, Orm};`), so the new file compiles without warnings;
Turso-primary projects receive
`#[derive(rullst_orm::Orm)] #[orm(backend = "turso")]` and an `i64` primary
key. Backend detection reads the generated manifest and does not treat an
additive `--turso` integration as the primary ORM. Like `make:resource`, it
rejects a name whose module or type would not be a non-keyword Rust identifier
(for example `Match`, which would declare `pub mod match;`) before writing.
* **Arguments:** `<name>` (e.g., `BlogPost`).
* **Optional Flags:**
  * `--migration` or `-m`: Simultaneously generates a reversible migration with the correctly pluralized table name. An existing model file is kept, and the migration is skipped when the model already existed or a `*_create_<table>.rs`/`*_create_<table>_table.rs` migration exists, since a second create migration would drop the live table on rollback.

### `cargo rullst make:chat-session`

Adds application-owned conversational memory for the project's primary ORM.
It generates and registers `ChatSession` and `ChatMessage`, a reversible
migration, and `StatefulChat`. SQLx and the bounded Turso-primary profile receive
backend-specific code; the command also enables the `orm` and `ai` umbrella
features if necessary.

```bash
cargo rullst make:chat-session
cargo rullst db:migrate
```

Save the generated `ChatSession` before constructing `StatefulChat`. Each
service instance serializes concurrent sends, restores at most the newest 100
messages in chronological order, persists the user message before provider
dispatch and persists the assistant response only after success. Database and
provider failures are returned as `StatefulChatError`; they are never silently
discarded. Multi-process ordering, tenant authorization, retention and deletion
remain application responsibilities. The command refuses to overwrite an
existing chat scaffold.

### `cargo rullst make:middleware <name>`
Generates a standard Axum/Rullst Middleware struct in `src/middlewares/`. Perfect for injecting headers, checking authentication, rate limiting, or logging.
A name whose module and function would not be a non-keyword Rust identifier is
rejected before any file is edited.

### `cargo rullst make:island <name>`
Creates a frontend interactive "Islands Architecture" component (similar to Fresh or Astro) in `src/islands/`. It generates the Rust infrastructure that, during build, will be transparently compiled to WebAssembly to run in the browser.

### `cargo rullst make:worker <name>`
Creates an asynchronous background worker in `src/workers/` against the queue
backends currently implemented in Core (memory, SQLite, and optional Redis).
RabbitMQ is not generated by this command. The generated handler logs only the
job name and its number of payload fields, never the payload itself, because
jobs often carry addresses, tokens or other personal data.

### `cargo rullst make:migration <name>`
Generates a timestamped reversible Rust migration for the project's primary
backend. SQLx projects use the schema DSL; Turso-primary projects use
`TursoMigration` and parameterized `TursoStatement` values, and regenerate a
fallible typed migration registry. The name is lowercased with `-` mapped to
`_` and otherwise kept intact (`modify_users_email` produces
`m<timestamp>_modify_users_email`); names with other characters, such as
`add_index.v2`, are rejected. Regenerating `src/migrations/mod.rs` fails, naming
the file, when an `m*.rs` file there is not a valid Rust module name.

`cargo rullst make:migration:auto` (SQLite `DATABASE_URL` in `.env` only)
compares the `#[derive(Orm)]` models under `src/` with the database. It ignores
framework tables (`migrations` and `rullst_*`) and writes a migration only for
additive changes (new tables or columns), with drops of model-less tables or
columns included as commented-out code for review. When only such destructive
differences remain, it lists them and writes no migration. Column types follow
the field's Rust type: `i8`/`i16`/`i32` use `integer`, `i64` `big_integer`,
`f32`/`f64` `float`, `bool` `boolean`, and `String`, `SecretString`, chrono
date/time types and `Json` use `string`. A new table declares non-`Option`
fields `NOT NULL`. A required field added to an existing table is declared
`NOT NULL DEFAULT 0`, `0.0` or `''` (a commented backfill to review); a
required date, encrypted or JSON field added to an existing table has no
neutral value and is refused. Any other type (for example `Vec<u8>`, `Uuid` or
an application enum) is refused, naming the field, and nothing is written; add
such columns with `make:migration`.

### `cargo rullst make:billing`
Scaffolds a SaaS billing starting point with subscription models, authenticated
billing routes, and signed-webhook integration points. Provider credentials,
tenant policy, and deployment behavior still require application configuration.

Empty or `mock_*` credentials select a local development fixture, which is
refused in production. Other credentials select the real Stripe profile
(`BILLING_PROVIDER=stripe`, the only accepted value), which creates provider
customers, checkout sessions and portal sessions, persists the
owner/customer/attempt bindings and processes signed webhooks atomically. It
requires `BILLING_ACCOUNT_ID=acct_...`, an `sk_test_`/`rk_test_` or
`sk_live_`/`rk_live_` `BILLING_API_KEY`, a strong `BILLING_WEBHOOK_SECRET` and
an HTTPS `BILLING_REDIRECT_URL`; live keys additionally require
`BILLING_LIVE_ACKNOWLEDGEMENT=I_UNDERSTAND_REAL_CHARGES`. To bill through
another gateway, implement the Capital provider traits in application code; see
[Writing your own payment provider](capital-custom-provider.md).

Mixed mock/real credentials, incomplete configuration and any other
`BILLING_PROVIDER` value return HTTP 503. The generated `BILLING.md` lists the
permissions, webhook events, recovery procedures and remaining limits.

Hosted checkout also requires the submitting page's CSP to allow its exact
reviewed destination in `form-action`. The SaaS starter selects Stripe and
generates `form-action 'self' https://checkout.stripe.com` while retaining the
rest of Core's strict policy. `make:billing` prints this requirement and leaves
your existing policy for review. A custom provider requires its exact checkout
origin; do not allow `https:` or wildcard domains. Keep a single `form-action` directive in `security.csp` and
review proxy/CDN policies too: another restrictive CSP still applies.

Independently validate each returned URL (HTTPS, exact host/port, no embedded
credentials) and the session's owner, product and test/live mode before a 303.
Test the form submission in a real browser: a successful HTTP redirect alone
does not prove that CSP permits navigation. Before enabling live billing,
resolve an owner's persisted open attempt before charging the new-session
quota; resume only a retrieved, fully bound open session. Expired, completed
and uncertain outcomes require separate handling and reconciliation.

### `cargo rullst make:mail <Name>`
Scaffolds a registered transactional mailable. `--welcome`, `--reset`, `--otp`
and `--invoice` select the bounded built-in variants (at most one); without a
flag the command generates a custom message type. It enables the umbrella
`mailer` feature, uses the `rullst::mail` facade, escapes dynamic HTML and
refuses invalid identifiers, path traversal or an existing target. Generated
mailables set no `from`, so the facade uses the `MAIL_FROM` (or `[mail] from`)
default sender, which new projects list in `.env.example` next to a
`MAIL_DRIVER` hint; staging and production must select a driver before
sending. Delivery credentials, URL
semantics, tenant policy and provider operation remain application
responsibilities.

### `cargo rullst make:mail-invoice [Name]`

Generates `PaymentReceiptEmail` by default and enables `mailer`. The result is
a bounded commercial payment receipt (`international_receipt`, optional
`with_document_url`) that states it is not a tax authorization; it runs the
mail pre-flight before returning the message. A custom valid struct name may be
supplied positionally.

### `cargo rullst make:mail-dunning [Name]`

Generates `PaymentDunningEmail` by default with explicit gentle D+1,
action-required D+3, and service-status D+7 stages. The application remains
responsible for calculating the due state, scheduling delivery, enforcing its
disclosed billing policy, and reconciling payment. The generated build path
runs the mandatory pre-flight and rejects dangerous links.

<a id="cargo-rullst-makeage-gate-unpublished-v13-preview"></a>

### `cargo rullst make:age-gate` (v13 prerelease preview)

Adds an explicit first-party declaration before each visit to the recognized
SaaS starter's `/dashboard`. Generation requires a server-owned policy version,
threshold and single-tenant deployment reference; it never chooses a universal
legal minimum age. The existing authentication and CSRF layers protect both
GET and POST. A successful answer remains declared, and authorizes only that
dashboard rendering after durable one-use consumption.

```bash
cargo rullst make:age-gate \
  --privacy-source /path/to/Rullst/rullst-privacy \
  --minimum-age 18 \
  --policy-version dashboard-v1 \
  --tenant-ref application-tenant-ref \
  --replay-store sqlite
```

The threshold above is an example for an application-assessed low-assurance
policy. Select `postgres` explicitly for a shared database across hosts.
Configure the required private key and replay database as described by the
generated `AGE_GATE.md`; missing state or configuration denies access. The
candidate defaults to the registry version matching this CLI. The source
override shown above is required for development before that version is published,
unless an explicit archive-only patch is configured. Generation refuses unknown
privacy dependencies or unrecognized authentication/route shapes before writing.
It composes with `make:privacy` when both use the same registry/local source;
both consumers' explicit dependency features are preserved.
Review the generated diff before deployment. It does not install facial models,
verified guardianship, reusable age flags or global compliance.

It targets the recognized SaaS starter (`--blueprint saas`, the default) and
requires `--tenant-ref`. v13 removed the LMS target together with the complete
Academy scaffold. The tenant is compiled into the generated configuration; no
query parameter, header or form field is read to select it.

<a id="cargo-rullst-makeprivacy-unpublished-v13-preview"></a>

### `cargo rullst make:privacy` (v13 prerelease preview)

Adds authenticated preferences at `/privacy`, an optional personalized greeting
at `/privacy/personalization`, and a private own-account JSON download at
`/privacy/export`. The concrete export projects only the authenticated account's
ID, name and email; it does not complete broader queued privacy requests or
export subscription, learning, guardian, backup or processor records.

```bash
cargo rullst make:privacy \
  --privacy-source /path/to/Rullst/rullst-privacy \
  --purpose-version greeting-v1 \
  --validity-seconds 86400 \
  --tenant-ref application-tenant-ref
```

The version and lifetime are explicit application choices; the engineering cap
of 365 days is not a legal retention rule. It targets the recognized SaaS
starter and requires `--tenant-ref`; v13 removed the LMS target. The tenant is
compiled into the generated configuration. The privacy routes take no query
string: a request carrying one, or an `x-school-id` header, is refused rather
than allowed to select another tenant or account.
The generated consumer composes with `make:age-gate` in either installation order.
Omitting `--privacy-source` selects this CLI's matching registry version; before
publication use the explicit local override shown above or a reviewed archive patch.
Unknown authentication, dependencies, routes or existing output files require
manual integration rather than overwriting application code.

Follow generated `PRIVACY.md`: provision an independent random
`RULLST_PRIVACY_FORM_KEY_HEX` and initialize a new private local consent file with
`cargo run --bin privacy-init`, with `RULLST_PRIVACY_DATABASE` exported into that
process environment. Ordinary opening never creates or repairs missing state.
The generated 10,000-record SQLite store requires one trusted shared local file;
it does not provide multi-host replication. Preferences and personalization deny
unavailable state, while the authenticated export remains independent of that
store and form key.

Choices are initially unselected. Refusal and withdrawal produce a generic
greeting; an earlier positive form cannot undo a completed withdrawal. Forms
bind the displayed notice, revision, account, tenant and session and expire after
five minutes. The server checks current permission before the optional name
query. Add visible application navigation and review the documented backup,
retention and broader rights obligations before deployment. This bounded
consumer does not establish worldwide legal compliance or verify age.

### `cargo rullst make:jwt`
Injects a pre-configured boilerplate Middleware into your project for strict JWT Authentication (verifying Bearer tokens in the `Authorization` header).

### `cargo rullst make:cors`
Generates and configures full CORS (Cross-Origin Resource Sharing) options in your project with recommended security defaults (blocking unused methods, restricting origins).

Projects generated by older CLI versions retain the middleware that was copied
into their source tree and must be reviewed manually. Follow the
[CORS scaffold security advisory](cors-scaffold-security-advisory.md) to detect
origin reflection/wildcards and migrate to the current fail-closed allowlist.

### `cargo rullst make:pwa` (13.0.0-alpha.3)

Makes an HTML application an installable Progressive Web App and wires it into
the starter's own router and page layouts:

```bash
cargo rullst make:pwa
cargo rullst make:pwa --name "Field Notes" --short-name Notes \
  --theme-color '#0f172a' --background-color '#020617' --lang pt-BR --start-url /
```

| Flag | Default | Meaning |
| :--- | :--- | :--- |
| `--name` | `[package].name` in words (`field-notes` → `Field Notes`) | 1-64 letters, digits, spaces, `.`, `-`, `_` |
| `--short-name` | the name when it has at most 12 characters, else its first word cut to 12 | label under the home-screen icon |
| `--theme-color`, `--background-color` | `#0f172a`, `#020617` | `#RRGGBB` |
| `--lang` | `en` | language tag such as `en` or `pt-BR` |
| `--start-url` | `/` | a path on this site that the installed app opens |
| `--android` | off | also write `android/twa-manifest.json` (Bubblewrap) and `PWA-ANDROID.md`; needs `--host` and `--package-id` |
| `--host` | none | with `--android`: the site's HTTPS host, such as `notes.acme.dev` |
| `--package-id` | none | with `--android`: the Android application ID you own (`com.example.*` is refused) |

It writes `static/manifest.webmanifest` (`id` and `scope` `/`, `display`
`standalone`, icons 192 and 512 plus a maskable 512), four placeholder PNG
icons in `static/icons/` (the Rullst logo; replace them), `static/pwa.js`,
`src/pwa.rs`, the worker in `src/pwa/sw.js` and `PWA.md`. `src/pwa.rs` serves
`/sw.js` (`text/javascript`, `Cache-Control: no-cache`, scope `/`) and
`/offline` (`no-cache`, so the worker may store it), and its `head()` returns
the manifest link, `theme-color`, the apple-touch icon and the registration
script. The existing source is edited in three recognized places:

- the router: `.merge_axum(crate::pwa::router().into_axum())` follows the one
  `.merge_axum(rullst::health::health_router())` of the crate root (a merge in
  a `return` expression, such as the fallback router of an application
  generated from the former ERP starter, does not count);
- every page layout in `src/`: `{crate::pwa::head()}` before the `</head>` of
  an `html!` document, the same tags before the `</head>` of a document
  written in a string, and `render_page_with_lang(...)` calls become
  `render_page_with_head(..., crate::pwa::head(), content)`;
- the crate root (`src/lib.rs`, else `src/main.rs`) declares `mod pwa`.

The registration script is a same-origin file, so the default
Content-Security-Policy (`script-src 'self' 'nonce-…'`, `worker-src 'self'`,
the manifest under `default-src 'self'`) allows it without a nonce. The worker
precaches `/offline` and the PWA's static files, goes to the network first for
navigations and unversioned static files, serves content-hashed static files
from the cache first, and never stores a non-GET request, another origin or a
response marked `no-store` or `private`; Rullst answers `no-store` by default,
so authenticated pages stay off the device. A new worker waits until the user
chooses **Reload** in the page's update prompt (`SKIP_WAITING`). The cache is
named after `[package].version`.

The generated router also serves `/.well-known/assetlinks.json` (Digital
Asset Links for the Google Play package) once `RULLST_ANDROID_ASSET_LINKS` or
`[[android.asset_links]]` in `Rullst.toml` names the app and its signing
certificate fingerprints; see `rullst::pwa::AssetLinks`.

The whole plan is checked before any write. Running the command again changes
nothing; a generated file that now differs (edited, or another `--name`) stops
the run without writing. `--android` can follow a plain run with the same
flags and adds only the Android files. The JSON API starter, a project without an HTML
layout, and a router without the health merge are refused; wire such a project
by hand as the [PWA guide](tutorials/43-progressive-web-apps.md) shows.

### `cargo rullst make:iot <DeviceName>`
**Maturity:** Experimental · [tiers](maturity.md#cli-generators)

Scaffolds and registers a telemetry-only IoT module in `src/iot/` using the
public `rullst::iot::SensorTelemetry` facade, and enables the `iot` feature in
the application manifest. Unsafe identifiers/path traversal and existing target
files are rejected. It does not install an MQTT/CoAP transport, HAL, firmware,
or claim broker connectivity.

### `cargo rullst make:k8s`
Scaffolds cloud-native Kubernetes manifest files in the `k8s/` directory (`deployment.yaml`, `service.yaml`, `configmap.yaml`, `hpa.yaml`, `ingress.yaml`, and `all-in-one.yaml`) pre-configured with liveness (`/health`) and readiness (`/ready`) HTTP probes.
The command fails before writing anything when any of these manifests already
exists, and it does not write through a symlinked `k8s/` directory or file; move
customized manifests aside to regenerate the templates.
Object names, the image reference and the ingress host use the `[package]`
name as a lowercase RFC 1123 label: characters other than letters and digits
become `-` (`my_app` becomes `my-app`), and the label is capped at 55
characters so suffixed names such as `<name>-service` stay valid.

### `cargo rullst make:scalar`
Scaffolds a Scalar API Documentation controller at
`src/controllers/docs_controller.rs`. The interactive view loads a pinned CDN
asset; its local fallback is status-only and final CSP/network policy belongs to
the application.

### `cargo rullst make:live <ComponentName>`
Scaffolds a LiveView-style server component at `src/live/<name>.rs` using a
WebSocket and HTMX out-of-band swaps. Application JavaScript may be unnecessary,
but HTMX remains client-side JavaScript and the generated transport requires
origin, reconnect, and backpressure review. It must run in a Rullst project
root, and a name whose module or type would not be a non-keyword Rust
identifier (for example `../notes`, `self` or `Bad.Name`) is rejected before
anything is written.

### `cargo rullst deploy`
Writes the files that run the application on any Linux VM with Docker or
Podman, each only when it is absent (existing files are kept, and nothing is
written through a link):

* `Dockerfile` and `.dockerignore`: the same multi-stage build as
  `dockerize`, with its dependency-cache (planner) stage and a distroless,
  non-root runtime image;
* `compose.yaml`: the app and a Caddy reverse proxy with automatic HTTPS for
  `DOMAIN`. Both containers have a read-only root filesystem, every capability
  dropped (Caddy keeps only `NET_BIND_SERVICE` for ports 80 and 443) and
  `no-new-privileges`; the app reads `.env.production` and keeps its data in
  the `app_data` volume; the app's `/health` endpoint is checked from the
  Caddy container, because the app image has no shell or HTTP client. It works
  with `docker compose` and `podman compose`;
* `Caddyfile`: proxies to the app and stops routing to it while `/health`
  fails.

Caddy is pinned to `172.31.250.10` on a `172.31.250.0/24` compose network,
and the command adds that address to `[security] trusted_proxies` in
`Rullst.toml` (before a missing `Dockerfile` is scaffolded, so the image
copies it), so rate limits see each client instead of Caddy. An existing
`trusted_proxies` list is left unchanged with a warning. The command prints
the next steps: create `.env.production`, point DNS at the VM, run
`DOMAIN=example.com docker compose up -d --build` (or
`podman compose up -d --build`) and run `db:migrate` once per release.
Nothing is uploaded, started or deployed. A `Cargo.toml` package name that is
not a valid Cargo name is refused before anything is written, because the
Dockerfile names the binary after it.

Step by step: [Deploy to a Linux VM](vm-deployment.md), and the guides for
[Hetzner Cloud](deploy-hetzner.md), [DigitalOcean](deploy-digitalocean.md),
[Oracle Cloud](deploy-oracle-cloud.md) and [Google Cloud](deploy-google-cloud.md).

**Removed in 13.0.0-alpha.4:** the Fly.io, Railway and Render targets
(`--platform fly|railway|render` now fails before writing anything;
`--platform vps`, the former name of this path, is still accepted). Provider
files you already have keep working with the provider's own CLI. Nix,
Buildah, Podman Quadlet and Kubernetes are described in
[Advanced deployment](advanced-deployment.md). Credentials, migrations,
availability, DNS and rollback remain operator responsibilities.

### `cargo rullst auth`
Creates an authentication starting point in your codebase, including:
- User model and migration with asynchronous Argon2 password hashing and a
  `session_epoch` column.
- Auth Controllers (Login, Registration, Logout, and `logout_everywhere`, which
  increments the account's session epoch to end its session on every device).
- Encrypted-session middleware that inserts the signed-in user's id as
  `Extension<i32>` and refuses a cookie whose session epoch is no longer the
  account's.
- A password controller (`src/controllers/password_controller.rs`), a reset
  token model (`src/models/password_reset.rs`) and a
  `password_reset_tokens` migration: **change password** (signed in; checks the
  current password, keeps only the asking browser signed in) and **reset by
  email** (a 30-minute single-use token stored as a SHA-256 digest, one live
  token per account, the same answer for unknown emails; a reset signs out
  every device and redirects to `/login?reset=done`). Reset links use
  `RULLST_PUBLIC_ORIGIN`, never the request's `Host` header. Account emails
  are written in the request's language (the `rullst::i18n` locale, else
  `Accept-Language`) and name the product `APP_NAME` or the editable
  `APPLICATION_NAME` constant.
- HTML Views for Login, Signup, Forgot password, Reset password and Change
  password.

It targets the SQLx ORM: Turso-primary projects are rejected. The command
enables the `orm`, `auth` and `mail` umbrella features, registers the generated
`controllers`, `middlewares`, `models` and `pages` modules in `src/lib.rs` (or
`src/main.rs`), and refreshes the migration registry. It fails before writing
anything when `src/models/user.rs`, `src/models/password_reset.rs`,
`src/controllers/auth_controller.rs`, `src/controllers/password_controller.rs`,
`src/middlewares/auth_middleware.rs` or `src/pages/auth.rs` already exists,
when a `*_create_password_reset_tokens_table.rs` migration exists, or
when a `*_create_users.rs`/`*_create_users_table.rs` migration already creates
the users table (the blank database starter and the SaaS starter ship one). It also refuses a project without a SQL migration runner (`pub mod
migrations;` in `src/main.rs` or `src/lib.rs` plus a
`rullst::artisan!(crate::migrations::get_migrations())` call, which database
starters include), such as a `--no-database` starter, because nothing would
compile or apply the users migration. Generated code names the ORM through the
`rullst::orm` facade, so no direct `rullst-orm` dependency is needed. Mounting
routes (including `POST /logout/everywhere` and `/password/change` behind
`auth_middleware`, and `credential_rate_limit` on the password submissions),
the security baseline and the mail settings (`MAIL_DRIVER`, `MAIL_FROM`,
`RULLST_PUBLIC_ORIGIN`) remain application work, as does calling
`User::revoke_sessions` when you deactivate an account. The SaaS starter and
the LMS example (`examples/lms`) ship the same password flows already mounted.

### `cargo rullst make:mfa`
Scaffolds a server-side RFC 6238 TOTP second factor: `src/controllers/mfa.rs`
and a reversible `user_mfa_factors` migration (one factor per account). The
secret is generated and stored on the server and bound to the signed-in
account: `mfa_setup`, `mfa_confirm` and `mfa_verify` take the user id from the
`Extension<i32>` that the `cargo rullst auth` middleware inserts, and a client
never submits a secret. Verification uses `verify_totp_step_after` with an
atomic conditional update of `last_accepted_step`, so each code is accepted at
most once. Setup returns the secret and `otpauth://` URI once with
`Cache-Control: no-store`; enrollment stays pending until `mfa_confirm`
accepts a current code.

The command targets the SQLx ORM (Turso-primary projects and projects without
a migration runner are rejected, as for `cargo rullst auth`), enables
the `orm` and `security` umbrella features, registers the module, refreshes the
migration registry and refuses to overwrite an existing `src/controllers/mfa.rs`
or `*_create_user_mfa_factors_table.rs` migration. Mount the handlers as POST
routes behind the authentication middleware, CSRF protection and rate limiting,
and require a recent password check before enrollment. To gate login, keep the
session pending until `verify_second_factor` succeeds; the generated auth
controller issues a full session after the password. Secrets are stored
unencrypted in the database, so protect that table and its backups like
credentials. Recovery codes and factor reset remain application work.

<a id="cargo-rullst-makeoauth-v13-prerelease-preview"></a>

### `cargo rullst make:oauth <provider>` (v13 prerelease preview)

Adds sign-in with Google, GitHub, Microsoft or Apple to the account model of
`cargo rullst auth` or of the SaaS starter (and of applications generated from
the former LMS starter, such as `examples/lms`). Run it once per provider;
other providers are refused until they have a generator and tests.

```bash
cargo rullst make:oauth google      # or github, microsoft, apple
cargo rullst db:migrate             # once, after the first provider
```

The first provider writes `src/controllers/oauth/mod.rs` (shared flow, account
linking and sign-in), `src/models/oauth_identity.rs` and an
`oauth_identities` migration (provider and subject unique, the user id with
`ON DELETE CASCADE`, created at). Every provider writes
`src/controllers/oauth/<provider>.rs` (settings and the `/auth/<provider>`
and `/auth/<provider>/callback` handlers), registers it in the shared module
and documents its settings in `.env.example`; a second provider adds no
migration and shares the table, the flow and the linking policy. The command
enables the `oauth` umbrella feature and the challenge store of the project's
database (`oauth-sqlite`, `oauth-postgres` or `oauth-mysql`) and adds an
application feature `oauth-mock = ["rullst/oauth-mock"]`. In the SaaS starter
and LMS-starter applications it also mounts the routes after the password login, with the same
`credential_rate_limit`, calls `controllers::oauth::<provider>::initialize()?`
at the start of `router()` and adds a "Sign in with …" link to the login page;
for a `cargo rullst auth` project, whose router is the application's, it
prints the routes to mount.

The flow keeps the OAuth state, PKCE verifier and OIDC nonce in
`rullst_connect_login_challenges` (created on first use) and only an opaque id
in the `rullst_oauth_challenge` cookie (ten minutes, `HttpOnly`,
`SameSite=Lax`, `Path=/auth/<provider>`, `Secure` in staging and production);
see [Tutorial 42](tutorials/42-server-bound-oauth-sessions.md#store-backed-login).
The callback consumes the challenge once, checks the state and the PKCE
verifier and, for OpenID Connect providers, the ID token and its nonce, then:

- an identity already in `oauth_identities` (provider and subject) signs in to
  its account;
- otherwise the provider must have verified the email; an unverified email is
  refused;
- a verified email that belongs to no account creates one without a password,
  linked to the identity;
- a verified email that already belongs to an account is refused by default
  (`EXISTING_ACCOUNT_POLICY = ExistingAccountPolicy::Refuse` in
  `src/controllers/oauth/mod.rs`), because registration does not prove that
  whoever registered the address owns it. `LinkAndResetPassword` links it
  instead, removes the account's password and signs out every device in the
  same transaction. The policy applies to every provider, including a second
  provider of an account that signed in with the first.

What counts as the subject and as a verified email differs per provider:

| Provider | Subject | Verified email |
| :--- | :--- | :--- |
| `google` (OpenID Connect) | ID token `sub` | `email` with `email_verified: true` |
| `github` (OAuth 2.0) | numeric account id of `GET /user`, never the login name | the `primary` address of `GET /user/emails` marked `verified`; the default `user:email` scope grants it |
| `microsoft` (OpenID Connect) | `<tid>:<oid>` (directory and object IDs) | `email` only with the optional claim `xms_edov: true`, which the app registration must request with `email` |
| `apple` (OpenID Connect, form POST) | ID token `sub` | `email` with `email_verified`; possibly a private relay address |

Sign-in issues the password login's cookie, `make_login_cookie_with_epoch` with
the account's current session epoch; an account whose session epoch the
password login refuses is refused the same way. The page to return to comes
from `?next=` on `/auth/<provider>`, and only a same-site absolute path is
kept.

Settings (process environment, then `.env`); setting none of a provider's
credentials turns it off and hides its link, a partial set stops startup:

- Google: `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`.
- GitHub: `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` of an OAuth App.
- Microsoft: `MICROSOFT_TENANT` (required with the credentials: `common`,
  `organizations`, `consumers` or a directory ID GUID; there is no default,
  because it decides who can create accounts), `MICROSOFT_CLIENT_ID`,
  `MICROSOFT_CLIENT_SECRET`.
- Apple: `APPLE_CLIENT_ID` (the Services ID), `APPLE_TEAM_ID`,
  `APPLE_KEY_ID` and `APPLE_PRIVATE_KEY`, the PEM text of the `.p8` key
  (newlines may be written as `\n`). Rullst Connect signs a five-minute ES256
  client secret with it for each code exchange; a key that cannot sign stops
  startup. Keep it in the environment or a secret store: the command never
  writes it to a file.
- `<PROVIDER>_REDIRECT_URL` defaults to `RULLST_PUBLIC_ORIGIN` followed by
  `/auth/<provider>/callback`, and to `http://localhost:3000/...` in
  development; staging and production without either stop at startup.
- `mock_*` values select the offline provider, which signs in
  `mock@example.invalid` without leaving the application; the command prints
  the exact settings, such as `GITHUB_CLIENT_ID=mock_github
  GITHUB_CLIENT_SECRET=mock_github cargo run --features oauth-mock`. Without
  that feature the application stops at startup with an error naming it, and
  staging and production refuse mock credentials.

Apple POSTs its callback from `appleid.apple.com`, so `make:oauth apple`
mounts `POST /auth/apple/callback` (plus a `GET` that only the offline mock
uses), makes the challenge cookie `SameSite=None; Secure`, and lists
`/auth/apple/callback` in `[security] csrf_signed_webhook_paths` of
`Rullst.toml` (creating the file when missing): instead of the CSRF token,
the handler requires the `state` of the login challenge that the browser's
challenge cookie names. Its name arrives only on the first authorization, in
the unsigned `user` form field, and only names a new account. Register the
`MAIL_FROM` domain with Apple's Private Email Relay service so account emails
reach relay addresses.

The command requires a strict single-database profile (`strict-sqlite`,
`strict-postgres` or `strict-mysql`, as `cargo rullst new --database ...`
selects), the `User` model with `session_epoch`, `current_session_epoch` and
`find_by_email`, the generated `auth_controller` helpers, `src/pages/auth.rs`
and a migration runner. It refuses Turso-primary projects, and it fails before
writing anything when any output exists or the provider is already installed.
It does not edit `auth_controller.rs` or `user.rs`.

---

## 🗄️ 3. Database and Migrations (`db:*`)

Every `db:*` command (including `cargo run -- db:migrate` and a deployed
binary's `db:migrate` job) and `studio` select the database exactly like
`Server`: the process `DATABASE_URL`, then `DATABASE_URL` in `./.env` (which
never overrides a variable already set in the process), then `[database].url`
in `Rullst.toml`, read with a TOML parser. A `Server::with_db` URL applies when
`Server::run` intercepts the command. Without a configured database, a `db:*`
command exits with status 1 instead of creating a SQLite file. Configuration
errors (reported without file content) and database initialization failures
also exit with status 1.

### `cargo rullst db:migrate`
Runs `cargo run -- db:migrate`, which the application's
`rullst::artisan!(crate::migrations::get_migrations())` call intercepts. It
applies, in one new batch, every migration registered in
`src/migrations/mod.rs` whose name is not yet recorded in the ORM's
`migrations` table, calling each migration's `up`.

### `cargo rullst db:rollback`
Reverts the last batch: it calls `down` for each migration recorded in the
latest batch, newest first, and removes their records.

### `cargo rullst db:status`
Lists every registered migration as `Applied` or `Pending` according to the
`migrations` table.

### `cargo rullst db:seed`
Runs the seeders (`rullst::db::Seeder` implementations) that `main` passes as
the second argument of `rullst::artisan!(migrations, seeders)`, for example an
initial administrator or development data.

Turso-primary starters handle `db:migrate`, `db:rollback` and `db:status` in
their generated `main.rs` through `TursoOrm` instead of `rullst::artisan!`.

### `cargo rullst studio`
Runs `cargo run -- studio`, which Core's Artisan handler serves as a smaller
local compatibility UI (database, AI, telemetry, security, capital and span
pages) on `127.0.0.1:5555` until it stops; it is not the full `rullst-studio`
router that debug builds of the generated starters start on the same port
(see [Rullst Studio](3-rullst-studio.md)). Treat it as a privileged
development tool; do not expose it publicly without an independently reviewed
authentication, authorization, and TLS boundary.

`cargo rullst studio --remote [--port PORT]` needs no project: it starts the
installed standalone `rullst-studio` binary (`cargo install rullst-studio --features remote --locked`),
which serves Studio's remote-projects mode on `127.0.0.1:5556` (or `PORT`)
and reads the [production monitoring endpoint](production-monitoring.md#watch-your-projects-from-studio)
of the projects you add. Without the binary it explains how to install it.

---

## 🧠 4. Analyzers and Code Generators (`generate:*`)

### `cargo rullst generate:openapi`
Reads recognizable route and Rustdoc patterns and generates an OpenAPI V3 draft.
Dynamic routes, custom extractors, and semantic constraints may require manual
edits; validate the result with an OpenAPI validator before publishing it.

### `cargo rullst generate:ts`
Scans recognizable route declarations in `src/main.rs` and `src/lib.rs` and
emits `rullst-client.ts` with unchecked request/response placeholders. Axum
`{name}` and `{*name}` captures (and legacy `:name` segments) become method
arguments interpolated with `encodeURIComponent` (per segment for a wildcard).
State-changing requests (anything but `GET`, `HEAD`, `OPTIONS` and `TRACE`)
follow the CSRF middleware's double-submit contract: the client echoes the
`rullst_csrf` cookie in the `X-CSRF-Token` header and sends cookies with
`credentials: 'same-origin'`. The cookie is set by a `GET`/`HEAD` response, so
load a page or issue a `GET` before the first state-changing call. Outside a
browser, or for a cross-origin `baseUrl`, pass
`new RullstClient(baseUrl, { csrfToken: () => token, credentials: 'include' })`.
Review the output before use; route scanning does not establish DTO shapes,
serialization or authorization.

### `cargo rullst generate:api` (v13 candidate)
Consumes one explicit bounded OpenAPI 3.1 profile and generates Rust DTOs/codecs,
a typed TypeScript HTTP client and a canonical schema copy. Requires `--schema`
and `--output`; `--check` verifies freshness without writes. Unsupported shapes
fail before generation. See the [profile and executable acceptance](typed-api.md).

### `cargo rullst generate:diagram`
Analyzes primary and foreign keys defined in your Models and exports a `diagram.md` file containing Mermaid.js code, visually generating an Entity-Relationship (ER) diagram.
The file starts with a generator marker comment. An existing `diagram.md` is
replaced only when it carries that marker (or is the single unmarked Mermaid
block earlier releases wrote); the command refuses a hand-written file or a
symlink, so move it aside to regenerate the diagram.

### `cargo rullst generate:models` / `cargo rullst make:models-from-db`
Connects to an existing database and generates reviewable starter structs from
the tables and columns visible in SQLite or the current PostgreSQL/MySQL schema.
Table lookups are parameterized and SQL identifiers are allowlisted.
PostgreSQL and MySQL metadata columns are cast to text and aliased, so
PostgreSQL 12+ `sql_identifier` columns and MySQL 8 upper-case labels are read
portably; a missing or undecodable metadata column fails with an error rather
than a panic. Table module names are normalized, while collisions and database columns that would
require an unsupported ORM field remapping fail before the output directory is
written. Existing model files are never replaced: if any `<table>.rs` target
already exists, the command fails before writing anything. An existing
`mod.rs` keeps its content and receives only missing `pub mod` declarations.
The bounded type mapping falls back to `String`; review keys, relations, custom
types, schema selection and generated files before compiling them.
* **Required Flags:**
  * `--driver`: `postgres`, `mysql`, or `sqlite`.
  * `--url`: The complete connection string.
* **Optional Flags:**
  * `--output`: Where to save the generated structs (Default: `src/models`).

### `cargo rullst generate:ai-context [--check]`
The v13 candidate writes a bounded `.llms.txt` and `.rullst/context-map.json`
with dependency metadata, configuration key names and source paths. It creates
`AGENTS.md` only when absent and preserves existing project instructions.
`--check` detects missing, altered or stale inventory without writing files.
See [project context](project-context.md) for limits, exclusions and legacy migration.

### `cargo rullst ai` (extras)
**Maturity:** Experimental; part of the optional
[`cargo-rullst-extras`](#extras-cargo-rullst-extras) package since
13.0.0-alpha.4.

A terminal assistant that knows Rullst and the current project. It answers
questions and proposes changes that you review one by one. A walkthrough is in
[the terminal AI assistant guide](ai-assistant.md).

```bash
cargo rullst ai connect                # choose provider, model and key (interactive)
cargo rullst ai                        # streaming chat in the current project
cargo rullst ai "add a posts page"     # one goal, then exit
cargo rullst ai status [--json]        # provider, model and key source (never the key)
cargo rullst ai disconnect             # delete the saved credentials file
cargo rullst ai upgrade [--to <VERSION>] [--all-rules] [--dry-run]  # reviewed fixes for upgrade findings
cargo rullst ai fix <ERROR_ID> [--url <URL>] [--dry-run]  # reviewed fix for a panic on the error page
cargo rullst ai review [--staged | --base <REF> | --include-untracked] [--json]  # read-only diff review
```

* **Providers:** OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Ollama and
  a local OpenAI-compatible server (LM Studio, llama.cpp server, vLLM,
  LocalAI, Jan), all through `rullst-ai` and its mandatory
  prompt-injection/PII guardrails. PII masking also applies to your goal and
  shared files (`help@acme.com` is sent as `h***@acme.com`); the CLI notes when
  your message is masked and flags a change that writes a masked value back. Every provider streams its answers except an
  Ollama host that is not a loopback address, which answers at once. A local
  server must listen on loopback (`http://127.0.0.1:...`, `http://[::1]:...`;
  `localhost` is pinned to `127.0.0.1`); its model name must match the model
  the server loaded.
* **Credentials:** a provider environment variable (`OPENAI_API_KEY`,
  `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY`, `OLLAMA_HOST`,
  `RULLST_AI_BASE_URL` for a local server) takes precedence over the user
  credentials file
  `$XDG_CONFIG_HOME/rullst/credentials.toml` (`~/.config/rullst/...`;
  `%APPDATA%\rullst\credentials.toml` on Windows). `connect` writes it
  atomically with mode 0600 in a 0700 directory on Unix, refuses symlinks and
  refuses a location inside the current project or a git work tree. Keys are
  never printed. Provider selection: `--provider`, then the saved provider,
  then the first variable above that is set. Model: `--model`, then
  `RULLST_AI_MODEL`, then the saved model, then the provider default.
  `ai status --json` prints one `rullst.ai-status.v1` object (its `schema`
  field) with the provider, model and credential source.
* **`connect` flags:** `--provider <openai|anthropic|claude|gemini|deepseek|ollama|local>`,
  `--model <name>`, `--api-key-stdin` (read the key from the first line of
  standard input, for scripts), `--host <url>` (Ollama, default
  `http://127.0.0.1:11434`), `--base-url <url>` (local server, default
  `http://127.0.0.1:1234/v1`) and the pair `--input-price-per-mtok <n>`
  `--output-price-per-mtok <n>` (your prices per million tokens, stored with the
  provider and used only for labelled estimates). Without a terminal,
  `--provider` is required; interactively, `connect` also offers to record
  prices.
* **Chat flags:** `--provider`, `--model` and `--dry-run` (show proposed
  actions, never execute them). REPL commands: `/add <path>` shares a project
  file with your next message, `/reset`, `/help`, `/exit`. Ctrl+C cancels the
  current answer; Ctrl+D exits.
* **Offline mode:** with no configured provider, or an empty or `mock_*` key,
  a deterministic offline assistant answers (no network). Inside a project it
  proposes a two-step demo plan (create and edit `rullst-ai-demo.md`); outside
  one it first proposes `cargo rullst new rullst-ai-demo` and continues inside
  it, so the review flow can be tried safely.
* **Actions:** the model may only propose `write_file`, `edit_file` (one exact
  replacement), an allowlisted `cargo rullst` command (`make:*`, `generate:*`
  except `generate:models` and its `make:models-from-db` alias, `db:status`,
  `db:migrate`, `doctor` without `--fix`, `audit` without `--network` or
  `--output` (`--compliance`, `--sbom` and `--report` write a file in the
  project, so they take the checkpoint first),
  `inspect` with no target or `routes`, `models` or `schema`, never a file path) or `cargo check`/`cargo test`. `db:migrate` is refused when the project environment, resolved as the
  application server resolves it (the process `RULLST_ENV`, then the process
  `APP_ENV`, then `RULLST_ENV` or `APP_ENV` from the project `.env`, then
  `[app].env` in `Rullst.toml`), is staging, production or unrecognized. Outside a project the
  only action is `cargo rullst new <name> --default` with optional
  `--blueprint`, `--database`, `--no-database`, `--api`, `--ai`, `--redis` and
  `--skip-initial-migration` (`--default` is added when missing); the name must
  be a new lowercase directory in the current directory, and after it succeeds
  the session continues inside the new project. `new` and `db:migrate` are
  always confirmed one by one, even after `a`. Paths must stay
  below the project root (nearest `Cargo.toml`): no `..`, absolute paths,
  symlinks, `.git/`, `target/`, `.cargo/`, `.env*` (except `.env.example`),
  credentials, keys, `Cargo.lock` or toolchain files, and no name a filesystem
  could map onto one of them (Windows 8.3 aliases such as `GIT~1`, reserved
  device names, trailing dots or spaces, characters HFS+ ignores). Changes to
  build files, `.github/`, `tests/`, `benches/`, `examples/` and Rust code with
  tests or procedural macros are flagged in the review. Each action shows a
  coloured diff or the exact command and asks `[y]es / [n]o / [a]ll this turn
  / [q]uit turn`. Files are replaced atomically; a replaced file keeps its
  mode and a new one gets the mode your umask allows. Command arguments may
  not name an absolute, drive (`C:`),
  home (`~`) or parent (`..`) path, also as `--flag=value`; the values of
  `--schema`, `--output` and `--privacy-source` follow the path rules above.
  Commands run without a shell, with standard input closed, bounded output and
  a time limit, and finish when the command exits. On Unix each runs in its
  own process group: Ctrl+C is forwarded to it, and the time limit, or
  processes it leaves holding its output, stop the whole group (on Windows,
  only the command itself).
* **Non-interactive use:** when standard input, output or error is not a
  terminal, or `CI`/`TERM=dumb` is set, actions are printed as a plan and never
  executed. `NO_COLOR` disables colour.
* **Upgrade sessions:** `cargo rullst ai upgrade` (flags `--to`, `--all-rules`,
  `--provider`, `--model`, `--dry-run`) prints the `cargo rullst upgrade --dry-run` plan and,
  when it has source findings, starts one goal grounded in them: the trusted
  instructions carry only the migration rows those findings reference; the
  findings, each flagged line and the affected files (within the path policy)
  are untrusted data. Fixes go through the same diff, confirmation, checkpoint
  and `cargo check` flow; Rullst dependency versions stay with
  `cargo rullst upgrade`. Without findings no session starts. Offline, the
  demo rewrites one `render_page` call (`V13-RENDER-PAGE-LANGUAGE`) and
  proposes `cargo check`. A one-word chat goal `upgrade` now runs this command.
  See [assisted fixes](tutorials/36-assisted-framework-upgrades.md#assisted-fixes-with-cargo-rullst-ai-upgrade).
* **Error fixes:** in a debug build running in Development, the error page
  for a panic shows `cargo rullst ai fix <ERROR_ID>`. The command reads that
  error (message, location, project backtrace frames, request method and path)
  from `GET /_rullst/errors/{id}` of the development server: by default
  `http://127.0.0.1:<PORT>` with the port `cargo rullst dev` uses, or
  `--url`, which must be plain HTTP on a loopback address (`127.0.0.1`,
  `[::1]` or `localhost`, pinned to `127.0.0.1`); any other source is refused
  before a connection. The server keeps at most 32 errors for 30 minutes and
  forgets them when it restarts, so an unknown or expired id is an error.
  High-signal secrets are redacted, the context and the source around the
  location (within the path policy) are sent as untrusted data, and the goal
  runs as a normal session: diff, confirmation, checkpoint and `cargo check`.
  Offline, the demo repeats the location it received and proposes
  `cargo check`.
* **Reviews:** `cargo rullst ai review` reviews the staged and unstaged
  changes against `HEAD` (`--staged`: staged only; `--base <REF>`:
  `git diff <REF>...HEAD`; `--include-untracked`: also untracked files that
  `.gitignore` does not exclude). It needs a git work tree and is read-only:
  it never edits files or runs commands, and action blocks in the answer are
  counted and ignored. Before anything is sent, files under the path policy
  (`.env*`, keys and certificate stores, credentials, databases,
  `Cargo.lock`, `.git/`, `target/`) and binary files are omitted and listed,
  the audit report's high-signal secret patterns are redacted, each file's
  diff is capped at 24 KiB and the whole review at 96 KiB (truncated and
  unsent files are listed). The model checks correctness, security (SQLx
  parameters, CSRF, ownership checks and `rullst-access` classifications,
  constant-time signature checks), panics in production paths, missing tests
  and Rullst rules such as bare or conditional `html!` boolean attributes.
  Findings print with `file:line`, severity and a suggested fix, or as the
  `rullst.ai-review.v1` JSON document with `--json` (`files.reviewed`,
  `files.omitted`, `files.truncated`, `redactions`, `findings[]` with `file`,
  `line`, `severity`, `title`, `detail`, `suggestion`, `summary`,
  `ignored_actions`, `format_error`, `raw_answer` and `usage`). The offline
  assistant flags `unwrap()`, `expect()` and `panic!` on added lines outside
  tests and redacted secrets deterministically. The assistant itself may
  propose the read-only `cargo rullst ai review` (with `--staged`,
  `--base <ref>`, `--include-untracked` or `--json`).
* **Checkpoint:** before the first change of a session the CLI stores a
  snapshot of the work tree (tracked and untracked, non-ignored files, without
  `.env*` and `target/`) under `refs/rullst/ai-checkpoints/<UTC timestamp>`,
  using a temporary index so your index, stash and files are untouched, and
  prints `git diff <ref>` and `git restore --source=<ref> --worktree -- .`.
  Outside a git repository it asks before continuing without one. After
  changes it offers to run `cargo check`.
* **Untrusted data:** project context, shared files and command output are
  sent inside delimited `<untrusted-data>` blocks, size-capped and checked by
  the rullst-ai guardrails (a match is withheld). The project context is a
  separate system message, so a match there cannot block the conversation.
* **Usage:** after each answer the CLI shows the tokens the provider reported
  (input, output and cached input) and the elapsed time, and the session totals
  on exit. Providers or servers that report nothing show `usage not reported`;
  counts are never estimated. A cost appears only when you configured prices,
  labelled `≈ … est. at your prices` (all input tokens at the input price, so
  cache discounts are not modelled). Rullst ships no price table.

### `cargo rullst audit [--ai] [--compliance] [--idor]`
Runs bounded source/configuration checks and can invoke installed dependency
scanners. Static findings require human review and are not a penetration test or
compliance certification.
* **Flags:**
  * `--ai`: Prints fixed, rule-based remediation suggestions after the checks. It calls no AI model or network service; the flag keeps its legacy name.
  * `--compliance`: Writes `SECURITY_COMPLIANCE.md`, an evidence report. Each executed check is `NO FINDINGS`, `NO FINDINGS OUTSIDE EXCEPTIONS`, `FINDINGS`, `GENERATED`, `OBSERVED`, `NOT CHECKED`, or `ERROR`, and control families outside the command's scope are `NOT EVALUATED`. It never reports `PASS` and does not confer SOC 2 or ISO 27001 certification.
  * `--idor`: Fails on parameterized routes without an adjacent `// rullst-access: public|owner|role|admin — reason` classification and the recognized guard required by non-public classifications. `public` is accepted only for recognized GET routes. This bounded heuristic cannot prove domain authorization correctness.
  * `--report [md|html|json]` (13.0): Writes an evidence report mapped to OWASP ASVS 5.0.0 Level 1, with a personal-data inventory and accessibility heuristics; see [the security report guide](security-report.md) and the flag details [below](#cargo-rullst-audit).

### `cargo rullst eject [--force] [--output <path>]`
Writes a reviewable Axum/Tokio entry-point template (`src/ejected_main.rs`). It
is not a translation of the project's `main.rs`: it serves a placeholder route
from `application_routes()`, wraps it in the configured Rullst security
baseline (`apply_security_baseline` with `Rullst.toml`, as `Server` applies it)
and reaches Axum through `rullst::web::axum`, so it compiles without a direct
Axum dependency. Move the application's routes and module declarations into it
before use. Static files, rate limiting, lifecycle probes, hot reload,
sessions, authentication and authorization are not included.
* **Flags:**
  * `--force`: Replaces `src/main.rs` with the template after copying the original to `src/main.rs.rullst-backup` (an existing backup stops the command). Application routes are not carried over.
  * `--output <path>`: Specifies a custom output path below `src/` for the ejected file.

### `cargo rullst deploy:doctor` (v13 candidate)

Read-only inspection of a local deployment configuration snapshot:

```bash
cargo rullst deploy:doctor --env-file .env.production --json
cargo rullst deploy:doctor --config Rullst.production.toml --process-env
```

Reuses Core environment/security validation, catches obvious key/configuration
mistakes and identifies application-policy reviews without echoing values.
`--target <production|staging>` (default `production`) names the expected
environment; `--json` prints `rullst.deployment-diagnostic.v1`.
Explicit environment sources remain separate. Exit zero covers only the inspected
local profile; `deployment_verified` remains false. See the
[input and output contract](deployment-diagnostic.md) before using it in CI.

### `cargo rullst inspect [target]`
Scans source files and prints structural summaries in the terminal without
starting a server, expanding macros or connecting to a database.
* **Arguments:**
  * `[target]`: The item or file to inspect (default `routes`):
    * `route` or `routes`: Lists `get`/`post`/`put`/`delete` declarations written as `method("path" => handler)` on one line in the regular `.rs` files under `src/`. Like `audit` and `generate:diagram`, the walk does not follow symlinks and is bounded in depth and entries; it reports when a bound left the table incomplete.
    * `model` or `models`: Lists the structs, enums and `pub` fields declared in `src/models`.
    * `schema`: Prints, as JSON, the table, fields, Rust types and optionality of every `#[derive(Orm)]` struct under `src/` (the extractor `make:migration:auto` uses). It describes the models, not the live database. A project-provided `rullst-schema.json` is printed instead when present; Rullst does not generate that file.
    * `<path/to/file.rs>`: Displays the first 40 lines of any target Rust file with line numbers.

---

## 🚀 5. Development, Infrastructure, and Build

### `cargo rullst dash`
Opens the Ratatui development control surface in an interactive terminal. The
dashboard reports the probed application port, supervised auto-reload
state, the child process exit state, and the configured database profile; it
does not label a database as connected merely because a URL exists. Logs and
input queues are bounded, ANSI control sequences are removed, terminal state is
restored on error, and the owned application process is stopped and reaped when
the dashboard exits.

**Live metrics (v13).** Once a second the dashboard reads
`GET /_rullst/dev-telemetry` from the application it started, on
`127.0.0.1:<port>`, and shows only what the application reports:

* requests per second over the last 10 s, from the application's exact request
  counters, and 4xx/5xx totals since the process started;
* errors: 5xx responses and their share of all requests over the last 60 s;
* p50/p95 latency over the last 60 s and a p95-per-poll sparkline (up to two
  minutes), computed from the individual requests the dashboard observed. They
  are marked `sampled` when more requests arrived between two polls, or before
  the first poll of a process, than the 64 newest the application returns;
* the newest requests with status, method, duration and path (never the query
  string);
* ORM queries since start and slow ORM operations (at least 100 ms);
* possible N+1 queries (see [N+1 query warning](#n1-query-warning));
* the pending jobs of a queue.

Counters, latency samples and slow operations restart with every new process
(a rebuild or `r`); the request list and the sparkline keep earlier entries.
Histories are bounded: 120 sparkline points, 50 requests, 16 slow operations and at most
4,096 latency samples. A response larger than 256 KiB, with another schema or
with inconsistent counters is rejected, and control or bidirectional-formatting
characters are replaced before anything reaches the terminal. A value the
application does not report is shown as `not reported`, never as zero. Only
the process the dashboard started is measured: a response carrying another
`RULLST_DEV_GENERATION`, for example from a second `dev`/`dash` already
listening on the port, replaces the metrics row with an **ANOTHER PROCESS ON
THE PORT** panel instead of showing that process's figures.

The endpoint exists only in a debug build running in Development under
`cargo rullst dev`/`dash`; see the
[telemetry guide](telemetry-guide.md#development-dashboard-endpoint). When the
application does not serve it, the metrics row is replaced by a **TELEMETRY NOT
AVAILABLE** panel with these steps:

1. Serve the application with `rullst::Server` from Rullst 13 or newer (the
   generated starters do).
2. Run a debug build in Development: leave `RULLST_ENV`/`APP_ENV` unset or set
   `RULLST_ENV=development`.
3. Press `r` (or save a file) to restart.

| Value | Reported when | To enable it |
| :--- | :--- | :--- |
| Requests, latency, errors | The endpoint answers. | The steps above. |
| ORM queries and slow operations | Rullst's default tracing subscriber is installed (`Server::run` installs it when the application has none) and keeps `rullst_orm` INFO spans. | Do not install another global subscriber first, and do not filter out `rullst_orm=info` with `RUST_LOG`. |
| Queue pending | The application passes its queue to the server. | `Server::new(router).with_dev_queue(queue)` (accepts a `Queue` or an `Arc<Queue>`). |

ORM queries count the outermost `rullst.orm.query` span of each ORM operation
(model queries, saves, deletes and `Orm::raw`). A `chunk`/`chunk_by_id`
traversal is not counted itself: each page it fetches and each operation its
handler runs counts separately. One operation can run several SQL statements,
and SQL executed directly through SQLx is not counted.

#### N+1 query warning

When the application reports that one request ran the same ORM operation at
least three times (the threshold Studio's trace view uses), a **POSSIBLE N+1
QUERIES** panel appears below the logs. Each line shows the request's method
and matched route (`GET /posts/{id}`), the operation fingerprint
(`Comment.select_many (comments)`, built from the ORM span's model, operation
and table labels; Rullst never records SQL text or bound values) and the
repetition count, followed by a hint to load the related rows eagerly or batch
the lookups into one query. Inspect the handler before changing it: repeating
an operation can be intentional. Raw statements (`Orm::raw`) and operations
started on another task (`tokio::spawn`) are not attributed to a request, so
they never produce a finding. The **VERIFIED PROJECT STATE** panel shows the
detection state in its `N+1 check` line: the number of findings since the
process started, `N+1 detection needs request-correlated ORM telemetry (Rullst
Core 13)` when the application's telemetry has no request-correlated ORM data
(an older Rullst Core), or `N+1 detection needs ORM query telemetry` when ORM
queries themselves are not reported (see the table above).

Performance note: a request that runs one ORM lookup per row (for example
loading each post's comments inside a loop over posts) makes one database round
trip per row. [Eager loading](crates/orm.md#query-row-cap) runs one
related-model query for all parents of a batch, and an `IN` query can fetch
the rows of several keys at once; measure the route before and after the
change.

Terminals at least 26 rows tall show the metrics row and terminals at least 105
columns wide add the recent-requests panel; shorter terminals show a one-line
summary in the header. The layout adapts to narrower terminals and provides
these keyboard controls:

* `r`: restart the application from the current build (no rebuild). It is
  refused, with a notice, while the application is still starting, while a
  migration runs and while a saved change is being rebuilt (a successful
  rebuild restarts the application itself). The notice clears once the
  restarted application is ready, or says that its readiness was not
  confirmed or that it exited.
* `o`: open the application.
* `s`: probe the loopback Studio endpoint and open it only when reachable.
* `d`: open existing Scalar docs. Missing files produce explicit
  `cargo rullst make:scalar` guidance rather than silently modifying the project.
* `m`: run `db:migrate` asynchronously and report its real exit result.
* `/`: search both log panes; `f` cycles all/warning+error/error filtering.
* `Tab`: switch the focused log pane; arrows and Page Up/Page Down scroll it.
* `c`: clear dashboard logs.
* `?`: keyboard and metrics help. Any key closes it; `q` still quits.
* `q` or `Esc`: exit (`Esc` closes the help first).

The footer lists `d` and `Tab` only on terminals at least 120 columns wide; the
help always lists every key.

The animated neon palette is enabled only for an interactive terminal. Set
`RULLST_REDUCED_MOTION=1` to keep colors with static rendering, or `NO_COLOR=1`
for a color-free, static interface. Non-interactive automation should use
`cargo rullst dev`; `dash` fails clearly when no terminal is attached.

### `cargo rullst dev`
Builds and starts a directly linked application. Saving source, static assets,
templates, `Cargo.toml`, `Cargo.lock`, `Rullst.toml` or `.env` schedules a
coalesced rebuild. A failed build leaves the current application running. A
successful build creates an owned executable snapshot, stops the previous
process and starts its replacement. The snapshot avoids locking Cargo's build
output on Windows. Initial migrations run before startup; later migrations
remain an explicit command.

The same-origin browser client polls an opaque process-generation marker and
refreshes only when a different generation serves successfully. This is enabled
only in debug/development. Readiness verifies that marker, not just an open port.
Changing the configured port requires restarting the CLI. In-memory state and
unsaved browser state reset during reload. The process receives a bounded
shutdown interval before forced termination; this is a development facility.
Ctrl+C, SIGTERM (an IDE stop button or `kill`) and SIGHUP (a closed terminal)
end `dev` and `dash` the same way: the application's process group is stopped
and its executable snapshot removed (on Windows, Ctrl+C and closing the console).

No scaffold question is required: `dev` and `dash` enable auto-reload, while
`cargo run` runs the application normally. The legacy `--hot-reload` scaffold
flag is rejected (since v12) because DLLs can split ORM/Tokio globals. Existing
legacy scaffolds can use their directly linked router; the supervisor removes
`HOT_RELOAD` from its child's environment.

See [Supervised Development Auto-Reload](tutorials/51-authenticated-hot-reload.md)
for limitations, failure recovery and the v13 architecture decision.

`dev` keeps its plain output; at startup it prints one line suggesting
`cargo rullst dash` for live requests, latency, errors and database metrics.

* **Optional Flags:**
  * `--ts-sync`: Regenerates the TypeScript client SDK (`rullst-client.ts`, as `generate:ts` writes it from the routes in `src/main.rs` and `src/lib.rs`) after the initial build and after every successful rebuild. A failed generation is reported and the application keeps running.

### `cargo rullst build:client`
Builds the library for `wasm32-unknown-unknown`, runs `wasm-bindgen`, and writes a
separate `static/rullst-islands.js` hydrator that awaits binding initialization.
It parses `Cargo.toml`, merges the required `cdylib` crate type without replacing
existing library crate types, and honors an explicit `lib.name`. The command
checks/installs the Rust target and `wasm-bindgen-cli`; any failed tool step
aborts. Bundle size and browser performance depend on the generated application
and must be measured.
* **Flags:** `--debug` (Avoids extreme minification so you can inspect and debug Wasm sourcemaps).

### `cargo rullst build`
Creates the monolithic final Production binary of the backend and writes Brotli
(`.br`) and Zstandard (`.zst`) siblings next to the `html`, `css`, `js`, `json`,
`svg`, `wasm`, `xml` and `txt` files under `static/`. The server prefers a
sibling over its source, so rerun the command after editing an asset and before
building an image; `cargo rullst dev` removes siblings that are not newer than
their source at startup and on every change (siblings without a source file are
kept).
* **Flags:** `--debug` (Compiles with debug information, generating a larger binary).

### `cargo rullst green` (extras)
**Maturity:** Experimental; part of the optional
[`cargo-rullst-extras`](#extras-cargo-rullst-extras) package. It replaces
`cargo rullst footprint` (13.0.0-alpha.1 to alpha.3).

Reports figures it measured for the app, each with its method: the release
binary size, a local container image's size (`--image`, through `docker` or
`podman image inspect`), the idle resident memory of the started app (Linux
`VmRSS`), the third-party dependency count from `Cargo.lock` and, with
`--bench`, the app process's CPU time during a bounded closed-loop loopback
load and the CPU time per request. Anything unavailable is `NOT MEASURED` with
the reason. It measures no energy: a Software Carbon Intensity (SCI,
ISO/IEC 21031:2024) figure appears only as an estimate labelled `estimate`,
computed from the measured CPU time, your `--cpu-watts` and your
`--grid-intensity`, with every input shown.

Without `--url` it builds the release binary (reusing an existing build),
starts it with `RULLST_ENV=production` (or `--env staging`, which accepts the
starters' mock billing credentials) on a free `127.0.0.1` port, measures it
and stops it. With `--url` it measures an app already running on loopback and
refuses any other host; it makes no network calls beyond loopback and the
local container engine, and never fetches grid intensity.

```bash
cargo rullst green --image my-app:latest
cargo rullst green --url http://127.0.0.1:3000 --path /health --bench --json
cargo rullst green --bench --cpu-watts 15 --grid-intensity 120
```

* **Flags:** `--url <http://127.0.0.1:PORT>`, `--env <production|staging>`
  (default `production`; not with `--url`), `--path <PATH>` (default `/`),
  `--image <NAME>`, `--bench`, `--duration <10s|1m|1500ms>` (default `10s`,
  1 s to 10 min; needs `--bench`), `--concurrency <N>` (default 4, at most
  256; needs `--bench`), `--cpu-watts <W>` (needs `--bench`),
  `--grid-intensity <gCO2e/kWh>` (needs `--cpu-watts`), `--embodied <gCO2e>`
  (needs `--grid-intensity`), `--json` (versioned `rullst.green.v1`).
* **Exit status:** `0` when the measurement ran, even with `NOT MEASURED`
  fields; `1` when the build failed or the app could not be started or reached;
  `2` for invalid arguments.

See the [green guide](green.md) for what each figure means and how to choose a
grid intensity.

### `cargo rullst dockerize`

Writes the `Dockerfile` (plus `.dockerignore` when absent) into an existing
project, as `new --docker` does. It refuses to replace an existing
`Dockerfile`; move a customized file aside to regenerate the template. The
binary is named after `[package].name`, read with a TOML parser (`app` when
`Cargo.toml` has no package name). `cargo rullst deploy` writes the same
Dockerfile together with `compose.yaml` and the `Caddyfile`.

**Removed in 13.0.0-alpha.4:** `nixify`, `generate:buildah`, `foundry:init`,
`foundry:deploy` and `make:grpc`. Files they generated stay yours; see
[Advanced deployment](advanced-deployment.md) for Nix, Buildah, Podman
Quadlet and Kubernetes.

## 🛡️ 6. Security, Compliance & System Diagnostics

### `cargo rullst audit`
Executes bounded automated checks across recognized source, configuration,
route, dependency, and local network patterns.
* **Optional Flags:**
  * `--ai`: Prints fixed, rule-based remediation suggestions; no AI model or network service is called.
  * `--compliance`: Generates the evidence report described above (no `PASS` results); it is not a SOC 2, ISO 27001, or transport certification.
  * `--idor`: Fails on parameterized routes without an explicit adjacent access classification. `owner` requires `RbacGuard::authorize_owner_or_role`; `role` requires a recognized role guard; `admin` requires `RequireRoleLayer` or `NexusAuthPolicy::protect_router`; `public` is restricted to recognized GET routes. The marker goes on the route's line or the line above it; in a multi-line `.route(` call, on the line above the path literal. The guard may appear anywhere in the same crate's `src` tree outside comments and `#[cfg(test)]` items, so the check does not prove that the route is mounted behind it. Manual review and runtime negative tests remain required.
  * `--geiger`: Inventories `unsafe` in the dependency tree. Unsafe may be justified and requires review; the command does not prove a zero-unsafe invariant.
  * `--sbom`: Generates a standardized **CycloneDX 1.5 JSON** Software Bill of Materials (`sbom-cyclonedx.json`) from `Cargo.lock`, with the SHA-256 checksums the lockfile records. It contains no license metadata.
  * `--audit-ignore RUSTSEC-YYYY-NNNN`: Passes one explicit, repeatable advisory exception to `cargo audit`. A successful run is reported as **NO FINDINGS OUTSIDE EXCEPTIONS**, not “no findings”; the caller must separately version, own, review, and expire every exception.
  * `--network`: Checks a bounded list of local ports/bindings for potentially exposed services; it is not a comprehensive network scan. The TCP listener inventory runs `ss -ltnH` (Linux iproute2). Where it cannot run, as on macOS, Windows or a Linux image without iproute2, the check is reported as `ERROR` and the command exits non-zero instead of reporting a clean scan.
  * `--report [md|html|json]` (new in 13.0): Runs the static report checks
    below and writes `SECURITY_REPORT.md` (the default), `SECURITY_REPORT.html`
    or `SECURITY_REPORT.json` in the current directory; `--output <path>`
    chooses another file (its directory must exist). It reuses the `cargo audit`
    step with its `--audit-ignore` exceptions and the `--idor` scanner, and
    cannot be combined with `--json`, `--compliance`, `--sbom`, `--network`,
    `--geiger` or `--ai`. See [the security report guide](security-report.md).
  * `--json`: Moves the progress lines to standard error and prints one
    `rullst.cli-audit.v1` summary on standard output; the exit status is
    unchanged (non-zero for findings or incomplete requested checks):

```json
{
  "schema": "rullst.cli-audit.v1",
  "schema_version": "rullst.cli-audit.v1",
  "issues_found": 0,
  "checks": [
    { "id": "secret_scan", "status": "not_checked", "count": null, "exceptions": [], "detail": "no .env file was available to inspect" },
    { "id": "dependency_audit", "status": "no_findings_outside_exceptions", "count": null, "exceptions": ["RUSTSEC-2099-0001"], "detail": "..." }
  ]
}
```

Check ids are `secret_scan`, `dependency_audit`, `unsafe_scan`, `idor_scan`,
`sbom` and `network_scan`, always in that order; `status` is `no_findings`,
`no_findings_outside_exceptions`, `findings`, `generated`, `observed`,
`not_checked` or `error`, and `count` is set for `findings`, `generated` and
`observed`. Details are redacted like error reports.

In a package directory, the unsafe and IDOR/BOLA scans cover its `src` and the
`src` of every workspace member below it, as listed by `cargo metadata`; in a
directory without `src`, such as a virtual workspace root, the IDOR/BOLA scan
walks every `src` tree below it. The source scans (unsafe syntax, IDOR/BOLA
routes and listener bindings) do not follow symlinked files or directories and
skip `target/` and `.git/`. A walk
stops at 64 directory levels or 250,000 entries; reaching either bound is
reported as a finding, so the scan fails as incomplete instead of passing. The
route and listener scans skip each top-level `#[cfg(test)]` item (such as
`mod tests;` or an inline test module) on its own; code after it is still
scanned.

`cargo audit` and the SBOM read the project's own `Cargo.lock`. In a workspace
member, which has none, they read the lockfile of the workspace root that
`cargo metadata` reports, and `cargo audit` receives it as `--file`; the SBOM
then lists every package of that workspace lockfile.

SBOM components come from `Cargo.lock`. Only crates.io packages receive the
plain `pkg:cargo/<name>@<version>` purl; a package from another registry adds a
`repository_url` qualifier, a git package adds a `vcs_url` qualifier with the
locked commit, and path or workspace packages (including the application) have
no purl. Every component that is not from crates.io carries a
`rullst:cargo:source` property with its lockfile source, or `local`.

`SECURITY_COMPLIANCE.md` and `sbom-cyclonedx.json` are written in the current
directory and replace a previous regular file. Because an audit may run on an
untrusted checkout, the command refuses to write either file through a symlink.

#### Security report (`--report`)

The report maps each check to [OWASP ASVS 5.0.0](https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release)
(release `v5.0.0_release`) Level 1 requirement IDs and section names, and
lists the Level 1 requirements no check relates to as `NOT EVALUATED`, by
chapter. A header states that it does not replace a manual review or a
penetration test; it never reports `PASS` or a certification.

| Check id | What it reads | ASVS 5.0.0 |
| :--- | :--- | :--- |
| `security_headers` | `Server::new` or a header layer in `src`; `[security] csp`/`coep` in `Rullst.toml` | V3.4.1 (L1); V3.4.3, V3.4.4, V3.4.6 (L2) |
| `csrf` | Single-line write routes; `Server`, `csrf_middleware`, `csrf_signed_webhook_paths` | V3.5.1 (L1) |
| `cookies` | `Set-Cookie`-shaped string literals, cookie builders, `csrf_same_site` | V3.3.1 (L1); V3.3.2, V3.3.4 (L2) |
| `auth_rate_limit` | Authentication write routes; a rate limiter or `LoginGuard` | V6.3.1 (L1) |
| `committed_secrets` | Files from `git ls-files` (not `target/`, binaries, symlinks or files over 2 MiB) | V13.3.1 (L2), chapter V13 |
| `dependency_audit` | `cargo audit` with the `--audit-ignore` exceptions | V15.2.1 (L1) |
| `idor` | The `--idor` route-classification scan | V8.2.1, V8.2.2 (L1) |
| `a11y_img_alt`, `a11y_form_labels`, `a11y_html_lang` | `html!` bodies and templates under `src/` and `templates/` | WCAG 2.2 1.1.1; 1.3.1, 3.3.2, 4.1.2; 3.1.1 |

Each check reports `no_findings`, `no_findings_outside_exceptions`,
`findings`, `not_checked` or `error` (shown as `NO FINDINGS`, `FINDINGS`, ...
in Markdown and HTML) with a one-line fix and a link to the guide. A
committed-secret finding shows only the file, the line and the first four
characters of the value followed by `…`. The personal-data inventory
(`#[privacy]`, `#[orm(encrypted)]`, `SecretString`, `#[orm(masked)]`, and
`review` for unclassified fields named like email, phone, cpf, cnpj, ssn,
birth, address, document or ip) is `observed` and never changes the exit
status.

**Exit status:** 1 when any check reports `findings` or `error`; 0 otherwise,
including when checks are `not_checked` (no `src/`, no Git work tree, no
`cargo-audit`). The terminal prints a grouped ✓/!/✗ summary, plain under
`NO_COLOR`, `CI`, `TERM=dumb` or a non-terminal stdout.

**Formats:** Markdown has one GitHub table per section; HTML is one
self-contained file (inline CSS with light and dark schemes, no script, no
external asset, every value escaped); JSON is the `rullst.cli-audit-report.v1`
document below, without ANSI sequences:

```json
{
  "schema_version": "rullst.cli-audit-report.v1",
  "generated_at": "2026-10-08T12:00:00+00:00",
  "tool": { "name": "cargo-rullst", "version": "13.0.0" },
  "notice": "This report records bounded static checks for a reviewer. ...",
  "standard": { "name": "OWASP Application Security Verification Standard", "version": "5.0.0", "level": 1, "source": "https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release", "level1_requirements": 70, "level1_mapped": 7, "mapping_note": "..." },
  "accessibility_standard": "WCAG 2.2",
  "exit_non_zero": true,
  "checks": [
    { "id": "csrf", "group": "security", "title": "CSRF", "status": "findings", "count": 1, "exceptions": [], "detail": "...", "fix": "...", "docs": "https://rullst.github.io/Rullst/book/security-report.html#csrf", "asvs": [{ "id": "V3.5.1", "section": "V3.5 Browser Origin Separation", "level": 1 }], "asvs_chapter": null, "wcag": [], "findings": [{ "file": "src/lib.rs", "line": 12, "message": "...", "preview": null }] }
  ],
  "personal_data": { "status": "observed", "count": 2, "review": 1, "detail": "...", "docs": "...", "asvs_chapter": "V14 Data Protection", "fields": [{ "model": "User", "field": "email", "file": "src/models/user.rs", "line": 8, "classification": "review", "encrypted": false }] },
  "not_evaluated": [{ "chapter": "V1", "name": "Encoding and Sanitization", "status": "not_evaluated", "whole_chapter": true, "requirements": ["V1.2.1", "..."] }]
}
```

`checks` always holds the ten ids of the table above, in that order. A new
key or a changed meaning needs a new `schema_version`.

**Limits:** every check is a bounded static heuristic. Routes are recognized
only when declared on one line; limiter, guard and header evidence is
project-wide and does not prove a route is mounted behind it; untracked files
and Git history are not scanned for secrets; templates are not rendered and
includes are not followed. The source walks share the `audit` bounds (no
symlinks, 64 levels, 250,000 entries) and an incomplete walk turns the source
checks into `error`. Files over 2 MiB are not read.

### `cargo rullst hook:install`
Installs managed `pre-commit` and `commit-msg` wrappers. The first runs
`cargo fmt --all -- --check`, strict workspace Clippy, and
`cargo rullst audit --idor`; the second enforces Conventional Commits while
accepting the subjects Git itself writes for merges (`Merge branch`,
`Merge remote-tracking branch`, `Merge tag`, `Merge pull request`, ...),
reverts (`Revert "..."`) and `fixup!`/`squash!`/`amend!` commits. Existing
active hooks are moved to explicit `.rullst-original` backups and invoked first,
while reinstalling the managed wrappers is idempotent. The command supports
linked worktrees, fails clearly outside a Git worktree, and refuses a backup
collision instead of overwriting it. When `core.hooksPath` (local, global or
system Git configuration, as used by Husky or shared hook directories) selects
another directory, it fails before writing, because Git would never run wrappers in the
default hooks directory; call the checks from that hook manager instead. These local hooks are bypassable by design;
protected CI remains authoritative.

### `cargo rullst doctor`
Runs grouped health checks and shows each one as ✓ (pass), ! (warning),
✗ (failed) or · (information, optional or not applicable). Every warning and
failure carries a fix (one line, or a short configuration snippet) and a link
to its group below. The command
exits with status `1` when any check failed; warnings do not fail it.
`--fix` installs missing rustfmt/clippy components with
`rustup component add rustfmt clippy`, then checks again. Outside a project
only the toolchain, project, security (cargo-audit) and disk groups appear.
Checks are local and bounded: tool probes are `--version` calls run in
parallel, the database probe is a 2-second TCP connect (no credentials are
sent) and a SQLite file is opened read-only. Kani is only looked up as a
`cargo-kani` executable (in Cargo's `bin` directory or on `PATH`), never run:
`cargo kani` starts its first-time download and toolchain installation when it
is not set up yet. Values from `.env` are measured,
never printed.

#### Doctor: toolchain
`rustc` (✗ when missing or older than the 1.96.0 MSRV), Cargo, rustfmt and
clippy (fixable with `--fix`), Git, the `wasm32-unknown-unknown` target when
the project has `src/islands`, and one informational line for the optional
tools (cargo-deny, cargo-geiger, cargo-mutants, Kani, cargo-llvm-cov, Docker).

On Linux, `toolchain.linker` (v13) is a `!` hint, never a failure, when no
fast linker is selected: no `-fuse-ld=mold`, `-fuse-ld=lld` (or
`--ld-path=` naming one of them) and no `linker` entry in `[build]`, the host's
`[target.<triple>]` or a `[target.'cfg(..)']` table of any `.cargo/config.toml`
(or legacy `.cargo/config`) from the working directory up, or of
`$CARGO_HOME/config.toml` (`~/.cargo` by default), and none in `RUSTFLAGS`,
`CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` or the host's
`CARGO_TARGET_<TRIPLE>_RUSTFLAGS`/`_LINKER`. It says whether `mold` or `ld.lld`
is on `PATH` and shows the snippet to add, for example:

```toml
[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

If the file already has that table, add the two flags to its `rustflags`
instead of repeating the table. The check is skipped on other operating systems
and on x86_64-unknown-linux-gnu with Rust 1.90 or newer, where rustc already
links with its bundled LLD by default.

#### Doctor: project
The enclosing Rullst project (a `Cargo.toml` that depends on `rullst`) and its
`rullst` requirement compared with the CLI's major version; an older project
suggests `cargo rullst upgrade --dry-run`.

#### Doctor: config and .env
Whether `.env` parses, which keys listed in `.env.example` are set neither in
`.env` nor in the process environment (names only), whether `RULLST_ENV`
(or `APP_ENV`) names a known environment, and whether `Rullst.toml` is valid.

#### Doctor: database
The primary database from `DATABASE_URL` (environment, then `.env`, then
`[database].url` in `Rullst.toml`): a SQLite file must exist or be creatable;
PostgreSQL and MySQL/MariaDB must accept a TCP connection (✗ otherwise).
`REDIS_URL` is probed the same way (! when unreachable). Turso/remote and
`mock_*` endpoints are not contacted.

#### Doctor: migrations
The `src/migrations/m*.rs` files and, for a local SQLite database, which of them
the ORM `migrations` table has not applied yet (`cargo rullst db:migrate`).
For server databases run `cargo rullst db:status`.

#### Doctor: security baseline
`cargo-audit` installed, `.env` neither tracked nor unignored by Git (✗ when
tracked), `APP_KEY` set to at least 32 characters and not a template
placeholder, and a `Cargo.lock` for reproducible builds.

#### Doctor: disk space
Free space on the filesystem of the project (or the working directory): ✗
below 1 GiB, ! below 5 GiB. Not measured on Windows yet.

`cargo rullst doctor --json` prints one document on standard output and keeps
the exit status rules:

```json
{
  "schema": "rullst.cli-doctor.v1",
  "schema_version": "rullst.cli-doctor.v1",
  "cli_version": "13.0.0",
  "ok": false,
  "summary": { "pass": 11, "warn": 5, "fail": 1, "info": 2 },
  "groups": [
    {
      "id": "database",
      "title": "Database",
      "checks": [
        {
          "id": "database.primary",
          "title": "Primary database",
          "status": "fail",
          "detail": "PostgreSQL at 127.0.0.1:5432: connection refused",
          "fix": "Start the server (for example `docker compose up -d`) or fix the URL in .env",
          "docs": "https://rullst.github.io/Rullst/book/cli_reference.html#doctor-database"
        }
      ]
    }
  ]
}
```

`status` is `pass`, `warn`, `fail` or `info`; `fix` and `docs` are `null` for
passing checks. Group ids are `toolchain`, `project`, `config`, `database`,
`migrations`, `security` and `disk`; check ids (`toolchain.rustc`,
`config.env_keys`, `security.app_key`, ...) are stable, but a check appears
only when it applies.

### `cargo rullst academy:doctor`
Evaluates the twelve requirements of `ProductionPreset::academy()` (identity,
school membership, entitlements, object authorization, tenant isolation,
assessments, score events, durable automation and audit, content safety,
privacy and distributed abuse controls) against the declarations in a
`rullst.academy-evidence.v1` file passed with `--evidence <file>`. It reads
only that file and never inspects the project, so it needs no Academy
scaffold: v13 removed the complete Academy scaffold, and the LMS example
(the former LMS starter) has no evidence file. Without `--evidence` every requirement is
`NOT_EVALUATED`.

Each requirement is reported as `PASS`, `FAIL`, `SKIPPED` or `NOT_EVALUATED`.
Unknown fields or requirements, a repeated requirement and a `PASS` without a
non-empty evidence reference are rejected. `--json` prints
`rullst.academy-diagnostic.v1` with `certification` (always `false`),
`contract_satisfied` and each requirement's `status` and `declared_evidence`.
The command exits with status `1` until every requirement has an evidenced
`PASS`. Evidence strings are caller declarations, not verified proof; see the
[Academy production-boundary diagnostic](security-architecture.md#academy-production-boundary-diagnostic).

### `cargo rullst inspect [target]`
Prints static structural summaries in the terminal (the analyzer entry above
describes the exact scope):
* `cargo rullst inspect route`: Lists the recognized one-line `get`/`post`/`put`/`delete` route declarations.
* `cargo rullst inspect model`: Lists the structs and public fields in `src/models`.
* `cargo rullst inspect schema`: Prints the ORM model schema derived from `#[derive(Orm)]` structs as JSON.
* `cargo rullst inspect routes --json`: Prints the recognized routes as one
  `rullst.cli-routes.v1` document:

```json
{
  "schema": "rullst.cli-routes.v1",
  "schema_version": "rullst.cli-routes.v1",
  "complete": true,
  "incomplete_reason": null,
  "routes": [
    { "method": "GET", "path": "/posts/{id}", "handler": "controllers::posts::show" }
  ]
}
```

`complete` is `false` when the bounded source walk stopped early or `src/` is
missing, with the reason in `incomplete_reason`.

`--json` is accepted for `routes`, `models` and `schema` (since
13.0.0-alpha.4):

* `inspect models --json` prints `rullst.cli-models.v1`: `complete`,
  `incomplete_reason` and `files`, one entry per `src/models/*.rs` file (not
  `mod.rs`, no symlinks) with its public `struct` and `enum` items. A struct has
  `fields` (`name`, `type`); an enum has `variants`. A file that is not valid
  Rust is named in `incomplete_reason`.
* `inspect schema --json` prints `rullst.cli-schema.v1`: the same `source` and
  `models` as the human output (`struct`, `table`, `fields` with `name`,
  `rust_type`, `optional`), or, when the project provides `rullst-schema.json`,
  `source: "rullst-schema.json"` with the parsed file under `snapshot`.

---

## 🛠️ Quick CLI Cheat Sheet

```bash
# Create a new project with fast-linker scaffolding
cargo rullst new my_app

# Reverse-engineer ORM models from an existing database
cargo rullst make:models-from-db --driver postgres --url "postgres://user:pass@localhost:5432/mydb"

# Statically inspect routes, models, or schemas in the terminal
cargo rullst inspect route
cargo rullst inspect model

# Open the local Studio compatibility UI on 127.0.0.1:5555
cargo rullst studio

# Ask the terminal AI assistant (needs cargo-rullst-extras; offline demo until `cargo rullst ai connect`)
cargo rullst ai "add a posts page"

# Write the Dockerfile, compose.yaml and Caddyfile for a Linux VM (Docker or Podman)
cargo rullst deploy
```
