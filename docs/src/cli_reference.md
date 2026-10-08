# 💡 Rullst CLI - Full Command Reference

The Command Line Interface (`cargo-rullst`) scaffolds projects, invokes build
tools, and provides bounded static-analysis and deployment helpers.

The CLI's `--help` output is authoritative for the installed version. This page
documents the principal commands of the unreleased version 13 CLI and their
security boundaries.

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
  with dev, dash, scaffolding, database, `doctor` and deploy, and keeps every
  project operation of earlier releases. Started from a subdirectory, menu
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

The opening animates for about 0.7 s only on the first run of each day; later
runs draw the final frame instantly and any key skips the animation. The day
of the last animation is kept in the user cache directory
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

In a terminal the home also asks crates.io in the background for a newer
stable `cargo-rullst` of the same major and prints a notice under the summary
when that answer has already arrived; nothing is installed or cached.
`RULLST_DISABLE_UPDATE_CHECK=1`, `CARGO_NET_OFFLINE=true` or `CI=true` turns
the query off.

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
use, permission denied, invalid input, network failures and an interrupted
prompt. Anything else gets a generic report naming the failed command.
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
(`cargo rullst make:modek` suggests `make:model`; `cargo rullst update chekk`
suggests `update check`; `cargo rullst db` lists the `db:*` group). Ranking
ignores case and separators (`make-model`), then uses edit distance with
transpositions, an exact `group:name` segment (`migrate`) and prefixes.

Colour follows the home screen rules on each stream: none for `NO_COLOR`, `CI`,
`TERM=dumb` or a stream that is not a terminal. When standard output is not
coloured, generator output is plain as well.

After a generator succeeds, the CLI prints up to three concrete **Next steps**
(for example `cargo rullst db:migrate` after `make:model -m`). Generators that
already print their own instructions (`new`, `auth`, `foundry:init`,
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
2. the **blueprint**, each with a one-line description and a compact preview
   of the file tree it generates. The preview is rendered by the real project
   writers into a private temporary directory that is removed immediately;
   nothing is built and no network is used;
3. for the Blank starter, **full-stack web app** or **JSON API**;
4. the **primary database** (Turso and *No database* only for Blank);
5. **optional features** (zero or more): AI, Redis adapters, Dockerfile, Nix
   flake and the Turso, MongoDB, DuckDB, SurrealDB and Qdrant add-ons;
6. a **review** of the answers, the file tree and the commands `new` will run
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
* **Starter Blueprint:** Blank, LMS Platform, SaaS Starter, Blog / Press,
  Portfolio and ERP Pocket (`--blueprint` values `blank`, `lms`, `saas`,
  `blog`, `portfolio` and `erp`).
* **Persistence:** a primary relational backend (SQLite, PostgreSQL, MySQL,
  MariaDB, or bounded Turso-primary for blank/API) plus optional Turso/libSQL,
  MongoDB, DuckDB, SurrealDB, and Qdrant capabilities. The optional selector
  accepts zero or more choices and omits capabilities already selected by the
  primary profile or flags. Specialized adapters remain separate from SQLx
  Active Record.
* **Application profile:** HTML blueprints use the audited `html!` SSR/HTMX path;
  `--api` uses the headless JSON path. Repository, LiveView, Wasm Island,
  Pico.css and Tera foundations remain application-owned APIs and are not
  presented as equivalent generated profiles.
* **Arguments:**
  * `<name>`: The folder and package name (e.g., `my_startup`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API from the Blank starter (no HTML view rendering); SQLx-specific product blueprints reject it instead of ignoring it. The interactive wizard then skips its blueprint and build-type questions instead of letting their Full-Stack default replace the flag.
  * `--docker`: Adds a multi-stage `Dockerfile` and `.dockerignore`. The
    `.dockerignore` mirrors the generated `.gitignore`: it excludes `.env` and
    `.env.*` (except `.env.example`), `Foundry.toml`, SQLite and DuckDB files
    (`*.db`, `*.sqlite`, `*.sqlite3`, `*.duckdb` and their journals, one list
    shared by both files) and the
    host-local `.cargo/config.toml` described below, so the builder's
    `COPY . .` never sends them to a (possibly remote) builder. An existing
    `.dockerignore` is kept unchanged. The runtime
    image installs CA certificates, runs as UID/GID 10001, sets the production
    bind address and copies local static/config assets when present. An explicit
    SQLite selection uses the writable `/app/data` directory. Secrets are never
    embedded and no anonymous volume is declared. Run schema migrations as one
    deployment job before starting or rolling multiple replicas; the generated
    image deliberately does not race migrations from every application process.
    Compose services, persistent-volume ownership, backup/restore and platform
    deployment hardening remain explicit project work. Every starter mounts
    `rullst::health::health_router()`, so the `/health` and `/ready` probes
    written by `make:k8s`, `deploy` and `foundry:deploy` answer `200` without
    authentication.
  * `--turso`: Adds the direct Hrana HTTP v3 Turso/libSQL adapter, checked migrations, and its real-SQL offline development fallback to the selected primary backend. It does not imply transparent replication.
  * `--mongodb`: Enables typed MongoDB document CRUD and its deterministic offline store.
  * `--duckdb`: Enables in-process DuckDB analytics; the optional native dependency increases the first build time.
  * `--surrealdb`: Enables SurrealDB HTTP document CRUD and bounded read-only graph queries.
  * `--qdrant`: Enables bounded dense-vector Qdrant operations and generates empty/`mock_*`-compatible environment fields; it is additive, not the SQL primary.
  * `--nix`: Adds `flake.nix` and `.envrc` (direnv) starting points; reproducibility still depends on pinned inputs and external services.
  * `--buildah`: Adds rootless Buildah container-build files where supported. The image is tagged with the lowercase, `-`-separated form of the package name (`my_startup` becomes `my-startup:latest`), the same name `make:k8s` uses, because OCI repository and Kubernetes names reject uppercase letters and `_`.
  * `--default`: Skips every question and uses deterministic defaults for anything no flag sets (name `app`, Blank, SQLite, no features), intended for CI and reproducible scaffolding.
  * `--blueprint <blank|lms|saas|blog|portfolio|erp>`: Selects the blueprint; the wizard skips that question.
  * `--database <sqlite|postgres|mysql|mariadb|turso>`: Selects the primary relational backend; the wizard skips that question. Network databases must be configured before migration bootstrap. Turso-primary currently supports the blank/API starter and rejects SQLx-specific blueprints explicitly.
  * `--no-database`: Generates the blank blueprint without a primary relational database; it conflicts with `--database` and rejects database-dependent blueprints.
  * `--ai`: Enables the umbrella AI facade in the generated manifest.
  * `--redis`: Enables the umbrella Redis queue/cache/ORM capabilities and the direct ORM Redis feature.
  * `--skip-initial-migration`: Generates the project without running the best-effort initial database migration. `cargo rullst dev` applies pending migrations when it starts; run `cargo rullst db:migrate` explicitly otherwise.
  * `--dry-run` (v13): Prints the plan (the answers, a compact file tree with the exact number of files and the commands it would run) and exits without creating anything. With `--default` it never prompts; in a terminal without `--default` the wizard's review ends with *Finish the dry run*.

Since v13, `--blueprint`, `--database`, `--no-database`, `--ai` and `--redis`
no longer require `--default`: in a terminal they answer their questions and
the wizard asks the rest. `--api`, `--no-database` and `--database turso`
select the Blank starter. The retired `--hot-reload` still requires
`--default` and is rejected.

When the generating Linux host has `mold` or `lld`, the project's
`.cargo/config.toml` selects it to speed up local linking. That file describes
the generating machine only: the generated `.gitignore` and `.dockerignore`
exclude it, so CI runners, teammates and the container builder (which has
neither linker) build with the toolchain default.

Without `--skip-initial-migration`, project creation performs the first Cargo
build before applying migrations. A clean first build can take several minutes,
especially for the larger LMS/SaaS profiles; the animated status remains visible
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

A deterministic profile can pin the blueprint, database and optional features:

```bash
cargo rullst new operations-portal --default --blueprint erp \
  --database mariadb \
  --ai --redis --skip-initial-migration
```

Generated SQLx applications disable the default features of both the umbrella
`rullst` dependency and the direct `rullst-orm` dependency, and select exactly
one strict primary profile (`strict-sqlite`, `strict-postgres`, or
`strict-mysql`; MariaDB uses the MySQL protocol). This prevents an implicit
SQLite default from masking the chosen backend and keeps the other drivers,
including bundled SQLite, out of the build. Turso-primary and database-free
profiles keep `rullst-orm`'s default drivers for its `AnyPool`.

#### Generated-project verification boundary

The repository does not treat template rendering as sufficient evidence. A
structural contract materializes 18 internal blueprint/profile shapes (nine
public directly linked layouts plus nine legacy DLL layouts retained for regression) and checks paths, Rust syntax and manifests. A slower eight-case set
crosses every blueprint, hot and non-hot layouts, database/API boundaries and a
release build, runs every generated test target, and constructs the public
router of each hot-reload project using offline-safe defaults. A separate
seven-case test invokes the public `cargo rullst new` binary and verifies exact
feature selection for SQLite, PostgreSQL, MySQL, MariaDB, AI, Redis, Turso,
MongoDB, DuckDB, SurrealDB and Qdrant across all six public blueprints plus a
polyglot profile. The invocation starts outside the source checkout, proving
that an unpublished pre-release CLI retains its exact matching checkout as a
path source instead of requesting unavailable registry packages.

The public polyglot profile uses `cargo check` in that CLI-level set because a
second bundled-DuckDB test build adds no adapter behavior and can consume
several GiB on small machines. DuckDB, MongoDB, SurrealDB, Turso and Qdrant
runtime behavior is exercised by their dedicated ORM matrices instead. These
gates prove reproducible local generation and bounded offline construction;
they do not prove provider accounts, production deployment, browser behavior
or application-specific authorization.

The LMS blueprint generates a small starter: catalog, courses, modules,
lessons, an accessible player, enrollment, progress, login and a Nexus admin.
Like every starter it reloads under `cargo rullst dev`/`dash` (the retired
`--hot-reload` DLL profile is rejected for every blueprint). It has no module
profiles; v13 retired `--lms-modules` and the earlier complete Academy
scaffold.

```bash
cargo rullst new academy --default --blueprint lms --skip-initial-migration
```

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

The v13 CLI's `rullst-upgrade-rules-v4` catalog (98 rules) parses Rust sources
(with `syn`; comments, doc comments and strings never match an API rule), Cargo
manifests and generated project files, and reports each finding as
`MUST-CHANGE` (a compile-breaking or API-shape change) or `REVIEW` (changed
behaviour of an API, feature or configuration in use) with a stable `V13-*`
code, `file:line`, a one-line message and the title of its row in the
[v13 migration guide](migration-v13.md#changes-from-the-published-1210-source).
Findings do not stop the transaction; `update project verify`/`review`/`apply`
refuse a preparation with must-change findings. In JSON, each
`source_findings` element keeps `path`, `line`, `code`, `severity` (`BLOCKER`
or `REVIEW`) and `message` and adds `kind`, `migration_row` and
`migration_url`; the root adds `finding_counts`, `unscanned_sources` and
`migration_guide`. `cargo rullst ai upgrade` proposes reviewed fixes for the
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

### `cargo rullst update check` (since 12.1.0)

Advisory release discovery; it does not install a CLI or migrate an application.

```bash
cargo rullst update check
cargo rullst update check --to VERSION --json
cargo rullst update check --refresh
cargo rullst update check --offline --json
cargo rullst update check --no-cache
```

The default stays in the installed major's stable channel. Exact other-major
targets require `--allow-major`; prereleases also require `--prerelease`.
Downgrades, yanked targets and ambiguous metadata fail closed. The bounded
HTTPS query reports the selected release's declared Rust minimum and the
current platform; it does not prove compatibility or artifact authenticity.
On Linux/macOS, explicit discovery caches validated metadata for up to six
hours under `$XDG_CACHE_HOME/rullst-update-v1` or
`$HOME/.cache/rullst-update-v1`. An unsafe owner, permissions, linked file,
oversized body or invalid timestamp prevents reuse. The directory is private
and writer contention does not block discovery; an unusable cache falls back
to the registry only when online. Cache failures never authorize an install.

`--offline` and `CARGO_NET_OFFLINE=true` use only a fresh cache and fail without
network access if none is usable. `--refresh` bypasses cache reads;
`--no-cache` disables both cache reads and writes. Neither bypasses the offline
environment setting. Windows uses `%LOCALAPPDATA%/rullst-update-v1` with an
atomically created protected user/SYSTEM/Administrators DACL. Handle-based
checks reject foreign owners, unsafe grants, reparse points and hard links;
UNC paths and alternate data streams are unsupported. Unsafe ACLs are not
modified. Native cache acceptance is recorded in the
[maintenance checkpoint](v12.md#1210-delivery-checkpoint-unreleased).
Ordinary dashboard notices remain process-local and never write this cache.

`--json` uses `rullst.update-discovery.v1`, includes metadata source/age and
grants no installation, project or deployment authority. Cached metadata is
not proof of current yank status or artifact authenticity. See the
[upgrade guide](tutorials/36-assisted-framework-upgrades.md).

<a id="cargo-rullst-update-verify-1210-working-source-unreleased"></a>

### `cargo rullst update verify` (since 12.1.0)

Authenticate a downloaded native CLI inventory and both executables:

```bash
cargo rullst update verify --to VERSION --directory ./downloaded-cli --json
```

This requires the exact release's `cli-manifest-TARGET.json`,
`cargo-rullst-VERSION-TARGET[.exe]` and `rullst-VERSION-TARGET[.exe]`.
The directory may contain other downloads; only these fixed names are read.
No archive is extracted and no downloaded executable is run. Releases since
12.1.0 publish this inventory for Linux x64 GNU, Windows x64 MSVC and macOS
x64/ARM64; 12.0.0 has none, and no v13 release has been published yet.

The caller-installed GitHub CLI must be available through an absolute trusted
PATH entry and support [attestation verification](https://cli.github.com/manual/gh_attestation_verify).
The command checks a temporary private copy of the manifest against the exact
official repository, release workflow, source tag/commit and GitHub issuer,
rejects self-hosted attestations, and compares both binary sizes and SHA-256
digests. Missing/failed/timed-out verification is an error, with no fallback
to checksums. It requires network access; `--offline` and `CARGO_NET_OFFLINE`
reject before filesystem/network I/O. Major/prerelease opt-ins match `check`.

JSON uses `rullst.update-verification.v1`. Only `artifact_verified` is true;
installation, execution, project writes and deployment remain unauthorized.
The report covers the bytes read during this invocation, is not an installation
token and does not recheck registry yank status. A later installer must validate
current release eligibility and reread/reverify the candidate. Hostile same-user
writers and a compromised verifier/PATH are outside this boundary.

### `cargo rullst update stage` (since 12.1.0)

```bash
cargo rullst update stage --to VERSION --json
```

This exact version must be published and non-yanked. Another major requires
`--allow-major`; a prerelease separately requires `--prerelease`. `--offline`
and `CARGO_NET_OFFLINE` reject before filesystem/network work. The command reads
fresh registry metadata, downloads the official platform manifest, authenticates
it with the caller-installed GitHub CLI, then downloads the two named binaries.
Redirects stay on HTTPS GitHub/release-assets hosts, with at most two redirects.
The manifest is limited to 16 KiB; each binary to its authenticated size and at
most 128 MiB. Each request has a 120-second total timeout. Exact hashes and sizes
must match. Ordinary failures remove the private stage. Forced termination can
leave an incomplete private directory; subsequent stages never reuse its files.

The `rullst.cli-staging.v1` report names the retained private directory and source
identity. Files remain unexecuted and uninstalled. No project files change. This
report grants no future installation authority: an installer must revalidate the
release, provenance and file contents. Native staging checks remain release
requirements.

<a id="cargo-rullst-update-guided-1210-working-source-unreleased"></a>

### `cargo rullst update guided` (since 12.1.0)

```bash
cargo rullst update guided --to VERSION --scope both \
  --root "$HOME/.local/share/rullst-cli" --project ./my-app
cargo rullst update guided --to VERSION --scope project --project ./my-app --offline
```

This interactive entry point composes the same authenticated installation and
isolated project commands described below. `--scope cli`, `project` or `both`
selects the work; `both` is the default and requires an absolute `--root`.
Each complete review appears before its own default-no confirmation. The flow
reuses the exact version, directories and review digests without shell commands
or manual copying. It reports elapsed milliseconds per executed stage, excluding
time spent answering prompts. Declining stops before the next operation;
completed steps and their recovery records remain available.

CLI download/review uses the network. Project verification separately asks
whether Cargo may use the network and requires explicit consent to execute
trusted build scripts, macros and tests. `--offline` keeps project verification
offline and rejects scopes containing CLI installation. Feature selection and
command deadlines use the same `--all-features`, `--features`,
`--no-default-features` and `--timeout-seconds` options as project verification.
The running CLI cannot acquire another major's migration rules by installing
it: use `--scope cli --allow-major` first, then explicitly invoke the new CLI's
project flow. PATH, databases and deployments are not changed.

Piped input/output rejects before I/O; use the explicit commands with JSON and
review digests for automation. This flow does not bypass release eligibility,
provenance, ownership or recovery checks.

### `cargo rullst update install review` (since 12.1.0)

```bash
cargo rullst update install review --to VERSION --directory STAGED_FILES \
  --root "$HOME/.local/share/rullst-cli" --json
```

The absolute installation root must be new/empty or contain exactly the two
updater-owned binaries and its bounded root-bound installation receipt. Its
parent must already exist and pass owner/ancestor checks. An existing destination
must be private (`0700` on Unix or a protected caller DACL on Windows). Unknown
files and package-manager installations are refused. No permissions are repaired.
The original attested manifest bytes and private single-link binaries must still
match; review reauthenticates the prior manifest and uses the installed version
to reject downgrades even when the reviewing CLI is older.

Review fetches fresh registry eligibility, verifies official provenance and
exact local binary hashes, then rechecks the destination. JSON binds source,
version, native target, destination and exact prior receipt to `review_sha256`, lists the future
version probes and supplies an exact Cargo source-install command for separately
reviewed manager use. Offline mode rejects before I/O. The destination is not
created and neither the probes nor fallback command run. The digest grants no
writes until passed explicitly to `apply`.

### `cargo rullst update install apply` / `recover` (since 12.1.0)

```bash
cargo rullst update install apply --to VERSION --directory STAGED_FILES \
  --root "$HOME/.local/share/rullst-cli" --approved-review REVIEW_SHA256 --json
cargo rullst update install recover --root "$HOME/.local/share/rullst-cli" \
  --approved-review REVIEW_SHA256 --json
```

Apply repeats registry, provenance, file and destination checks. It refuses a
changed review. A destination-local lock coordinates installations even when
callers use different advisory caches. Private copies on the destination
filesystem must pass both approved `--version` checks (15 seconds and 4 KiB per
output stream each) before any installed entry is replaced. The CLI retains
verified predecessor bytes and a bounded intent, then replaces the two binary
entries and receipt. A running older executable is moved aside, never truncated.
The three replacements are individually performed; this is not an atomic swap
or a power-loss durability guarantee.

An interrupted replacement reports the same root and approval digest needed for
recovery. Recovery authenticates the recorded manifests again, accepts only the
recorded before/after states and restores the exact predecessor. For a first
installation, it removes only its recorded new entries. Unrelated edits reject
without being overwritten; repeated successful recovery is a no-op. Both
commands reject offline mode before I/O. They never change PATH, compile source,
migrate a project or deploy an application.

Keep the private `.rullst-install-*` sibling: it holds the destination lock,
receipt ownership record and recovery evidence. The selected operation remains
recoverable. Later installs prune only verified older completed/recovered
operations; at most eight operation directories may be retained. Unknown or
incomplete historical evidence requires manual review. On Windows, closing an
older running CLI may be necessary before its historical executable is pruned.
Native fault and complete user-journey acceptance remain release requirements.

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
  * `--api`: Instead of returning HTML Views via the `html!` macro, the generated methods will automatically extract/return `Json<T>`.

### `cargo rullst make:model <name>`
Creates a model struct in `src/models/` with the ORM annotations. SQLx projects
receive `FromRow` plus `Orm`; Turso-primary projects receive
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

### `cargo rullst make:age-gate` (unpublished v13 preview)

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

### `cargo rullst make:privacy` (unpublished v13 preview)

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

### `cargo rullst make:omni`
Generates a Tauri/Omni shell and development configuration for desktop, Android
or iOS. Interactive use prompts for platforms. Automation can select one or
more targets deterministically:

```bash
cargo rullst make:omni --platform desktop
cargo rullst make:omni --platform android \
  --backend-url http://10.0.2.2:3000 --identifier com.acme.myapp
cargo rullst make:omni --platform ios \
  --backend-url https://app.example.com --identifier com.acme.myapp
cargo rullst make:omni --platform desktop,ios \
  --backend-url https://app.example.com --identifier com.acme.myapp \
  --product-name "Acme App" --app-version 1.2.3
```

Mobile generation requires an explicit backend URL. HTTPS is required except
for the bounded localhost/Android-emulator development hosts; embedded
credentials are rejected. Mobile also requires an application-owned lowercase
reverse-DNS `--identifier`; reserved framework and `com.example` placeholders
are rejected. `--product-name` and `--app-version` are optional validated
overrides and otherwise inherit the host package metadata. Desktop-only
development can derive a documented `com.example` placeholder, which must be
replaced before distribution.

The generator installs an exact Tauri npm CLI, creates platform icons,
initializes mobile targets non-interactively, emits a restrictive local CSP and
fails if a requested prerequisite step fails. iOS initialization requires
macOS and Xcode. Native-side navigation is restricted to the packaged
bootstrap and the configured backend's exact origin. Remote pages receive no
privileged Tauri IPC surface; cross-origin OAuth/external-link behavior needs a
separate reviewed system-browser/deep-link integration.

The canonical product remains the Rullst web application and the generated
client packages that application; it does not by itself
implement native plugins, offline synchronization, production network policy,
release signing, privacy declarations, physical-device validation, Play
Store/App Store publication or review acceptance. The generated README contains
the application-owned distribution checklist. Path-aware repository workflows
generate fresh desktop, Android and iOS shells and compile only their declared
targets; those runs are packaging evidence, not store, physical-device or
universal behavior guarantees.

### `cargo rullst make:iot <DeviceName>`
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

### `cargo rullst make:grpc <ServiceName>`
Scaffolds a new gRPC service implementation in `src/grpc/<name>.rs` and Protobuf schema definition in `proto/<name>.proto` powered by `tonic`.

### `cargo rullst deploy [--platform <fly|railway|render|vps>]`
Guided deployment helper that generates cloud manifests (`fly.toml`,
`railway.json`, `render.yaml`, or `docker-compose.prod.yml`) and invokes the
selected provider CLI where supported. A provider CLI that is not installed
only prints the manual commands; one that runs and fails (`flyctl deploy`,
`railway up`) makes `deploy` exit non-zero. The Fly.io `app` name uses the same
lowercase label as `make:k8s` (`my_app` becomes `my-app`), while the
`Dockerfile` and Railway start command keep the package's binary name. An
unknown `--platform` value is rejected before anything is written, including
the `Dockerfile` the command otherwise scaffolds when it is missing.
`--platform vps` pins the Caddy container to `172.31.250.10` on a
`172.31.250.0/24` compose network and adds that address to `[security]
trusted_proxies` in `Rullst.toml` (creating the file before a missing
`Dockerfile` is scaffolded, so the image copies it), so rate limits see each
client instead of Caddy. An existing `trusted_proxies` list is left unchanged
with a warning. Fly.io, Railway and Render print a notice while `Rullst.toml`
trusts no proxy, because their proxy networks are provider-specific.
Credentials, migrations, availability, DNS/TLS and rollback remain operator
responsibilities.

### `cargo rullst auth`
Creates an authentication starting point in your codebase, including:
- User model and migration with asynchronous Argon2 password hashing.
- Auth Controllers (Login, Registration, Logout).
- Encrypted-session middleware that inserts the signed-in user's id as `Extension<i32>`.
- HTML Views for Login and Signup.

It targets the SQLx ORM: Turso-primary projects are rejected. The command
enables the `orm` and `auth` umbrella features, registers the generated
`controllers`, `middlewares`, `models` and `pages` modules in `src/lib.rs` (or
`src/main.rs`), and refreshes the migration registry. It fails before writing
anything when `src/models/user.rs`, `src/controllers/auth_controller.rs`,
`src/middlewares/auth_middleware.rs` or `src/pages/auth.rs` already exists, or
when a `*_create_users.rs`/`*_create_users_table.rs` migration already creates
the users table (the blank database starter and the SaaS/LMS blueprints ship
one). It also refuses a project without a SQL migration runner (`pub mod
migrations;` in `src/main.rs` or `src/lib.rs` plus a
`rullst::artisan!(crate::migrations::get_migrations())` call, which database
starters include), such as a `--no-database` starter, because nothing would
compile or apply the users migration. Generated code names the ORM through the
`rullst::orm` facade, so no direct `rullst-orm` dependency is needed. Mounting
routes and the security baseline remains application work.

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

### `cargo rullst ai` (v13 preview)
A terminal assistant that knows Rullst and the current project. It answers
questions and proposes changes that you review one by one. A walkthrough is in
[the terminal AI assistant guide](ai-assistant.md).

```bash
cargo rullst ai connect                # choose provider, model and key (interactive)
cargo rullst ai                        # streaming chat in the current project
cargo rullst ai "add a posts page"     # one goal, then exit
cargo rullst ai status [--json]        # provider, model and key source (never the key)
cargo rullst ai disconnect             # delete the saved credentials file
cargo rullst ai upgrade [--to <VERSION>] [--dry-run]  # reviewed fixes for upgrade findings
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
  `db:migrate`, `doctor` without `--fix`, `audit` without `--network`,
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
* **Upgrade sessions:** `cargo rullst ai upgrade` (flags `--to`, `--provider`,
  `--model`, `--dry-run`) prints the `cargo rullst upgrade --dry-run` plan and,
  when it has source findings, starts one goal grounded in them: the trusted
  instructions carry only the migration rows those findings reference; the
  findings, each flagged line and the affected files (within the path policy)
  are untrusted data. Fixes go through the same diff, confirmation, checkpoint
  and `cargo check` flow; Rullst dependency versions stay with
  `cargo rullst upgrade`. Without findings no session starts. Offline, the
  demo rewrites one `render_page` call (`V13-RENDER-PAGE-LANGUAGE`) and
  proposes `cargo check`. A one-word chat goal `upgrade` now runs this command.
  See [assisted fixes](tutorials/36-assisted-framework-upgrades.md#assisted-fixes-with-cargo-rullst-ai-upgrade).
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

### `cargo rullst dockerize` / `cargo rullst nixify`
Injects infrastructure files into a pre-existing project (similar to the flags
used in `new`): `dockerize` writes a `Dockerfile` (plus `.dockerignore` when
absent), `nixify` writes `flake.nix` and `.envrc`, and `generate:buildah`
writes `build_buildah.sh`, which builds the `Dockerfile` rootless with
`buildah bud`. All three refuse to replace an existing file;
move a customized file aside to regenerate its template. The Dockerfile's binary
and the Buildah image are named after `[package].name`, read with a TOML parser
(`app` when `Cargo.toml` has no package name).

### `cargo rullst foundry:init`
Generates the `Foundry.toml` deployment manifest at the project root containing
SSH access settings and environment variables for a compatible systemd-based
Linux VPS. Before writing it, the command creates `.gitignore` when missing and
appends `Foundry.toml` unless an exact, non-negated `Foundry.toml` line already
ignores it. The manifest is created owner-readable only (`0600` on Unix);
operators must still verify that secrets were never committed.

### `cargo rullst foundry:deploy`
Executes an SSH deployment pipeline: local release build, remote directory and
systemd provisioning, `scp` transfer, environment/Caddy configuration, service
restart, and a bounded remote-local `/health` probe. It requires a preinstalled,
reviewed `curl`, systemd, and Caddy installation plus root or passwordless
non-interactive `sudo`. Candidate files are staged under an application-specific
`/opt/rullst/<app>` root: the binary is the executable Cargo reports for the
package (so `CARGO_TARGET_DIR`, `build.target-dir`, a workspace target directory
and `build.target` are honored; with several binaries, `package.default-run` or
the one named after the package is chosen), uploaded into
`/opt/rullst/<app>/incoming` (mode `0700`, owned by the SSH user), and its
owner and the SHA-256 of the local build are checked before it is installed.
The Caddy configuration is validated, and `.previous` copies of replaced files
are retained. The service environment file holds the `[env]` table plus
`PORT`, the `[app] port` (default 3000) that Caddy proxies to and the health
check probes, unless `[env]` sets `PORT` itself; an `[env] PORT` different from
`[app] port` is rejected. It also sets `HOST="127.0.0.1"` unless `[env]` sets
`HOST` or `RULLST_HOST`: a production server otherwise binds `0.0.0.0`, and only
Caddy needs that plain-HTTP port. Keep it private when overriding `HOST`. The
application runs as a dedicated
`rullst-<app>` system account (created with `useradd`) under a sandboxed unit
(`NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, no
capabilities except `CAP_NET_BIND_SERVICE` for a port below 1024) and can write
only under `/opt/rullst/<app>/data`. The current command replaces the global
`/etc/caddy/Caddyfile`; it does not perform migrations, data backup, external
reachability check, or automatic rollback. It
does not guarantee zero downtime and does not support IPv6 SCP targets.

### `cargo rullst omni`
The 12.1.0 executable added `cargo rullst omni android --release` for
an explicit Android release build using application-owned signing inputs. It
does not change the existing Rust `Commands::Omni` variant or start a backend.
The v13 development CLI additionally requires `--signing-certificate` and
`--apksigner-jar` (or `RULLST_ANDROID_SIGNING_CERTIFICATE` and
`RULLST_ANDROID_APKSIGNER_JAR`), verifies one
fresh release APK against that certificate and reports its SHA-256. Use
`--apk` to select a relative output when variants are ambiguous and
`--android-arch` to restrict the native build. Neither a successful build alone
nor a previous unchanged artifact is accepted as fresh verified output.
See [Android signing and icons](tutorials/49-omni-android-signing.md) for key
setup, migration of existing shells and certificate/device verification.

Runs the generated Tauri development client after `make:omni`. Android/iOS
require their official SDK/toolchain and a reachable backend. For `desktop`, the
spinner lasts until the shell prints `Launching Omni interface...` (or 200
output lines arrive); the command then prints the held lines and keeps streaming
the shell's and its managed backend's standard output until the window closes.
For `android` and `ios`, the command resolves the Tauri CLI (the local
`omni-app/node_modules/@tauri-apps/cli` or `cargo tauri`) before it starts the
backend with `cargo run`, so a missing CLI fails without leaving a backend
process on port 3000; the backend is stopped when the mobile client exits.
* **Optional Arguments:** `<target>` specifies where to run (e.g., `desktop`, `android`, `ios`).

---

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
  * `--json`: Moves the progress lines to standard error and prints one
    `rullst.cli-audit.v1` summary on standard output; the exit status is
    unchanged (non-zero for findings or incomplete requested checks):

```json
{
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

SBOM components come from `Cargo.lock`. Only crates.io packages receive the
plain `pkg:cargo/<name>@<version>` purl; a package from another registry adds a
`repository_url` qualifier, a git package adds a `vcs_url` qualifier with the
locked commit, and path or workspace packages (including the application) have
no purl. Every component that is not from crates.io carries a
`rullst:cargo:source` property with its lockfile source, or `local`.

`SECURITY_COMPLIANCE.md` and `sbom-cyclonedx.json` are written in the current
directory and replace a previous regular file. Because an audit may run on an
untrusted checkout, the command refuses to write either file through a symlink.

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
failure carries a one-line fix and a link to its group below. The command
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
scaffold: v13 removed the complete Academy scaffold, and the LMS starter
generates no evidence file. Without `--evidence` every requirement is
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
  `rullst.cli-routes.v1` document (`--json` is accepted only for routes):

```json
{
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

# Ask the terminal AI assistant (offline demo until `cargo rullst ai connect`)
cargo rullst ai "add a posts page"

# Run the reviewed Foundry pipeline on a compatible, prepared VPS
cargo rullst foundry:deploy
```
