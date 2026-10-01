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

The current command creates or updates:

- `src/models/user.rs`;
- `src/migrations/m<timestamp>_create_users_table.rs` and the migrations module;
- `src/controllers/auth_controller.rs`;
- `src/middlewares/auth_middleware.rs`;
- `src/pages/auth.rs`; and
- the corresponding module declarations.

It does not support an `auth --api` flag, and it does not silently register
application routes. Review the generated diff before editing or rerunning the
command.

---

## Step 2: Register the generated handlers

Wire the generated view, submit, logout, and authenticated routes in the
application router. The exact route tree is an application decision; a typical
mapping is:

```rust,ignore
use rullst::{routes, routing::post, server::from_fn};
use crate::controllers::auth_controller;

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

Before production, exercise registration, login, logout, invalid credentials,
duplicate email, expired/tampered cookies, CSRF failure, rate limiting, and key
rotation. Configure a strong `APP_KEY` through a secret manager and HTTPS at the
edge.

---

## What the scaffold currently enforces

- Passwords are hashed with Argon2id on Tokio's blocking pool; plaintext is not
  written to the user model. At most four hashes or verifications run at once
  (each holds about 19 MiB); a submission that waits two seconds without
  capacity receives 503. Adjust `MAX_CONCURRENT_PASSWORD_WORK` to the host.
- Registration accepts passwords from 12 through 72 bytes and normalizes email.
- Login performs a dummy password verification for unknown users to reduce the
  obvious account-enumeration timing difference.
- Session values use authenticated encryption and are emitted as cookie headers
  through `rullst-auth` helpers.

These controls do not replace application review. Add account verification,
password reset/recovery, abuse controls, audit policy, MFA/passkeys, session
revocation, and privacy/retention behavior according to the product's threat
model.
