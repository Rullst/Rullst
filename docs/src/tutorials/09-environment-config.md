# Tutorial 09: Environment Management & Configuration ⚙️

Rullst provides a validated runtime environment shared by its subsystems. New
projects use `RULLST_ENV`; `APP_ENV` remains a legacy compatibility alias.

---

## 🛠️ Step 1: Define Environment Variables in `.env`

```dotenv
RULLST_ENV=development
PORT=3000
APP_KEY=REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY
DATABASE_URL=postgres://postgres:password@localhost:5432/my_app
```

---

## 💻 Step 2: Access Configuration in Rust

```rust,no_run
use rullst_core::config::RullstConfig;

fn report_environment() -> Result<(), Box<dyn std::error::Error>> {
    let environment = RullstConfig::global().environment()?;
    let database_url = std::env::var("DATABASE_URL")?;

    println!("Booting Rullst in {environment} mode with {database_url}");
    Ok(())
}
```

The generated server bootstrap loads `.env` before applying its runtime policy.
Standalone utilities must load a dotenv file themselves or receive exported
process variables. Environment precedence is exact: `RULLST_ENV`, legacy
`APP_ENV`, then `[app].env` in `Rullst.toml`. Unknown values are configuration
errors rather than silently becoming development.

---

## 💡 Key Takeaways
- Never hardcode credentials, database passwords, or JWT secrets in `.rs` source code.
- Add `.env` to `.gitignore` and commit `.env.example` to document required keys for team members.
- Prefer `RULLST_ENV`; keep `APP_ENV` only while migrating an existing project.
