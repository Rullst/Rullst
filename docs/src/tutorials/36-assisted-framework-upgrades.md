# 36. Assisted framework upgrades

Rullst v12 introduces a bounded upgrade transaction for existing applications.
The goal is to make the safe, repeatable part a single command while refusing
to guess about application data or security policy.

> This guide targets `12.1.0`; check [publication status](../v12.md). Install
> the exact release and complete the application-specific validation below
> before any production rollout.

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

The plan shows every dependency edit and source finding. `BLOCKER` means a
known old API requires attention; `REVIEW` means the CLI found a boundary that
must be revalidated. Neither label means that unreported code is automatically
safe.

For automation, request JSON:

```bash
cargo rullst upgrade --dry-run --json > upgrade-plan.json
```

The root object uses `schema_version: "rullst.upgrade-plan.v1"` and identifies
the rule catalog, exact target, manifest changes, detected source majors,
findings, automatic scope and mandatory manual gates. Consumers must reject an
unknown schema version rather than silently interpreting it as v1.

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

## Future upgrades such as v12 to v13

The engine and migration knowledge are intentionally separated. Each new major
must ship a new CLI from that major, extend the versioned rule catalog, document
the supported source baselines, and add process-level fixtures for dry-run,
machine-readable output, successful application and rollback. A v13 CLI can
therefore reuse the transaction while owning v13-specific rules; a v12 CLI is
not allowed to guess them.

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
