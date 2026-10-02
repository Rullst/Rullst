# 36. Assisted framework upgrades

Rullst v12 introduces a bounded upgrade transaction for existing applications.
The goal is to make the safe, repeatable part a single command while refusing
to guess about application data or security policy.

> The transaction steps below were written for `12.1.0`; check [publication status](../v12.md).
> The v13 source rules, the assisted `cargo rullst ai upgrade` session and the
> rule classification are described in
> [v12 to v13: source findings](#v12-to-v13-source-findings) and belong to the
> unpublished `13.0.0-alpha.1` source. Install the exact release and complete
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

Install the exact CLI from the same release train as the target framework:

```bash
cargo install cargo-rullst --version 12.1.0 --locked --force
```

The framework command does not update its own executable. This matters for v5:
the already-published v5 CLI cannot gain the new v12 migration engine
retroactively. Install the v12 CLI first, then run the command inside the
application.

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

### Recovery boundaries and unreleased hardening

Stop editors, watchers and other writers before restoring. File recovery does
not undo build-script/test side effects or database/external-service changes.
Keep an independent version-control backup; the directory under `target` is not
a substitute for one. A filesystem error during application can leave some
files restored and others unchanged, so always review the result.

The working 12.1.0 implementation now preflights the entire backup, stages every
replacement before writing originals, and rejects linked or malformed paths.
It limits indexes to 8 MiB/100,000 entries, each snapshot to 64 MiB and the total
to 512 MiB. Disk failure while staging leaves originals intact; failure during
the later per-file apply reports progress and retains the backup. This is not
an all-files atomic commit and does not defend against hostile concurrent
filesystem changes. These improvements are **not in published 12.0.0** and still
require the 12.1.0 cross-platform release checks.

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
YAML, `omni-app/src/lib.rs`, `rullst-client.ts`, `.env`/`.env.example` keys and
release scripts). Files are read without following symbolic links; a finding
records a rule and a line, never file contents. A Rust file that does not
parse, is not UTF-8 or is larger than 2 MiB is listed as `NOT SCANNED`.

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
`update project verify`, `review` and `apply` refuse a preparation that has
must-change findings and carry review findings in the plan.

### JSON plan fields

`--dry-run --json` keeps every `rullst.upgrade-plan.v1` key. Each element of
`source_findings` keeps `path`, `line`, `code` (the stable rule id), `severity`
(`BLOCKER` or `REVIEW`) and `message`, and adds:

| Key | Meaning |
| :--- | :--- |
| `kind` | `must-change` or `review` |
| `migration_row` | First-column title of the row in `migration-v13.md` |
| `migration_url` | The published table of the v13 migration guide |

The root object adds `finding_counts` (`must_change`, `review`),
`unscanned_sources` (project-relative paths) and `migration_guide`.

## Assisted fixes with `cargo rullst ai upgrade`

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
`cargo rullst upgrade` stays deterministic and keeps its published command and
`UpgradeOptions` shape. A one-word chat goal of `upgrade` now opens this
command; phrase such a goal with more words.

With no provider connected, the deterministic offline assistant demonstrates
the flow on `V13-RENDER-PAGE-LANGUAGE`: it rewrites the first flagged
`render_page(&htmx, title, body)` call as
`rullst::htmx::render_page_with_lang(&htmx, "en", title, body)` and proposes
`cargo check`. It explains the other findings without proposing edits.

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

`.github/rehearse-v12-upgrade.sh` is a manual helper (not a CI job). It
generates starters with a v12 CLI built from `origin/v12`, points their Rullst
dependencies at a v13 checkout through `[patch.crates-io]` (the registry
requirements stay, so the upgrade edits them as it would for a published
release), records the dry-run plans, applies the upgrade with
`--keep-on-failure` and runs each project's tests:

```bash
git worktree add ../rullst-v12 origin/v12
cargo build --locked -p cargo-rullst --manifest-path ../rullst-v12/Cargo.toml
cargo build --locked -p cargo-rullst
CARGO_TARGET_DIR=/tmp/rehearsal-target .github/rehearse-v12-upgrade.sh \
  ../rullst-v12/target/debug/cargo-rullst target/debug/cargo-rullst . /tmp/rehearsal blank blog
```

The October 2026 rehearsal (v12.1.2 CLI, SQLite, `--default`) produced:

| Starter | Must-change | Review | Upgrade and `cargo check` | Project tests |
| :--- | :--- | :--- | :--- | :--- |
| Blank | 0 | 5 (`V13-GITIGNORE-DATABASES`, `V13-ORM-DEFAULT-FEATURES`, `V13-MODEL-ALL`, `V13-RENDER-PAGE-LANGUAGE`, `V13-HEALTH-PROBES`) | passed without source edits | passed (the starter has no tests) |
| Blog | 0 | 8 (the Blank items except page language, plus `V13-BLOG-ROBOTS-SITEMAP`, `V13-NEXUS-DOTENV`, `V13-PAGE-CSP-NONCE`, `V13-SQLITE-ONLY-SEED-TIME`) | passed without source edits | passed (the starter has no tests) |

After `db:migrate`, the upgraded Blog served `/`, a seeded post and the
loopback Nexus panel with `200` and an unknown post with `404`; `/health`
answered `404` in both starters, as `V13-HEALTH-PROBES` reports.

The same script with a v12.0.0 CLI (built from a worktree of the `v12.0.0`
tag) gave the same results: both upgrades passed `cargo check` without source
edits, the tests passed and the findings matched the table. The 12.0 starters
also carry the build files listed in
[Projects generated by the 12.0 CLI](../migration-v13.md#projects-generated-by-the-120-cli).
A later dry run with the current catalog therefore adds `V13-MSVC-FASTLINK`
and `V13-CARGO-LOCK-IGNORED` (7 review findings for Blank, 10 for Blog), plus
`V13-DOCKER-UNLOCKED-BUILD` for a starter generated with `--docker`.

Manual steps: before publication, the `[patch.crates-io]` entries pointing at
the local v13 crates (remove them once v13 is on crates.io); the review
findings above, which the generated v13 starters resolve and existing projects
adopt by hand; and the application-owned gates below. In the rehearsal
`cargo rullst ai upgrade` with the offline assistant applied the page-language
fix to the upgraded Blank starter after a checkpoint, and the proposed
`cargo check` passed against the v13 crates.

## v12 → v13 rule classification

Every row of the [v13 migration guide](../migration-v13.md#changes-from-the-published-1210-source)
is classified for `rullst-upgrade-rules-v4`:

- **(a) must-change**: a compile-breaking or API-shape change detectable in
  application source (8 rows);
- **(b) review**: changed behaviour worth reviewing when the application uses
  the affected API, feature or configuration, which the rules locate (91 rows);
- **(c) none**: no application impact, or not detectable in the application
  (CLI behaviour, generator output for new projects, opt-in features, fixes
  of inputs that previously failed) (44 rows).

The catalog has 98 rules. A row can map to several rules and a rule to
several rows; when a row changes in a later release, update this table and
the rule together.

| Migration row | Class | Rules | Notes |
| :--- | :--- | :--- | :--- |
| Existing runtime APIs | (c) none | — | General statement; the session-schema migration is the next row |
| Account/session registry | (b) review | `V13-AUTH-RECOVERY-MIGRATE` |  |
| Private object storage | (c) none | — | Opt-in `storage-s3` candidate |
| R2 public URLs | (b) review | `V13-R2-PUBLIC-URL` |  |
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
| Core request validation | (b) review | `V13-VALIDATION-STATUS` |  |
| Core feature flags and scheduler | (b) review | `V13-SCHEDULER-WEEKDAYS` | A/B splits: the two feature-flag rules below; the bucket reassignment depends on flag configuration and is not located |
| Memory feature-flag splits | (b) review | `V13-MEMORY-FEATURE-SPLITS` |  |
| Database feature-flag splits | (b) review | `V13-DB-FEATURE-SPLITS` |  |
| Security headers | (b) review | `V13-REFERRER-NO-REFERRER` |  |
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
| Android release command | (b) review | `V13-ANDROID-RELEASE-SIGNING` | Detected in workflows, root scripts, Makefile and justfile |
| Age assurance | (c) none | — | Opt-in APIs and generator |
| Optional consent and export | (c) none | — | Opt-in generator |
| Project context | (c) none | — | CLI generator |
| Typed API generation | (c) none | — | Opt-in CLI profile |
| CLI generator output | (b) review | `V13-TS-CLIENT-REGENERATE` | Other generator changes affect new output only |
| CLI generator robustness | (b) review | `V13-TS-CLIENT-REGENERATE`, `V13-GITIGNORE-DATABASES` | Other generator changes affect new output only |
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
| ORM migrations | (c) none | — | Existing migrations keep their behaviour |
| ORM instance mutations | (b) review | `V13-ORM-MISSING-ROW` | Plain `delete()` calls are not located |
| ORM sandbox tests | (a) must-change | `V13-ORM-SANDBOX-TEST` | Flags every sandbox test as review; `cargo check` reports the signatures that no longer compile |
| ORM model SQLx options | (a) must-change | `V13-ORM-SQLX-STRUCT-OPTION` |  |
| ORM trashed scopes | (b) review | `V13-ORM-ONLY-TRASHED` |  |
| ORM bulk soft deletes | (b) review | `V13-ORM-TRASHED-DELETE-ALL` |  |
| ORM joined scopes | (b) review | `V13-ORM-SQL-TEXT` |  |
| ORM chunk bounds | (b) review | `V13-ORM-CHUNK-BOUNDS` |  |
| ORM eager pivot order | (c) none | — | The order was unspecified before |
| PersonalData reports | (b) review | `V13-PERSONAL-DATA-REPORT` |  |
| Nexus derive | (a) must-change | `V13-NEXUS-PRIMARY-KEY`, `V13-NEXUS-FIELD-KIND-NUMBER` | The field-kind rule is a review |
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
| CLI `dev`/`dash` shutdown | (c) none | — | CLI |
| CLI `dev --ts-sync` | (c) none | — | CLI |
| CLI `dash` live metrics | (c) none | — | CLI |
| Core development telemetry | (b) review | `V13-DEV-TELEMETRY-ROUTE` |  |
| CLI project updates on SELinux | (c) none | — | CLI |
| CLI `pkg` in a virtual workspace | (c) none | — | CLI |
| CLI `dockerize` and `generate:buildah` | (c) none | — | Affected generated files are not distinguishable |
| CLI errors and exit codes | (c) none | — | CLI output and scripts |
| CLI `doctor` | (c) none | — | CLI |
| CLI output additions | (c) none | — | CLI |
| SaaS plan gates | (c) none | — | Opt-in generated module |
| Capital provider webhooks | (b) review | `V13-CAPITAL-WEBHOOKS` |  |
| Capital quota keys on MySQL/MariaDB | (b) review | `V13-CAPITAL-QUOTA-KEYS` | `SqlQuotaStore`/`SqlQuotaBackend`; the database backend is not located |
| Capital zero tier limit | (b) review | `V13-CAPITAL-ZERO-TIER` |  |
| Capital provider subscription IDs | (c) none | — | Rejects dot-only identifiers before a request |
| Messaging outbox relay key | (b) review | `V13-OUTBOX-RELAY-KEY` |  |
| Messaging encrypted SQLite startup | (c) none | — | Correct keyrings are unaffected |
| Messaging Redis Streams candidate | (c) none | — | No 12.x release contains the adapter |
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
| Education candidates | (c) none | — | Unpublished v13 candidates |
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
| Interactive `new --api` | (c) none | — | CLI |
| CLI `new` wizard | (c) none | — | CLI |
| Generated `rullst-orm` dependency | (b) review | `V13-ORM-DEFAULT-FEATURES` |  |
| Omni desktop runner | (b) review | `V13-OMNI-RUNNER` |  |
| Omni managed backend | (b) review | `V13-OMNI-BACKEND` |  |
| Starter page language | (b) review | `V13-RENDER-PAGE-LANGUAGE` |  |
| Repository examples | (c) none | — | Repository examples, not applications |

## Planned simpler update experience

**Working 12.1.0 source only, not published 12.0.0:** advisory discovery is now
available through these commands:

```bash
cargo rullst update check
cargo rullst update check --to 12.1.0 --json
cargo rullst update check --offline
cargo rullst update check --refresh
cargo rullst update check --no-cache
```

The default selects an eligible stable CLI in the installed major. An exact
newer major needs `--allow-major`, and an exact prerelease also needs
`--prerelease`. Neither flag authorizes migration or provides future-major
rules. Unknown/yanked versions and downgrades are rejected. The report includes
the exact version, declared Rust minimum, release-notes link and current
OS/architecture; it does not certify compatibility. JSON uses
`rullst.update-discovery.v1`; reject unknown schemas. Metadata checksums are
not proof of publisher identity, and every authority field remains false.
No CLI/project files change. Linux/macOS explicit checks reuse a private,
owner/permission-checked catalog for six hours. `--offline` and
`CARGO_NET_OFFLINE=true` read only a fresh cache and revalidate its metadata;
missing, expired or invalid caches fail without network access or writes.
`--refresh` forces online discovery and `--no-cache` disables persistence;
neither overrides the offline environment setting. JSON includes source and
age, never installation authority. The old shared cache is not trusted.
Windows has a protected owner/DACL cache implementation, with native cache
acceptance recorded in the [maintenance checkpoint](../v12.md#1210-delivery-checkpoint-unreleased).
Online discovery can recover from an unavailable cache. See the
[CLI reference](../cli_reference.md#cargo-rullst-update-check-1210-working-source-unreleased)
for locations and boundaries.

Working-source `cargo rullst update verify --to VERSION --directory PATH`
also authenticates an already-downloaded native manifest through the installed
GitHub CLI, then checks the matching executables' sizes and hashes. Its
[separate verification contract](../cli_reference.md#cargo-rullst-update-verify-1210-working-source-unreleased)
does not install or execute those files, replace project files, or establish
current registry eligibility. Native release assets are still being prepared;
this command is not proof that 12.1.0 artifacts have been published.

Working-source `cargo rullst update project prepare --project PATH --json`
now copies the Git working directory into private storage and edits only the
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

The [safe-update priority](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md#safe-update-experience) proposes
one guided flow for CLI installation, project preparation, validation and
approved application. The working-source **12.1.0** CLI now composes those
commands through `cargo rullst update guided --to 12.1.0 --scope both --root
ABSOLUTE_PRIVATE_DIRECTORY --project PATH`. Each approval defaults to no and
follows its complete review. Version, directories and digests are carried
between steps; separate prompts govern download, CLI installation, trusted
project execution/network and original-file application. `--scope project
--offline` uses local project preparation/verification without CLI downloads.
See the [guided command](../cli_reference.md#cargo-rullst-update-guided-1210-working-source-unreleased).
Native/fault and complete user-journey acceptance remain release gates; these
unpublished changes do not imply published 12.1.0 assets. File recovery
does not replace database backups or application acceptance tests, and updating
the CLI alone never updates a deployed application.

Preparing the update mechanism in 12.1.0 does not implement unknown v13
migrations. The future v13 CLI must still ship its own versioned rules and
application acceptance fixtures before that major upgrade can be offered.

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
