# Tutorial 11: Authentication Scaffolding 🔒

The authentication generator creates a reviewable starting point for
cookie-session login and registration. It is application source code, not a
hosted identity service or a claim that every production policy is automatic.

---

## Step 1: Run the generator

Run this from a Rullst project root:

```bash
cargo rullst auth
```

The current command creates:

- `src/models/user.rs`;
- `src/migrations/m<timestamp>_create_users_table.rs` and the migrations module;
- `src/controllers/auth_controller.rs`;
- `src/middlewares/auth_middleware.rs`;
- `src/pages/auth.rs`;
- the corresponding module declarations, including `controllers`,
  `middlewares`, `models` and `pages` in `src/lib.rs` (or `src/main.rs`); and
- a best-effort refresh of the generated `.llms.txt`,
  `.rullst/context-map.json` and `diagram.md` (a hand-written `diagram.md` is
  left unchanged).

It also enables the `orm` and `auth` umbrella features in `Cargo.toml`. It
fails before writing anything when one of those files exists or when a
`*_create_users.rs`/`*_create_users_table.rs` migration already creates the
users table (the blank database starter ships one); add the account columns
with `cargo rullst make:migration` in that case. Turso-primary projects are not
supported, and neither is a project without a database migration runner (a
`--no-database` starter): create the project with a database so `src/main.rs`
declares `pub mod migrations;` and calls
`rullst::artisan!(crate::migrations::get_migrations())`. It does not support an
`auth --api` flag, and it does not silently register application routes. Review
the generated diff before editing it.

---

## Step 2: Register the generated handlers

Wire the generated view, submit, logout, and authenticated routes in the
application router. The exact route tree is an application decision; a typical
mapping is:

```rust,ignore
use rullst::{routes, routing::post, server::from_fn};
use crate::controllers::auth_controller;
use crate::middlewares::auth_middleware::auth_middleware;

let public_auth = routes![
    get("/login" => auth_controller::login_view),
    get("/register" => auth_controller::register_view),
    post("/logout" => auth_controller::logout),
]
// Each credential submission runs Argon2id: budget it per client.
.route("/login", post(auth_controller::login_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)))
.route("/register", post(auth_controller::register_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)));

let signed_in = routes![
    get("/dashboard" => auth_controller::dashboard),
    // Ends the session on every device.
    post("/logout/everywhere" => auth_controller::logout_everywhere),
]
.layer(from_fn(auth_middleware));
```

`credential_rate_limit` allows ten submissions per minute for each connecting
peer. Behind a reverse proxy, configure `[security] trusted_proxies` so clients
do not share the proxy's budget.

The generated form pages expect the Core CSRF and CSP-nonce extensions. Apply
the canonical security baseline and place authenticated routes behind the
generated authentication middleware.

---

## Step 3: Migrate and test

```bash
cargo rullst db:migrate
cargo rullst dev
```

Before production, exercise registration, login, logout, sign-out everywhere,
invalid credentials, duplicate email, expired/tampered/revoked cookies, CSRF
failure, rate limiting, and key rotation. Configure a strong `APP_KEY` through a secret manager and HTTPS at the
edge.

---

## What the scaffold currently enforces

- Passwords are hashed with Argon2id on Tokio's blocking pool; plaintext is not
  written to the user model. At most four hashes or verifications run at once
  (each holds about 19 MiB); a submission that waits two seconds without
  capacity receives 503. Adjust `MAX_CONCURRENT_PASSWORD_WORK` to the host.
- Registration accepts passwords of at least 12 characters and at most 72
  bytes, and normalizes email.
- Login performs a dummy password verification for unknown users to reduce the
  obvious account-enumeration timing difference.
- Session values use authenticated encryption and are emitted as cookie headers
  through `rullst-auth` helpers.
- The `users.session_epoch` column (0 for a new account) goes into every login
  cookie, and `auth_middleware` refuses a cookie whose epoch differs.
  `User::revoke_sessions` increments it in one `UPDATE`, ending every cookie of
  the account; the *Sign out everywhere* button (`logout_everywhere`) calls it.
  Plain logout clears only the current browser's cookie. Call
  `User::revoke_sessions` too from the password change or reset you add, and
  when you deactivate an account. See
  [encrypted sessions](../crates/auth.md#revocable-cookies-the-session-epoch).

These controls do not replace application review. Add account verification,
password change and reset/recovery, abuse controls, audit policy, MFA/passkeys,
per-device session management, and privacy/retention behavior according to the
product's threat model.
