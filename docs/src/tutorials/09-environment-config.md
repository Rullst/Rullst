# Tutorial 09: Environment Management & Configuration ⚙️

Rullst provides type-safe environment loading using `.env` files and `Config` models.

---

## 🛠️ Step 1: Define Environment Variables in `.env`

```dotenv
APP_ENV=development
APP_PORT=3000
DATABASE_URL=postgres://postgres:password@localhost:5432/my_app
JWT_SECRET=super_secret_jwt_key_change_in_prod
```

---

## 💻 Step 2: Access Configuration in Rust

```rust
use rullst_core::config::Config;

#[tokio::main]
async fn main() {
    // Rullst automatically loads .env files on boot
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");
        
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

    println!("🚀 Booting Rullst App in [{}] mode...", app_env);
}
```

---

## 💡 Key Takeaways
- Never hardcode credentials, database passwords, or JWT secrets in `.rs` source code.
- Add `.env` to `.gitignore` and commit `.env.example` to document required keys for team members.
