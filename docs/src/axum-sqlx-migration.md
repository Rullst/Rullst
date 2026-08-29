# Axum and SQLx interoperability guide

Rullst builds on Axum, Tokio, Tower and SQLx and preserves direct access to
their APIs. This reduces coupling for HTTP and database code; it does not make
every optional framework subsystem free to remove.

## Mount an existing Axum router

`rullst::Router` supports conversion to and from `axum::Router`:

```rust
use axum::{Router as AxumRouter, routing::get};
use rullst::Router;

async fn existing_handler() -> &'static str {
    "existing Axum route"
}

let existing = AxumRouter::new().route("/existing", get(existing_handler));
let rullst_router: Router = existing.into();
let axum_router: AxumRouter = rullst_router.into();
# let _ = axum_router;
```

The standard Axum extractor and response types remain available, including
through Rullst's documented re-exports.

## Keep raw SQLx where it is useful

Application-owned SQL can live beside generated ORM queries. Bind every dynamic
value and review structural SQL separately:

```rust
use sqlx::PgPool;

async fn active_names(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT name FROM users WHERE active = $1")
        .bind(true)
        .fetch_all(pool)
        .await
}
```

## Generate an escape-hatch snapshot

```bash
cargo rullst eject
cargo check
```

The command writes an inspectable Axum/Tokio entry-point snapshot. Review it;
ORM models, migrations, authentication policy, Studio/Nexus integration and
other selected subsystems can still require deliberate migration work. Use
`--force` only when replacing `src/main.rs` is intended and the worktree is
backed up.

## Practical migration sequence

1. Keep domain types and handlers independent of framework globals.
2. Convert router boundaries incrementally.
3. Replace generated ORM calls with raw SQLx only where that trade-off helps.
4. Inventory authentication, middleware order, jobs, cache and admin surfaces.
5. Run application-specific integration and authorization tests before removing
   dependencies.

Interoperability is a maintained design goal. “Zero lock-in” or zero migration
cost is not a framework guarantee.
