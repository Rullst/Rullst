# Tutorial 05: Database Migrations & Seeders 🗄️

Rullst manages database schemas using timestamped SQL migrations and Rust seeders.

---

## 🛠️ Step 1: Create a Migration

Generate a new empty migration:

```bash
cargo rullst make:migration create_products_table
```

Edit the generated SQL migration in `migrations/<timestamp>_create_products_table.sql`:

```sql
-- Up Migration
CREATE TABLE products (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    price NUMERIC(10, 2) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Down Migration
DROP TABLE products;
```

---

## 🚀 Step 2: Run and Rollback Migrations

```bash
# Run pending migrations
cargo rullst db:migrate

# Rollback last migration batch
cargo rullst db:rollback

# Check migration status
cargo rullst db:status
```

---

## 🌱 Step 3: Seed the Database

In `src/db/seeds.rs`:

```rust
use crate::models::User;

pub async fn seed() -> Result<(), Box<dyn std::error::Error>> {
    User::create(serde_json::json!({
        "name": "Admin User",
        "email": "admin@rullst.dev"
    })).await?;
    
    println!("🌱 Database seeded successfully!");
    Ok(())
}
```

Execute seeders via CLI:

```bash
cargo rullst db:seed
```

---

## 💡 Key Takeaways
- Migrations are versioned sequentially and run inside single database transactions.
- Use `cargo rullst db:seed` in development environments or CI test setups.
