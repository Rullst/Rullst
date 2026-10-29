# 36. Assisted framework upgrades

Rullst v12 introduces a bounded upgrade transaction for existing applications.
The goal is to make the safe, repeatable part a single command while refusing
to guess about application data or security policy.

> The transaction steps below were written for `12.1.0`; check [publication status](../v12.md).
> The v12 line reached end of life on 16 October 2026, so a v12 application
> moves to v13 with the v13 CLI. The v13 source rules, the assisted
> `cargo rullst ai upgrade` session and the rule classification are described
> in [v12 to v13: source findings](#v12-to-v13-source-findings) and belong to
> the `13.0.0-alpha.3` prerelease. Install the exact release and complete
> the application-specific validation below before any production rollout.

## What the command can guarantee

`cargo rullst upgrade` can inventory the Cargo workspace, update the Rullst
release train, apply compiler suggestions, require `cargo check`, and restore
the files it controls after a failure. It cannot prove that a database upgrade,
authorization rule, provider integration or deployment still behaves correctly.

The automatic transaction owns only:

- versioned Rullst dependencies in exact Cargo workspace manifests;
- the root `Cargo.lock` produced by Cargo resolution;
- Rust edits proposed by `cargo fix`;
- a `cargo check --workspace --all-targets --locked` gate using the application's
  selected features.

The 12.1 updater writes managed requirements as `=VERSION`, preserving the
explicitly selected release instead of allowing Cargo to choose a later patch
or minor version. `cargo fix` resolves the candidate lockfile, and the final
check must use that resolution unchanged. Review these pins before restoring
any broader dependency-update policy in your application.

It never runs migrations, changes secrets, invents tenant/ownership policy,
opens Nexus or Studio, contacts application providers, or marks the result
production-ready.

## 1. Prepare the application

Create a branch, make the worktree reviewable, record the old test result, and
back up every database. Prove the database backup can be restored before
changing the framework.

Install the exact CLI from the same release train as the target framework.
For a v12 application, that is the v13 prerelease (Cargo never selects a
prerelease on its own):

```bash
cargo install cargo-rullst --version =13.0.0-alpha.3 --locked --force
```

The framework command does not update its own executable. This matters for v5:
the already-published v5 CLI cannot gain the v12 migration engine
retroactively, and the v13 CLI upgrades only from v12 or later. A v5
application first moves to v12 with the end-of-life 12.x CLI
(`cargo install cargo-rullst --version 12.1.0 --locked --force`, as the steps
below show), then to v13. Run each CLI inside the application.

## 2. Inspect without writing

```bash
cargo rullst upgrade --dry-run
```

The plan shows every dependency edit and source finding. `MUST-CHANGE`
(`BLOCKER` in the 12.x CLI) means a known old API or option requires a source
change; `REVIEW` means the code uses an API, feature or configuration whose
behaviour changed and must be revalidated. Neither label means that unreported
code is automatically safe.

For automation, request JSON:

```bash
cargo rullst upgrade --dry-run --json > upgrade-plan.json
```

The root object uses `schema_version: "rullst.upgrade-plan.v1"` and identifies
the rule catalog, exact target, manifest changes, detected source majors,
findings, automatic scope and mandatory manual gates. Consumers must reject an
unknown schema version rather than silently interpreting it as v1. The v13 CLI
extends v1 additively (see [the v13 fields](#json-plan-fields)); consumers must
ignore keys they do not know.

Use an explicit target to make the selected published version visible:

```bash
cargo rullst upgrade --to 12.1.0 --dry-run
```

Other targets must exist in the registry and belong to the installed CLI's
major train. This restriction prevents a v12 rules engine from pretending it
understands an eventual v13 migration.

## 3. Apply the transaction

After resolving or accepting every finding:

```bash
cargo rullst upgrade
```

Before the first write, the CLI snapshots Cargo workspace manifests, the root
lockfile and Rust sources under:

```text
target/rullst-upgrades/<UTC-run-id>/
├── files/
├── index.tsv
├── report.md
└── report.json
```

It then edits the TOML while preserving comments and relative order, runs
compiler-provided fixes and executes the Cargo check gate. A failing gate
restores the controlled files automatically and returns a non-zero status.

To deliberately keep a partial result for diagnosis:

```bash
cargo rullst upgrade --keep-on-failure
```

To restore a persisted snapshot after that mode or after an interrupted run:

```bash
cargo rullst upgrade \
  --restore target/rullst-upgrades/<UTC-run-id>
```

Restore accepts only a path-validated snapshot inside the current project's
`target/rullst-upgrades` directory. `cargo clean` deletes `target`, so retain a
normal version-control commit or copy a needed diagnostic report before
cleaning.

### Recovery boundaries and 12.1.0 hardening

Stop editors, watchers and other writers before restoring. File recovery does
not undo build-script/test side effects or database/external-service changes.
Keep an independent version-control backup; the directory under `target` is not
a substitute for one. A filesystem error during application can leave some
files restored and others unchanged, so always review the result.

Since 12.1.0 the CLI preflights the entire backup, stages every replacement
before writing originals, and rejects linked or malformed paths. It limits
indexes to 8 MiB/100,000 entries, each snapshot to 64 MiB and the total to
512 MiB. Disk failure while staging leaves originals intact; failure during
the later per-file apply reports progress and retains the backup. This is not
an all-files atomic commit and does not defend against hostile concurrent
filesystem changes. These improvements are **not in 12.0.0**.

## 4. Finish a v5 to v12 migration

This step uses the v12 CLI. The v13 CLI accepts only v12 and v13 projects, so
upgrade older applications to v12 first and then to v13.

The v5 README used attribute-style routing and a server builder with no router
or port. The scanner reports these markers instead of applying a global text
replacement. Replace the old shape:

```rust,ignore
#[routes]
fn home() -> Response {
    // ...
}

Server::new()
    .route("/", get(home))
    .run()
    .await;
```

with an explicit v12 router and typed error propagation:

```rust,no_run
use rullst::{Server, response::Html, routes};

async fn home() -> Html<&'static str> {
    Html("Hello from v12")
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![get("/" => home)];
    Server::new(app).run(3000).await?;
    Ok(())
}
```

Then follow the complete [v5 → v12 guide](../migration-v5-to-v12.md), including
feature selection, disposable database migration/rollback, explicit Nexus and
Studio boundaries, provider validation and authorization negatives.

## 5. Run the application-owned gates

At minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release
```

Also test the actual production feature set, restore/migrate/rollback against a
production-shaped database copy, cross-user and cross-tenant denials, proxy/TLS
identity, CSRF/CORS, Nexus/Studio exposure and every configured live provider.

## v12 to v13: source findings

The v13 CLI carries the `rullst-upgrade-rules-v4` catalog. It parses the
application instead of matching text: Rust files with `syn` (so comments, doc
comments and string literals never match an API rule, and macro bodies such as
`html!` and `routes!` are read token by token), Cargo manifests with a TOML
parser, and a few generated project files (`.gitignore`, `.dockerignore`,
`.cargo/config.toml`, `Foundry.toml`, `docker-compose.prod.yml`, Kubernetes
YAML, an Omni shell under `omni-app/`, `rullst-client.ts`, `Dockerfile`,
`.env`/`.env.example` keys and release scripts). Files are read without
following symbolic links; a finding records a rule and a line, never file
contents. A Rust file that does not parse, is not UTF-8 or is larger than
2 MiB is listed as `NOT SCANNED`.

Each finding has a stable code (`V13-...`), a kind, a location, a one-line
message and the first-column title of its row in the
[v13 migration guide](../migration-v13.md#changes-from-the-published-1210-source):

```text
Source findings (rullst-upgrade-rules-v4): 1 must-change, 2 review
  MUST-CHANGE src/views/home.rs:12 [V13-HTML-DYNAMIC-EVENT-HANDLER] a dynamic value in an `on*`/`hx-on*` attribute of `html!` no longer compiles
      migration-v13 row: `html!` event handlers
  REVIEW src/main.rs:64 [V13-RENDER-PAGE-LANGUAGE] `render_page` declares `lang="pt-BR"`; English pages should use `render_page_with_lang`
      migration-v13 row: Starter page language
```

- **must-change**: a compile-breaking or API-shape change that the code uses
  (for example a dynamic `onclick={...}` in `html!`, a `belongs_to` without
  `foreign_key`, a struct-level `#[sqlx(rename_all)]` on a model, a `match` on
  `FieldKind` without a wildcard, a utoipa 5 dependency used with Studio).
- **review**: the code uses an API, feature or configuration whose behaviour
  changed (for example `Model::all()`, `ValidatedJson`, the `Mail` facade,
  `render_page`, a generated `.gitignore` without the SQLite journal files).

A review rule is reported once per file (its first location); a must-change
rule at every location. The [rule classification](#v12--v13-rule-classification)
lists which migration rows have rules. Findings never block `cargo rullst upgrade`, whose
`cargo check` gate still restores the project when the code does not compile.
The gate's error names the must-change findings and repeats the guidance of
`V13-CAPITAL-REMOVED` and `V13-MAIL-REMOVED` findings, which name removed APIs
but are review findings.
`update project verify`, `review` and `apply` refuse a preparation that has
must-change findings and carry review findings in the plan.

### Rules by release

Each rule records the release that introduced the change it reports. The
rules for the 12.x to 13 changes date from 13.0.0-alpha.1; a rule added later,
or reworded because a later release changed what it reports, carries that
release:

| Release | Rules |
| :--- | :--- |
| 13.0.0-alpha.1 | The other 95 rules |
| 13.0.0-alpha.2 | `V13-HTML-MARKUP-RETURN`, `V13-HTML-RAW-MARKUP`, `V13-ORM-CONSTRAINT-ERRORS`, `V13-TEXT-TIMESTAMPS`, `V13-POLYGLOT-FEATURE` |
| 13.0.0-alpha.3 | `V13-CONNECT-GITHUB-EMAIL`, `V13-ORM-TRANSACTION-ERRORS`, `V13-ORM-SKIP-MANAGED-COLUMNS`, `V13-ORM-DEFAULT-FEATURES` (reworded: `rullst-orm` has no default driver), `V13-RULLST-DEFAULT-FEATURES`, `V13-OMNI-REMOVED`, `V13-OFFLINE-SYNC-REMOVED` |
| 13.0.0-alpha.4 | `V13-DEPLOY-TARGETS-REMOVED`, `V13-DEPLOY-GENERATORS-REMOVED`, `V13-CLI-SELF-UPDATE-REMOVED`, `V13-CLI-EXTRAS` |

A rule is dated by the first release tag whose catalog contains its code
(`git show v13.0.0-alpha.2:cargo-rullst/src/generators/build/upgrade/rules/catalog.rs`,
or `git log -S'"V13-..."'` on that file), checked against the CHANGELOG entry
that names it; a new rule takes the next release. A unit test checks that every
rule names one of these releases.

The plan takes the application's Rullst version to be the oldest version that
a Rullst requirement in the workspace manifests admits (`=13.0.0-alpha.3`,
or `13.0.0-alpha.1`, a caret requirement) or that `Cargo.lock` holds. When that
version is a 13.0 prerelease, only the rules introduced after it apply: an
alpha.3 application gets the alpha.4 rules, an alpha.2 application the seven
alpha.3 rules and the alpha.4 rules, and an alpha.1 application the alpha.2,
alpha.3 and alpha.4 rules, because the earlier changes are already in the
version it runs. The plan says what it skipped:

```text
Source findings (rullst-upgrade-rules-v4): 0 must-change, 1 review
  Rules: introduced after 13.0.0-alpha.2, the Rullst version this project uses; earlier rules skipped: 100 (findings hidden: 3); --all-rules applies every rule
```

A v12 application, or one whose version cannot be read (path dependencies
without a version and no `Cargo.lock`), gets every rule. `--all-rules` (also
accepted by `cargo rullst ai upgrade`) applies every rule to a prerelease
application too, for example to review the 12.x to 13 changes again. Project
preparations (`cargo rullst update`) always apply every rule, because their
must-change findings gate execution.

### JSON plan fields

`--dry-run --json` keeps every `rullst.upgrade-plan.v1` key. Each element of
`source_findings` keeps `path`, `line`, `code` (the stable rule id), `severity`
(`BLOCKER` or `REVIEW`) and `message`, and adds:

| Key | Meaning |
| :--- | :--- |
| `kind` | `must-change` or `review` |
| `introduced_in` | The release that introduced the rule's change, such as `13.0.0-alpha.3` |
| `migration_row` | First-column title of the row in `migration-v13.md` |
| `migration_url` | The published table of the v13 migration guide |

The root object adds `finding_counts` (`must_change`, `review`),
`unscanned_sources` (project-relative paths), `migration_guide` and
`rule_selection`: `current_version` (the application's Rullst version, or
`null` when it cannot be read), `all_rules` (`--all-rules`, and always `true`
in project preparations), `skipped_rules` (rules introduced in or before
`current_version` that were not applied) and `hidden_findings` (the findings
those rules would have added). A consumer that ignores unknown keys reads the
plan as before; findings of skipped rules are absent from `source_findings`
and `finding_counts`.

## Assisted fixes with `cargo rullst ai upgrade`

`cargo rullst ai` is part of the optional `cargo-rullst-extras` package since
13.0.0-alpha.4; install it with the same version as the CLI
(`cargo install cargo-rullst-extras --version =13.0.0-alpha.3 --locked`).

```bash
cargo rullst ai upgrade                 # plan, then a reviewed session
cargo rullst ai upgrade --dry-run       # show the proposed fixes only
cargo rullst ai upgrade --provider anthropic --model <name>
```

The command computes the same dry-run plan (nothing is written) and prints it.
Without findings it stops there. Otherwise it starts a `cargo rullst ai`
session whose trusted instructions contain only the migration rows of the
reported findings, with their fix guidance; the findings (and the text of each
flagged line) and the affected files are sent as untrusted data, subject to the
assistant's path policy (`.env`, `.cargo/`, `Cargo.lock` and other protected
files are never read). The assistant proposes fixes through the normal action
protocol: each edit is shown as a diff and confirmed, a git checkpoint is
stored before the first change, and `cargo check` is proposed after the edits.
Without an interactive terminal, or with `--dry-run`, nothing is executed.
Rullst dependency versions stay owned by `cargo rullst upgrade`.

It is a subcommand of `ai` rather than an `upgrade --assist` flag because it
needs the assistant's provider, model, credential and terminal handling, while
`cargo rullst upgrade` stays deterministic and keeps its published command
(`UpgradeOptions` only gained `all_rules` in 13.0.0-alpha.3). A one-word chat goal of `upgrade` now opens this
command; phrase such a goal with more words.

With no provider connected, the deterministic offline assistant demonstrates
the flow on `V13-RENDER-PAGE-LANGUAGE`: it rewrites the first flagged
`render_page(&htmx, title, body)` call as
`rullst::htmx::render_page_with_lang(&htmx, "en", title, body)` and proposes
`cargo check`. It proposes no edits for the other findings.

A practical order:

1. `cargo rullst upgrade --dry-run` and read every finding with its migration row.
2. Fix findings that also compile against v12 (most review items and the
   `html!`, ORM and Nexus must-change items) with `cargo rullst ai upgrade` or
   by hand, then run the v12 tests.
3. `cargo rullst upgrade`. When a fix needs a v13 API (for example utoipa 6
   with Studio), run `cargo rullst upgrade --keep-on-failure`, then
   `cargo rullst ai upgrade` on the kept state, and finish with
   `cargo rullst upgrade` (no further version edits; it validates the result).
4. Run the application-owned gates below.

## Rehearsal on generated starters

A rehearsal generates starters with a published CLI, points their Rullst
dependencies at a checkout of the next version through `[patch.crates-io]`
(the registry requirements stay, so the upgrade edits them as it would for a
published release), runs `cargo rullst upgrade` with the newer CLI and then
each project's build, tests and Clippy.

### 13.0.0-alpha.2 to 13.0.0-alpha.3

The October 2026 rehearsal used the published `v13.0.0-alpha.2` Linux binary,
checked with `sha256sum -c cli-checksums-x86_64-unknown-linux-gnu.txt` from the
same release, and a CLI built from `main`; the projects built with Rust
1.99.0. Because `main` still carried the alpha.2 version, the rehearsal
exported it (`git archive`), set every package to `13.0.0-alpha.3` in that
copy, patched the projects to the copy and passed `--to 13.0.0-alpha.3`. The
published alpha.3 CLI targets its own version, so `--to` is not needed there.

```bash
./cargo-rullst-13.0.0-alpha.2-x86_64-unknown-linux-gnu rullst new a2-saas --default \
  --blueprint saas --database sqlite --skip-initial-migration
cargo build && cargo test            # on 13.0.0-alpha.2; commit Cargo.lock
# append [patch.crates-io] entries for every rullst package in Cargo.lock, commit
cargo rullst upgrade --dry-run --to 13.0.0-alpha.3
cargo rullst upgrade --to 13.0.0-alpha.3
# edit what the findings ask for, then
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

Blank, Blank `--api`, SaaS and LMS starters built on alpha.2 without warnings,
and 3, 3, 7 and 11 tests passed. Against the alpha.3 candidate:

| Starter | Must-change | Review | `cargo rullst upgrade` | Manual edits | alpha.3 build, tests and Clippy |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Blank | 0 | 0 | passed; changed only `Cargo.toml` and `Cargo.lock` | none | no warnings; 3 passed; clean |
| Blank `--api` | 0 | 1 (`V13-HOT-RELOAD-MACHINE-ENDPOINTS`) | passed; changed only `Cargo.toml` and `Cargo.lock` | none | no warnings; 3 passed; clean |
| SaaS | 0 | 4 (`V13-ORM-SKIP-MANAGED-COLUMNS`; `V13-REFERRER-NO-REFERRER` in two files, `V13-MAIL-FACADE-CONFIG`) | passed; changed only `Cargo.toml` and `Cargo.lock` | `#[orm(skip)]` to `#[orm(db_default)]` on `User::is_admin` and `User::session_epoch` | no warnings; 7 passed; clean |
| LMS | 0 | 6 (`V13-ORM-SKIP-MANAGED-COLUMNS`; `V13-LMS-RECORD-PROGRESS` in three files, `V13-REFERRER-NO-REFERRER`, `V13-MAIL-FACADE-CONFIG`) | passed; changed only `Cargo.toml` and `Cargo.lock` | the same `User` edit | no warnings; 11 passed; clean |

`V13-ORM-SKIP-MANAGED-COLUMNS` is the only finding about an alpha.3 change.
The other findings locate APIs whose behaviour changed between 12.x and
13.0.0-alpha.2, so they also appear on a new alpha.2 or alpha.3 starter and
need no edit in an application that already runs alpha.2. After the `User`
edit the SaaS and LMS dry runs report 3 and 5 findings, all of that kind.
The rehearsal's CLI applied every rule; the plan now skips the rules an
alpha.2 application already runs with (see [Rules by release](#rules-by-release)),
so its default dry run lists only `V13-ORM-SKIP-MANAGED-COLUMNS` for SaaS and
LMS and nothing for Blank and Blank `--api`, and `--all-rules` lists the
findings in the table.

An HTTP smoke test of each upgraded starter (`db:migrate`, then the server in
development) passed. In SaaS and LMS, registration stored `is_admin = 0` and
`session_epoch = 0` from the column defaults, "Sign out everywhere" raised the
epoch and the earlier cookie was refused, signing in again worked and the
loopback Nexus answered `200`. Blank and the JSON API answered `/` and
`/health`, and the API's bearer `POST /api/messages` returned `201` (`401`
without the token).

The same starters generated by the alpha.3 CLI differ from the alpha.2 ones
only in the `User` fields above, in the localized account mail and `APP_NAME`
of the password controller and `.env` and the checked reset-link lifetime of
`src/models/password_reset.rs` (the "Account mail language and product name"
and "Generated reset link lifetime" rows; existing projects keep their code)
and in the SaaS users migration, which imports through `rullst::db` instead of
`rullst_orm` (both compile). The
[alpha.2 to alpha.3 checklist](../migration-v13.md#upgrading-an-alpha2-application-to-alpha3)
turns this into steps for a real application.

Limits: SQLite only; the starters use none of the other APIs that alpha.3
removes or changes (Omni, `offline-sync`, default features, `Orm::transaction`,
mock Connect credentials), so their rules ran only in the catalog's unit
tests; no live mail, billing or OAuth provider was called.

### Earlier rehearsals: v12 to v13

`.github/rehearse-v12-upgrade.sh` is a manual helper (not a CI job) for the
v12 to v13 path; its header documents the arguments. In October 2026 it and
the published 12.3.0 binary upgraded Blank, Blog, SaaS and LMS starters of the
12.0.0, 12.1.2 and 12.3.0 CLIs to the 13.0.0-alpha.1 workspace with review
findings only (5 to 14 per starter, for example `V13-GITIGNORE-DATABASES`,
`V13-MODEL-ALL` and `V13-HEALTH-PROBES`) and no source edit, with one
exception: the 12.3 SaaS starter still names `LemonSqueezyProvider`, so
`cargo fix` failed and the project was restored until the Lemon Squeezy
branches of `src/controllers/billing_controller.rs` and the `BILLING_STORE_ID`
lines of `.env` and `.env.example` were removed, as `V13-CAPITAL-REMOVED`
describes (that edit also compiles on 12.3). Starters of the 12.0 CLI also
carry the [build files](../migration-v13.md#projects-generated-by-the-120-cli)
listed in the migration guide. Those counts predate the alpha.2 and alpha.3
rules and were not re-measured.

## v12 → v13 rule classification

Every row of the [v13 migration guide](../migration-v13.md#changes-from-the-published-1210-source)
is classified for `rullst-upgrade-rules-v4`:

- **(a) must-change**: a compile-breaking or API-shape change detectable in
  application source (11 rows);
- **(b) review**: changed behaviour worth reviewing when the application uses
  the affected API, feature or configuration, which the rules locate (99 rows);
- **(c) none**: no application impact, or not detectable in the application
  (CLI behaviour, generator output for new projects, opt-in features, fixes
  of inputs that previously failed) (93 rows).

A row with both a must-change and a review part counts in both. The catalog
has 111 rules. A row can map to several rules and a rule to
several rows; when a row changes in a later release, update this table and
the rule together.

| Migration row | Class | Rules | Notes |
| :--- | :--- | :--- | :--- |
| Existing runtime APIs | (c) none | — | General statement; the session-schema migration is the next row |
| Account/session registry | (b) review | `V13-AUTH-RECOVERY-MIGRATE` |  |
| Private object storage | (c) none | — | Opt-in `storage-s3` candidate |
| R2 public URLs | (b) review | `V13-R2-PUBLIC-URL` |  |
| Tenant local storage | (c) none | — | Only a tenant directory below a regular file changes the error; not located |
| Recoverable Live UI | (c) none | — | Opt-in module |
| Distributed tracing | (b) review | `V13-OTLP-ENVIRONMENT` |  |
| Core queue | (b) review | `V13-QUEUE-SEMANTICS` | Unindexed 12.x Redis failures are not located |
| Core scheduler, cache and realtime | (b) review | `V13-PRESENCE-COUNTING` | The `TenantCache` key encoding is not located |
| Core memory cache sweeps | (c) none | — | Internal sweep cadence; reads unchanged |
| Studio queue monitor | (b) review | `V13-QUEUE-DRIVER-PREVIEWS` |  |
| Studio API playground | (a) must-change | `V13-STUDIO-UTOIPA-6` | Must-change when a manifest declares utoipa < 6 or utoipa-axum < 0.3 |
| Studio local access and table view | (b) review | `V13-STUDIO-TABLE-VALUES` |  |
| LMS blueprint | (c) none | — | Generator output; existing applications keep their code |
| Consumer generators | (c) none | — | Generator commands |
| `html!` event handlers | (a) must-change | `V13-HTML-DYNAMIC-EVENT-HANDLER` |  |
| `html!` returns `Markup` | (a) must-change | `V13-HTML-MARKUP-RETURN`, `V13-HTML-RAW-MARKUP` | Located in function signatures and `RawHtml` calls, including `html!` bodies; a `String` collected from `html!` output, an `#[island]` call or `Live::mount` output is reported where `RawHtml` wraps it |
| Core request validation | (b) review | `V13-VALIDATION-STATUS` |  |
| Core feature flags and scheduler | (b) review | `V13-SCHEDULER-WEEKDAYS` | A/B splits: the two feature-flag rules below; the bucket reassignment depends on flag configuration and is not located |
| Memory feature-flag splits | (b) review | `V13-MEMORY-FEATURE-SPLITS` |  |
| Database feature-flag splits | (b) review | `V13-DB-FEATURE-SPLITS` |  |
| Security headers | (b) review | `V13-REFERRER-NO-REFERRER` |  |
| Core WAF signatures | (c) none | — | Only requests that previously failed with 403 change |
| Core security header precedence | (c) none | — | Explicit header values in handlers and inner layers are not located |
| AI provider streaming and stop reasons | (b) review | `V13-AI-GEMINI-STOP-REASONS` |  |
| AI image-beacon guardrail | (c) none | — | Guardrail hardening without an application API change |
| AI RAG tenant tags | (b) review | `V13-AI-RAG-TENANT-TAGS` |  |
| Anthropic provider output | (b) review | `V13-AI-ANTHROPIC-OUTPUT` |  |
| OpenAI provider output | (b) review | `V13-AI-OPENAI-OUTPUT` |  |
| Ollama host | (c) none | — | Only values that previously failed change meaning |
| AI chat memory keys on MySQL/MariaDB | (b) review | `V13-AI-CHAT-MEMORY-KEYS` | The database backend is not located |
| Core CSRF on HEAD | (c) none | — | HEAD previously failed with 500 |
| Hot-reload machine endpoints | (b) review | `V13-HOT-RELOAD-MACHINE-ENDPOINTS` |  |
| Core JSON PII masking | (b) review | `V13-PII-MASKING` |  |
| Core PII masking of range responses | (b) review | `V13-PII-MASKING` |  |
| Security DLP, RASP and honeypot | (b) review | `V13-SECURITY-DLP-HONEYPOT` |  |
| Security audit log lines | (b) review | `V13-SECURITY-AUDIT-LOG-LINES` |  |
| Connect generic OIDC | (b) review | `V13-OIDC-OPTIONAL-NAME` |  |
| Age assurance | (c) none | — | Opt-in APIs and generator |
| Optional consent and export | (c) none | — | Opt-in generator |
| Project context | (c) none | — | CLI generator |
| Typed API generation | (c) none | — | Opt-in CLI profile |
| CLI generator output | (b) review | `V13-TS-CLIENT-REGENERATE` | Other generator changes affect new output only |
| CLI generator robustness | (b) review | `V13-TS-CLIENT-REGENERATE`, `V13-GITIGNORE-DATABASES` | Other generator changes affect new output only |
| ORM database-managed columns | (b) review | `V13-ORM-SKIP-MANAGED-COLUMNS` | Reports `#[orm(skip)]` on `is_admin`/`session_epoch` without `#[sqlx(skip)]`, as alpha.2 starters declared them |
| ORM dirty tracking | (c) none | — | Opt-in `RowSnapshot` field |
| ORM partial updates | (b) review | `V13-ORM-PARTIAL-UPDATE` |  |
| ORM derive checks | (a) must-change | `V13-ORM-BELONGS-TO-KEY`, `V13-ORM-IGNORED-RELATION-KEY`, `V13-ORM-SEARCHABLE-TABLE` |  |
| ORM queries and cache | (b) review | `V13-ORM-SQLX-JSON-SERIALIZE`, `V13-ORM-QUERY-CACHE-KEY` | Count, chunk-order and stream changes are not located |
| ORM query-cache index | (b) review | `V13-ORM-CACHE-PREFIX` |  |
| ORM Redis model hashes | (b) review | `V13-ORM-REDIS-HASHES` |  |
| ORM outbox keys on MySQL/MariaDB | (b) review | `V13-OUTBOX-MYSQL-KEYS` | The database backend is not located |
| ORM audit payloads on MySQL/MariaDB | (b) review | `V13-ORM-AUDIT-PAYLOADS` | Auditable models and audit-table setup; the database backend is not located |
| Turso migrations | (b) review | `V13-TURSO-ROLLBACK-DRIFT` |  |
| Schema table names on PostgreSQL | (b) review | `V13-SCHEMA-PG-TABLE-CASE` |  |
| Legacy `SecretString` ciphertext | (c) none | — | Older values become readable; no source change |
| `SecretString` client input | (b) review | `V13-SECRET-STRING-CLIENT-INPUT` |  |
| ORM protected values and `SecretString` serialization | (b) review | `V13-ORM-PROTECTED-VALUES` | Encrypted and masked model fields, and `SecretString` fields of serialized structs and models |
| Offline Redis mock | (b) review | `V13-REDIS-MOCK-TIE-ORDER` |  |
| SQLite DSN paths | (c) none | — | Only stray files of earlier versions |
| Auto-healing diagnostics | (b) review | `V13-AUTO-HEALING-DIAGNOSTICS` |  |
| ORM migrations | (c) none | — | Existing migrations keep their behaviour; `db:seed` refusing pending migrations is command behaviour |
| ORM transactional migrations and the pool | (c) none | — | `within_transaction()` is a v13 API, so v12 migrations are unaffected |
| ORM instance mutations | (b) review | `V13-ORM-MISSING-ROW` | Plain `delete()` calls are not located |
| ORM encryption key settings | (c) none | — | Only `.env` entries that the process environment lacks change; `.env` is not read |
| ORM constraint errors | (b) review | `V13-ORM-CONSTRAINT-ERRORS` | Code that matches `DatabaseError` |
| ORM transaction errors | (b) review | `V13-ORM-TRANSACTION-ERRORS` | `Orm::transaction` calls in files that mention `DatabaseError`; `cargo check` reports closure error types without `From<rullst_orm::Error>` |
| ORM query values | (c) none | — | Only exhaustive `match`es on `RullstValue` break, and `cargo check` reports them |
| ORM date/time columns | (b) review | `V13-TEXT-TIMESTAMPS` | `String` `created_at`/`updated_at` fields of SQLx models; whether the table's migration calls `timestamps()` is not checked |
| Schema builder constraints | (c) none | — | Only `Column` struct literals break, and `cargo check` reports them |
| ORM sandbox tests | (a) must-change | `V13-ORM-SANDBOX-TEST` | Flags every sandbox test as review; `cargo check` reports the signatures that no longer compile |
| ORM model SQLx options | (a) must-change | `V13-ORM-SQLX-STRUCT-OPTION` |  |
| ORM trashed scopes | (b) review | `V13-ORM-ONLY-TRASHED` |  |
| ORM bulk soft deletes | (b) review | `V13-ORM-TRASHED-DELETE-ALL` |  |
| ORM joined scopes | (b) review | `V13-ORM-SQL-TEXT` |  |
| ORM chunk bounds | (b) review | `V13-ORM-CHUNK-BOUNDS` |  |
| ORM eager pivot order | (c) none | — | The order was unspecified before |
| PersonalData reports | (b) review | `V13-PERSONAL-DATA-REPORT` |  |
| Nexus derive | (a) must-change | `V13-NEXUS-PRIMARY-KEY`, `V13-NEXUS-FIELD-KIND-NUMBER` | The field-kind rule is a review |
| Nexus composite keys | (c) none | — | Opt-in composite keys; only `RegistryEntry` struct literals break, and `cargo check` reports them |
| Nexus dates and times | (c) none | — | Derived field kinds change without a source edit; comparisons with `FieldKind::DateTime` are not located |
| Nexus access in the starters | (c) none | — | Generator output; alpha.2 `#[orm(skip)]` fields are reported under ORM database-managed columns |
| Nexus credentials | (b) review | `V13-NEXUS-DOTENV` |  |
| Nexus panel | (a) must-change | `V13-NEXUS-FIELD-KIND-MATCH` | Other panel changes are runtime behaviour without an application API |
| CLI migration catalog | (c) none | — | CLI |
| CLI `upgrade` downgrades | (c) none | — | CLI |
| CLI Foundry service environment | (b) review | `V13-FOUNDRY-SERVICE` |  |
| CLI Foundry build artifact | (b) review | `V13-FOUNDRY-SERVICE` |  |
| CLI generators | (b) review | `V13-MFA-CLIENT-SECRET` | Other changes affect new generator output only |
| CLI operations | (b) review | `V13-FOUNDRY-SERVICE` | Script exit statuses are not located |
| CLI diagnostics and side effects | (c) none | — | CLI |
| CLI home screen | (c) none | — | CLI |
| CLI security audit | (c) none | — | CLI |
| CLI dashboard audit | (c) none | — | CLI |
| Core deferrable jobs | (c) none | — | Opt-in APIs |
| Core scheduler time zones | (c) none | — | Opt-in `scheduler-tz` feature |
| Dash N+1 warning | (c) none | — | CLI |
| CLI security report | (c) none | — | CLI |
| Generated security tests | (c) none | — | Generator output for new projects |
| Blank JSON API starter | (c) none | — | Generator output; existing API projects keep their behaviour |
| CLI audit lockfile | (c) none | — | CLI |
| RASP shell signatures | (c) none | — | Request matching; not detectable in source |
| CLI footprint report | (c) none | — | New CLI command; replaced by `green` (CLI extras row) |
| CLI `dev`/`dash` shutdown | (c) none | — | CLI |
| CLI `dev --ts-sync` | (c) none | — | CLI |
| CLI `dash` live metrics | (c) none | — | CLI |
| Core development telemetry | (b) review | `V13-DEV-TELEMETRY-ROUTE` |  |
| Core development error console | (c) none | — | Development-only page; `handle_autofix` callers are not located |
| CLI `ai fix` and `ai review` | (c) none | — | New CLI commands |
| CLI project updates on SELinux | (c) none | — | CLI |
| CLI `pkg` in a virtual workspace | (c) none | — | CLI |
| CLI `new` repository | (c) none | — | CLI |
| CLI generated `validator` dependency | (c) none | — | Generator output; `cargo check` reports a missing `validator` dependency |
| CLI `make:model` imports | (c) none | — | Generator output for new models |
| CLI `dockerize` and `generate:buildah` | (c) none | — | Affected generated files are not distinguishable |
| CLI errors and exit codes | (c) none | — | CLI output and scripts |
| CLI `doctor` | (c) none | — | CLI |
| CLI output additions | (c) none | — | CLI |
| SaaS plan gates | (c) none | — | Opt-in generated module |
| Capital providers and NFS-e removed | (b) review | `V13-CAPITAL-REMOVED` | Compile-breaking; reported for review because a name such as `FiscalError` can also be an application type; also locates `BILLING_PROVIDER=lemonsqueezy` and `BILLING_STORE_ID` in `.env` and `.env.example` |
| Capital quota keys on MySQL/MariaDB | (b) review | `V13-CAPITAL-QUOTA-KEYS` | `SqlQuotaStore`/`SqlQuotaBackend`; the database backend is not located |
| Capital zero tier limit | (b) review | `V13-CAPITAL-ZERO-TIER` |  |
| Capital provider subscription IDs | (c) none | — | Rejects dot-only identifiers before a request |
| Capital invoice amounts | (c) none | — | Only amounts below one minor unit change |
| Messaging outbox relay key | (b) review | `V13-OUTBOX-RELAY-KEY` |  |
| Messaging encrypted SQLite startup | (c) none | — | Correct keyrings are unaffected |
| Messaging Redis Streams candidate | (c) none | — | No 12.x release contains the adapter |
| Mail providers removed | (b) review | `V13-MAIL-REMOVED` | Compile-breaking; also locates removed `MAIL_DRIVER` values and settings in Rust strings, `.env`, `.env.example` and `Rullst.toml` |
| Mail driver default | (b) review | `V13-MAIL-FACADE-CONFIG` |  |
| Mail sender | (b) review | `V13-MAIL-FACADE-CONFIG` |  |
| Mail attachment inspection | (b) review | `V13-MAIL-ATTACHMENT-INSPECTION` |  |
| Mail link checks | (c) none | — | Depends on message content |
| Mail facade settings | (b) review | `V13-MAIL-FACADE-CONFIG` |  |
| Mail queued attachments | (b) review | `V13-MAIL-QUEUED-ATTACHMENTS` | `Mail::init_queue`, `Mail::enqueue`/`enqueue_for_tenant` and `register_mail_handler` |
| Mail SES addresses | (c) none | — | Requests SES rejected now succeed |
| Mail Resend scheduling | (b) review | `V13-MAIL-RESEND-SCHEDULE` |  |
| Mail tracking recipient | (b) review | `V13-MAIL-TRACKING-RECIPIENT` |  |
| Mail plain-text fallback | (b) review | `V13-MAIL-TEXT-FALLBACK` |  |
| Labs runner | (a) must-change | `V13-LABS-RUNNER-REMOVED` |  |
| Education candidates | (c) none | — | New v13 packages, first published in 13.0.0-alpha.1 |
| Labs package | (c) none | — | New package in the release inventory |
| Media S3 storage | (c) none | — | New opt-in feature of a package first published in 13.0.0-alpha.1 |
| Generated billing settings | (b) review | `V13-BILLING-PROJECT-SETTINGS` |  |
| Application templates | (c) none | — | Generator output |
| Starter migrations | (b) review | `V13-SQLITE-ONLY-SEED-TIME` |  |
| ERP starter access | (b) review | `V13-ERP-DASHBOARD-ACCESS` |  |
| LMS lesson progress | (b) review | `V13-LMS-PROGRESS-KEY` |  |
| LMS concurrent progress saves | (b) review | `V13-LMS-RECORD-PROGRESS` |  |
| LMS lesson media in Nexus | (b) review | `V13-LMS-MEDIA-KINDS` |  |
| Blog robots and sitemap | (b) review | `V13-BLOG-ROBOTS-SITEMAP` |  |
| Generated login and registration | (b) review | `V13-CREDENTIAL-RATE-LIMIT` |  |
| SaaS user JSON | (b) review | `V13-PASSWORD-HASH-HIDDEN` |  |
| Registration timestamps | (b) review | `V13-EMPTY-TIMESTAMPS` |  |
| Billing row timestamps | (b) review | `V13-EMPTY-TIMESTAMPS` |  |
| Registration lengths | (b) review | `V13-UTF16-LENGTHS` |  |
| Revocable login sessions | (c) none | — | Additive APIs and generator output |
| Password change and reset | (c) none | — | Additive APIs and generator output |
| VPS deploy proxy | (b) review | `V13-VPS-DEPLOY-PROXY` |  |
| Blank database status | (b) review | `V13-MODEL-ALL` |  |
| Portfolio profile | (b) review | `V13-PORTFOLIO-PROFILE` |  |
| Blog and Portfolio pages | (b) review | `V13-PAGE-CSP-NONCE` |  |
| Generated linker configuration | (b) review | `V13-LINKER-CONFIG` |  |
| Build files from the 12.0 CLI | (b) review | `V13-MSVC-FASTLINK`, `V13-CARGO-LOCK-IGNORED`, `V13-DOCKER-UNLOCKED-BUILD` | Projects generated by the 12.0 CLI |
| ERP orders and stock | (b) review | `V13-ERP-STORE-ORDER` |  |
| Blog and ERP reads | (b) review | `V13-MODEL-ALL` |  |
| Blog and ERP page bounds | (b) review | `V13-PAGE-BOUNDS` |  |
| Docker build context | (b) review | `V13-DOCKER-CONTEXT` |  |
| Kubernetes Ingress | (b) review | `V13-K8S-INGRESS-TLS` |  |
| Starter health probes | (b) review | `V13-HEALTH-PROBES` |  |
| Kubernetes and Buildah names | (b) review | `V13-K8S-NAMES` |  |
| Pre-compressed static assets | (c) none | — | CLI |
| Static asset caching | (c) none | — | Runtime default; not detectable in source |
| Generated release profile | (c) none | — | Generator output for new projects |
| Generated container runtime | (c) none | — | Generator output for new projects |
| Interactive `new --api` | (c) none | — | CLI |
| CLI `new` wizard | (c) none | — | CLI |
| Minimal default features | (b) review | `V13-RULLST-DEFAULT-FEATURES`, `V13-ORM-DEFAULT-FEATURES` | Reports `rullst` and `rullst-orm` dependencies that keep their defaults and select no driver |
| Generated `rullst-orm` dependency | (c) none | — | Generator output; such a dependency is reported under Minimal default features |
| ORM adapter aggregate features | (a) must-change | `V13-POLYGLOT-FEATURE` | Dependency `features` and `[features]` entries such as `"rullst/orm-polyglot"` |
| Omni and offline sync removed | (a) must-change, (b) review | `V13-OFFLINE-SYNC-REMOVED`, `V13-OMNI-REMOVED` | The feature in dependency `features` and `[features]` entries; the shell's `omni-app/tauri.conf.json` or `omni-app/src/lib.rs`, and `cargo rullst omni`/`make:omni` in workflows, root scripts, Makefile and justfile |
| Starter page language | (b) review | `V13-RENDER-PAGE-LANGUAGE` |  |
| Repository examples | (c) none | — | Repository examples, not applications |
| Windows paths in CLI and console output | (c) none | — | CLI output |
| Consumer scaffolds under symlinked directories | (c) none | — | CLI |
| Core HSTS default | (c) none | — | Runtime default; not detectable in source |
| Artisan and `.env` | (c) none | — | `std::env::var` calls in migrations and seeders are not located |
| Core WAF in development | (c) none | — | Development-only behaviour |
| CSRF token on static-looking paths | (c) none | — | Only requests that previously received no token change |
| Multipart CSRF refusal | (c) none | — | Only the refusal text changes; tests that compare it are not located |
| Secure cookies over HTTPS | (c) none | — | Runtime behaviour; not detectable in source |
| CSRF cookie on cacheable responses | (c) none | — | `Cache-Control` values set by handlers are not located |
| Connect mock credentials | (c) none | — | Credentials come from configuration and are not located |
| Connect mock authorization URL | (c) none | — | Mock mode only |
| Account mail language and product name | (c) none | — | Generator output; existing projects keep their code |
| Production log defaults | (c) none | — | Runtime defaults; not detectable in source |
| Connect GitHub email | (b) review | `V13-CONNECT-GITHUB-EMAIL` | `GithubProvider` and `Connect::driver("github", ...)` |
| Account mail link lifetime | (c) none | — | Only reset lifetimes that previously failed change; the link wording is not located |
| Generated reset link lifetime | (c) none | — | Generator output; existing projects keep their code |
| CLI deploy targets removed | (b) review | `V13-DEPLOY-TARGETS-REMOVED` | `fly.toml`, `render.yaml`, `railway.json`, `railway.toml`, and `deploy --platform fly\|railway\|render` in workflows, root scripts, Makefile and justfile; introduced in 13.0.0-alpha.4 |
| CLI Foundry, Nix, Buildah and gRPC generators removed | (b) review | `V13-DEPLOY-GENERATORS-REMOVED` | `Foundry.toml`, a `nixify` `flake.nix`, `build_buildah.sh`, `src/grpc/mod.rs`, and the removed commands in workflows, root scripts, Makefile and justfile; introduced in 13.0.0-alpha.4 |
| CLI self-update removed | (b) review | `V13-CLI-SELF-UPDATE-REMOVED` | `rullst update check\|verify\|stage\|install`, or `update guided` with `--scope`/`--root`, in workflows, root scripts, Makefile and justfile; introduced in 13.0.0-alpha.4 |
| CLI extras (`ai`, `green`) | (b) review | `V13-CLI-EXTRAS` | `rullst ai` or `rullst footprint` in workflows, root scripts, Makefile and justfile; introduced in 13.0.0-alpha.4 |

## Planned simpler update experience

**Since 13.0.0-alpha.4** the CLI never downloads or replaces its own binary.
Install a newer CLI with Cargo and its exact version
(`cargo install cargo-rullst --version =VERSION --locked`). At most once a day,
in an interactive terminal, the CLI prints that command when crates.io has a
newer release of its major; see the
[CLI reference](../cli_reference.md#cargo-rullst-update-check-1210-working-source-unreleased).
The 12.1.0 to 13.0.0-alpha.3 self-updater (`update check`, `update verify`,
`update stage` and `update install`) was removed; the project commands below
remain.

`cargo rullst update project prepare --project PATH --json`
copies the Git working directory into private storage and edits only the
candidate's versioned workspace dependencies. It preserves dirty and untracked
source, tracked deletions and the root lockfile, including a legacy ignored
lockfile. Compare `before/` and `candidate/` at the reported location and review
`preparation.json`. Builds/tests are not executed and neither execution nor
application is authorized. See the [preparation limits and exclusions](../cli_reference.md#cargo-rullst-update-project-prepare-1210-working-source-unreleased).
Use `update project verify --prepared PATH --dry-run` to inspect its validation
commands. Only after reviewing the trusted project, use `--allow-project-code`
to authorize lockfile resolution, locked checks and tests in a fresh private
copy. This is not a sandbox; tests inherit your environment and can have
external effects. See the [verification options and limits](../cli_reference.md#cargo-rullst-update-project-verify-1210-working-source-unreleased).
Then `update project review --verified PATH --json` revalidates the verification
record and shows the full dependency diff with a review digest. Use the returned
`verified_directory`; the digest grants no apply authority. Explicit
`update project apply --verified PATH --approved-review SHA256` applies only
the reviewed manifests/root lockfile. The matching `recover` command restores
only that operation and refuses unrelated edits.

The [safe-update priority](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md#safe-update-experience)
proposes one guided flow for project preparation, validation and approved
application. `cargo rullst update guided --to VERSION --project PATH` composes
those commands. Each approval defaults to no and follows its complete review.
Version, directories and digests are carried between steps; separate prompts
govern trusted project execution/network and original-file application.
`--offline` keeps verification local. Since 13.0.0-alpha.4 the flow no longer
installs a CLI (`--scope` and `--root` were removed): install the target CLI
with Cargo first. See the [guided command](../cli_reference.md#cargo-rullst-update-guided-1210-working-source-unreleased).
File recovery does not replace database backups or application acceptance
tests, and updating the CLI alone never updates a deployed application.

The 12.1.0 update mechanism does not implement v13 migrations. The
v13 CLI (13.0.0-alpha.3 prerelease) carries its own rule catalog
([v12 to v13: source findings](#v12-to-v13-source-findings)); `update guided`
offers a project update only within the installed CLI's major.

## Is this unique?

No. Assisted upgrades are an established framework practice: Rails documents
interactive `bin/rails app:update`, Angular provides `ng update`, and Dart
offers preview/apply analysis fixes through `dart fix`. Microsoft's .NET
Upgrade Assistant also analyzed and changed projects, although Microsoft now
marks it deprecated in favor of its modernization tooling.

Rullst's useful distinction is the bounded composition: Cargo-workspace-aware
TOML edits, a version-selected framework rule catalog, human and JSON plans,
controlled snapshots, default rollback, explicit recovery and Cargo gates in
one CLI flow. This is a testable design choice, not evidence that Rullst is the
first or universally the best updater.

Official references:

- [Upgrading Ruby on Rails](https://guides.rubyonrails.org/upgrading_ruby_on_rails.html)
- [Angular `ng update`](https://angular.dev/cli/update)
- [Dart `dart fix`](https://dart.dev/tools/dart-fix)
- [.NET Upgrade Assistant overview](https://learn.microsoft.com/en-us/dotnet/core/porting/upgrade-assistant-overview)
