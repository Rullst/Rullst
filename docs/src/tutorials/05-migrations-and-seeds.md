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
