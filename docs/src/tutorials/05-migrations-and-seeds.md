# Tutorial 05: Database Migrations & Seeders 🗄️

Rullst uses timestamped Rust migration modules for SQLx-primary projects. Turso
primary projects receive explicit, reversible `TursoMigration` statements
instead. This tutorial shows the SQLx path: `Schema` works with SQLite,
PostgreSQL and MySQL/MariaDB only, as [Database Support](../database-support.md)
lists.

---

## Step 1: Create a migration

Run this in the `shop` project of [Tutorial 02](02-cli-generators.md), which
already has `products` and `categories` tables:

```bash
cargo rullst make:migration create_catalog_items_table
```

The command creates `src/migrations/m<timestamp>_create_catalog_items_table.rs`
and regenerates `src/migrations/mod.rs`. Edit the generated `up` and `down`
methods:

```rust,no_run
use rullst_orm::{async_trait, schema::{ForeignKeyAction, Migration, Schema}};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260901000000_create_catalog_items_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        Schema::create("catalog_items", |table| {
            table.id();
            table.string("name").not_null();
            // MySQL/MariaDB cannot index a TEXT column, so the indexed SKU
            // has a bounded type.
            table.string("sku").not_null().unique().col_type = "VARCHAR(64)".to_string();
            table
                .foreign_id("category_id", "categories")
                .on_delete(ForeignKeyAction::Restrict);
            table.integer("price_cents").not_null();
            table.check("price_cents >= 0");
            table.date("available_on");
            table.index(&["category_id", "available_on"]);
            table.timestamps();
        })
        .await
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Schema::drop_if_exists("catalog_items").await
    }
}
```

Keep the generated timestamp/name in `name()`; the runner uses that stable value
to record migration state. The foreign key needs the referenced table:
`categories` comes from `cargo rullst make:resource Category`, and this
migration, generated later, has a later timestamp and runs after it.

`unique()`, `index(...)`, `foreign_id(...)`/`foreign(...)` and `check(...)`
declare constraints and indexes with the table (see the
[ORM guide](../crates/orm.md#constraints-indexes-and-foreign-keys) for every
option and the generated names). PostgreSQL and SQLite create the indexes with
`CREATE INDEX IF NOT EXISTS` right after the table; MySQL/MariaDB declare them
inside `CREATE TABLE`.

`table.timestamps()` creates nullable `created_at`/`updated_at` UTC-instant
columns that default to the current time: `TIMESTAMPTZ` on PostgreSQL,
`DATETIME(6)` on MySQL/MariaDB and RFC 3339 text on SQLite. Read them as
`rullst::orm::chrono::DateTime<Utc>` model fields. `date()`, `time()`,
`timestamp()` and `timestamp_tz()` add single date/time columns for
`NaiveDate`, `NaiveTime`, `NaiveDateTime` and `DateTime<Utc>` fields. They need
a strict driver feature (`strict-postgres`, `strict-mysql` or `strict-sqlite`),
which `cargo rullst new` selects: the multi-driver SQLx `Any` pool cannot read
date/time values, so with it `timestamps()` keeps `TEXT` columns for `String`
fields and the other date/time helpers fail on PostgreSQL and MySQL/MariaDB.

To change an existing table, use `Schema::table`:

```rust,no_run
use rullst_orm::schema::{ForeignKeyAction, Schema};

# async fn up() -> Result<(), rullst_orm::Error> {
Schema::table("products", |table| {
    table.integer("category_id");
    table
        .foreign("category_id")
        .references("categories", "id")
        .on_delete(ForeignKeyAction::SetNull);
    table.unique_index(&["name", "category_id"]);
})
.await
# }
```

SQLite cannot add constraints to an existing table: a foreign key works there
only on a nullable column added in the same call, and a CHECK, a foreign key on
an existing column or a `NOT NULL` column without a default return
`Error::UnsupportedSchemaChange` before anything runs.

`table.float(...)` maps to an `f64` model field: it emits `DOUBLE PRECISION` on
PostgreSQL, `DOUBLE` on MySQL/MariaDB and `REAL` on SQLite. PostgreSQL columns
created by earlier Rullst versions were single-precision `REAL`; migrate them
explicitly (`ALTER TABLE products ALTER COLUMN price TYPE DOUBLE PRECISION`)
before relying on `f64` precision.

`table.boolean(...)` maps to a `bool` model field. PostgreSQL receives a native
`BOOLEAN` (an integer default of `0`/`1` becomes `FALSE`/`TRUE`); MySQL/MariaDB
and SQLite keep an `INTEGER` 0/1 flag. PostgreSQL columns created by earlier
versions were `INTEGER`; migrate them with
`ALTER TABLE t ALTER COLUMN c TYPE BOOLEAN USING c <> 0`.

Text defaults (`ColumnDefault::Text`) and `table.enum_col(...)` variants are
embedded in the DDL as single-quoted literals with doubled single quotes.
`Schema::create` rejects such text when it contains a backslash or a control
character, because MySQL/MariaDB treat a backslash inside a quoted literal as an
escape character by default.

---

## Step 2: Run, inspect, and roll back migrations

```bash
cargo rullst db:migrate
cargo rullst db:status
cargo rullst db:rollback
```

`db:rollback` runs `down()` for the last recorded batch, in reverse order. The
SQLx migration runner does **not** wrap the whole batch in one database
transaction. By default it runs each `up()`/`down()` and then records it in the
`migrations` table as separate statements, so a crash or lost connection in
between leaves a change applied but unrecorded. A migration that returns `true`
from `within_transaction()` (v13 prerelease API) runs `up()`/`down()` and its
record inside one `Orm::transaction` instead. The template written by the ORM's
own artisan `make:migration` command opts in; add the method to migrations
generated by `cargo rullst make:migration` yourself:

```rust,ignore
fn within_transaction(&self) -> bool {
    true
}
```

Only statements that join that transaction are covered: `Schema` calls,
generated model methods, `rullst_orm::execute_query!` and raw SQL sent with
`Orm::raw(sql).execute()` (or `.map_to()`). `Orm::raw` is the raw-SQL path that
stays in the transaction:

```rust,ignore
async fn up(&self) -> Result<(), rullst_orm::Error> {
    Schema::create("tags", |table| {
        table.id();
        table.string("name").not_null();
    })
    .await?;
    // Runs in the migration's transaction, so it sees the new table and rolls
    // back with it.
    Orm::raw("CREATE UNIQUE INDEX tags_name_key ON tags (name)")
        .execute()
        .await?;
    Ok(())
}
```

`Orm::pool()` and `Orm::read_pool()` would hand out another connection that
cannot see the transaction's uncommitted tables and does not roll back with
it. Since 13.0.0-alpha.3 they return `Error::Validation`, naming the migration,
while such a migration's `up()`/`down()` runs, so the migration fails and rolls
back unrecorded instead of half-applying (`Orm::begin_transaction()` fails the
same way). Keep the default `false` for a migration that sends queries such as
`sqlx::query(sql).execute(Orm::pool()?)`. A pool obtained before the migration
ran, another crate's pool or a task spawned from `up()` is not detected and
still runs outside the transaction. PostgreSQL and SQLite roll the DDL back with the record; MySQL and
MariaDB commit each DDL statement implicitly, so there only data changes after
the last DDL statement are atomic with the record. Statements that refuse to
run in a transaction block, such as PostgreSQL `CREATE INDEX CONCURRENTLY`,
also need `false`. Make every migration reversible and test both directions
against each supported database.

These commands use the same database as the running server: the process
`DATABASE_URL`, then `DATABASE_URL` in `./.env` (never overriding the process
environment), then `[database].url` in `Rullst.toml`. If none is set, the
command fails instead of creating a new SQLite file, and a database that cannot
be initialized also fails the command with exit status 1.

Each migration's tracking row is removed as soon as its `down()` succeeds. If a
later `down()` fails, the rollback stops with that error: the migrations it
already reverted are no longer recorded as applied, while the failed migration
and the rest of the batch stay recorded. Fix the cause and run `db:rollback`
again to continue from that point.

`db:migrate` and `db:rollback` hold a runner lock for the whole run on
PostgreSQL (`pg_advisory_lock`) and MySQL/MariaDB (`GET_LOCK`), on a dedicated
connection outside the pool. Replicas that start together therefore apply each
pending migration once: a second runner waits, then finds it already recorded.
The lock is released when the run ends or its process exits. A newly created
`migrations` table also declares the migration name `UNIQUE`. SQLite has no
cross-process advisory lock, so run a single migrator per SQLite database.

---

## Step 3: Define and register a seeder

Create `src/seeds.rs` with a seeder for the `Product` model of
[Tutorial 03](03-active-record-crud.md), and declare `mod seeds;` in
`src/main.rs`:

```rust,ignore
use rullst::db::{async_trait, RowSnapshot, Seeder};
use crate::models::product::Product;

pub struct CatalogSeeder;

#[async_trait]
impl Seeder for CatalogSeeder {
    async fn run(&self) -> Result<(), rullst::orm::Error> {
        // Idempotent: a second `db:seed` finds the row and adds nothing.
        let existing = Product::query()
            .where_eq("name", "Starter kit")
            .first()
            .await?;
        if existing.is_some() {
            return Ok(());
        }
        let mut product = Product {
            id: 0,
            name: "Starter kit".to_string(),
            price_cents: 2_500,
            snapshot: RowSnapshot::default(),
        };
        product.save().await
    }
}

pub fn get_seeders() -> Vec<Box<dyn Seeder>> {
    vec![Box::new(CatalogSeeder)]
}
```

Register migrations and seeders before starting the server: in `main()`,
replace the starter's `rullst::artisan!(crate::migrations::get_migrations());`
with

```rust,ignore
rullst::artisan!(
    crate::migrations::get_migrations(),
    crate::seeds::get_seeders()
);
```

Without `artisan!`, `Server::run` still recognizes `db:migrate`, `db:rollback`,
`db:status` and `db:seed`, but it has no registry to run them against: the
process exits with status 1 and asks for `rullst::artisan!` rather than
reporting "Nothing to migrate." as a success.

Then run:

```bash
cargo rullst db:migrate
cargo rullst db:seed
```

`db:seed` first checks the registered migrations against the `migrations`
table. While one is pending it runs no seeder and fails, instead of letting a
seeder fail on a table that does not exist yet:

```text
Validation error: 1 migration(s) pending, starting with `m20261012_create_users`: run `db:migrate` before `db:seed`
```

Seeders execute sequentially. Make development/CI seeders idempotent, as above,
if the command may run more than once. Never commit real passwords or provider
secrets; for authentication records, hash a test password with `rullst-auth` or
create a non-login fixture.

---

## Key takeaways

- Migrations are Rust modules, not split `up/down` SQL files.
- Migration names and order are generated deterministically from timestamps.
- Batch tracking exists, but batch-wide transactional rollback is not implied.
- `db:seed` executes only seeders explicitly registered with `artisan!`, and
  only after every registered migration is applied.
