# 36. Assisted framework upgrades

Rullst v12 introduces a bounded upgrade transaction for existing applications.
The goal is to make the safe, repeatable part a single command while refusing
to guess about application data or security policy.

> v12 is currently unreleased and **NO-GO for production**. The examples use
> `12.0.0-rc.1` as a placeholder for the planned first RC. Install or request
> that version only after it exists on crates.io.

## What the command can guarantee

`cargo rullst upgrade` can inventory the Cargo workspace, update the Rullst
release train, apply compiler suggestions, require `cargo check`, and restore
the files it controls after a failure. It cannot prove that a database upgrade,
authorization rule, provider integration or deployment still behaves correctly.

The automatic transaction owns only:

- versioned Rullst dependencies in exact Cargo workspace manifests;
- the root `Cargo.lock` produced by Cargo resolution;
- Rust edits proposed by `cargo fix`;
- a `cargo check --workspace --all-targets` gate using the application's
  selected features.

It never runs migrations, changes secrets, invents tenant/ownership policy,
opens Nexus or Studio, contacts application providers, or marks the result
production-ready.

## 1. Prepare the application

Create a branch, make the worktree reviewable, record the old test result, and
back up every database. Prove the database backup can be restored before
changing the framework.

Install the exact CLI from the same release train as the target framework:

```bash
cargo install cargo-rullst --version 12.0.0-rc.1 --locked --force
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

Use an explicit target only when intentionally evaluating another release in
the installed CLI's major train:

```bash
cargo rullst upgrade --to 12.0.0-rc.2 --dry-run
```

The same-major restriction prevents a v12 rules engine from pretending it
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
