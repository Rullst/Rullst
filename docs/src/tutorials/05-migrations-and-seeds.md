# Tutorial 05: Database Migrations & Seeders 🗄️

Rullst uses timestamped Rust migration modules for SQLx-primary projects. Turso
primary projects receive explicit, reversible `TursoMigration` statements
instead. This tutorial shows the SQLx path.

---

## Step 1: Create a migration

```bash
cargo rullst make:migration create_products_table
```

The command creates `src/migrations/m<timestamp>_create_products_table.rs` and
regenerates `src/migrations/mod.rs`. Edit the generated `up` and `down` methods:

```rust,no_run
use rullst_orm::{async_trait, schema::{Migration, Schema}};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260901000000_create_products_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        Schema::create("products", |table| {
            table.id();
            table.string("name").not_null();
            table.integer("price_cents").not_null();
            table.timestamps();
        })
        .await
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Schema::drop_if_exists("products").await
    }
}
```

Keep the generated timestamp/name in `name()`; the runner uses that stable value
to record migration state.

`table.timestamps()` creates nullable `created_at`/`updated_at` text columns
that default to the current timestamp. On MySQL/MariaDB the builder emits the
expression form `DEFAULT (CURRENT_TIMESTAMP)`, which those servers require for
text columns (MySQL 8.0.13+, MariaDB 10.2.1+); SQLite and PostgreSQL use the
plain `DEFAULT CURRENT_TIMESTAMP`.

`table.float(...)` maps to an `f64` model field: it emits `DOUBLE PRECISION` on
PostgreSQL, `DOUBLE` on MySQL/MariaDB and `REAL` on SQLite. PostgreSQL columns
created by earlier Rullst versions were single-precision `REAL`; migrate them
explicitly (`ALTER TABLE products ALTER COLUMN price TYPE DOUBLE PRECISION`)
before relying on `f64` precision.

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
current SQLx migration runner does **not** wrap the whole batch automatically in
one database transaction. Make every migration reversible, test both directions
against each supported database, and use backend-appropriate transactional DDL
inside the migration when atomicity is required.

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

---

## Step 3: Define and register a seeder

```rust,ignore
use rullst_orm::{async_trait, Seeder};

pub struct AdminSeeder;

#[async_trait]
impl Seeder for AdminSeeder {
    async fn run(&self) -> Result<(), rullst_orm::Error> {
        let mut admin = crate::models::User {
            id: 0,
            name: "Admin User".to_string(),
            email: "admin@example.test".to_string(),
        };
        admin.save().await
    }
}

pub fn get_seeders() -> Vec<Box<dyn Seeder>> {
    vec![Box::new(AdminSeeder)]
}
```

Register migrations and seeders before starting the server:

```rust,ignore
rullst::artisan!(
    crate::migrations::get_migrations(),
    crate::seeds::get_seeders(),
);
```

Without `artisan!`, `Server::run` still recognizes `db:migrate`, `db:rollback`,
`db:status` and `db:seed`, but it has no registry to run them against: the
process exits with status 1 and asks for `rullst::artisan!` rather than
reporting "Nothing to migrate." as a success.

Then run:

```bash
cargo rullst db:seed
```

Seeders execute sequentially. Make development/CI seeders idempotent if the
command may run more than once. Never commit real passwords or provider secrets;
for authentication records, hash a test password with `rullst-auth` or create a
non-login fixture.

---

## Key takeaways

- Migrations are Rust modules, not split `up/down` SQL files.
- Migration names and order are generated deterministically from timestamps.
- Batch tracking exists, but batch-wide transactional rollback is not implied.
- `db:seed` executes only seeders explicitly registered with `artisan!`.
