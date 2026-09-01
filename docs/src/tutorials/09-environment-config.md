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
    let database_is_configured = std::env::var_os("DATABASE_URL").is_some();

    println!(
        "Booting Rullst in {environment} mode; database configured: {database_is_configured}"
    );
    Ok(())
}
```

Do not log `DATABASE_URL`: it commonly embeds a username and password. Report
only whether configuration is present, and redact sensitive fields in structured
telemetry.

The generated server bootstrap loads `.env` before applying its runtime policy.
Standalone utilities must load a dotenv file themselves or receive exported
process variables. Environment precedence is exact: `RULLST_ENV`, legacy
`APP_ENV`, then `[app].env` in `Rullst.toml`. Unknown values are configuration
errors rather than silently becoming development.

---

## Step 3: Configure the Browser Security Baseline

`Server` applies the same public `apply_security_baseline` composition in
staging and production. CORS is deny-by-omission: list exact origins without a
trailing slash, and enable credentialed cross-origin requests only when the
application genuinely needs them.

```toml
[security]
csrf_same_site = "Strict"
cors_allow_origins = ["https://academy.example"]
cors_allow_credentials = false
csp = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'nonce-{NONCE}'; style-src 'self' 'nonce-{NONCE}'"
```

Wildcard, path-bearing, credential-bearing, queried or duplicate CORS origins
are configuration errors. When credentials are enabled, Core still grants them
only to an origin in the exact allowlist. Test the final policy behind the real
TLS proxy because an intermediary can change headers and cookie behavior.

---

## 💡 Key Takeaways
- Never hardcode credentials, database passwords, or JWT secrets in `.rs` source code.
- Add `.env` to `.gitignore` and commit `.env.example` to document required keys for team members.
- Prefer `RULLST_ENV`; keep `APP_ENV` only while migrating an existing project.
