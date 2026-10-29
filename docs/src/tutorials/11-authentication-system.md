# Tutorial 11: Authentication Scaffolding 🔒

The authentication generator creates a reviewable starting point for
cookie-session login and registration. It is application source code, not a
hosted identity service or a claim that every production policy is automatic.

---

## Step 1: Run the generator

The SaaS starter already contains this code. In a Blank or API project with a
database, such as the `shop` project of [Tutorial 02](02-cli-generators.md),
run from the project root:

```bash
cargo rullst auth
```

Those starters ship a minimal `users` migration
(`src/migrations/m20260601000000_create_users_table.rs`), and the command
refuses to add a second one. In a learning project whose `users` table holds
no data, delete that migration file; if `cargo rullst dev` or
`cargo rullst db:migrate` already ran, also delete the local SQLite database
(`db.sqlite`, with any `db.sqlite-wal`/`db.sqlite-shm` beside it), which the
next run recreates with every migration. Then run `cargo rullst auth` again.
In an application with real accounts, keep the migration and add the columns
with `cargo rullst make:migration` instead, as the error message says.

The current command creates:

- `src/models/user.rs` and `src/models/password_reset.rs`;
- `src/migrations/m<timestamp>_create_users_table.rs`,
  `src/migrations/m<timestamp>_create_password_reset_tokens_table.rs` and the
  migrations module;
- `src/controllers/auth_controller.rs` and
  `src/controllers/password_controller.rs`;
- `src/middlewares/auth_middleware.rs`;
- `src/pages/auth.rs`;
- the corresponding module declarations, including `controllers`,
  `middlewares`, `models` and `pages` in `src/lib.rs` (or `src/main.rs`); and
- a best-effort refresh of the generated `.llms.txt`,
  `.rullst/context-map.json` and `diagram.md` (a hand-written `diagram.md` is
  left unchanged).

It also enables the `orm`, `auth` and `mail` umbrella features in
`Cargo.toml`. It fails before writing anything when one of those files exists,
when a `*_create_password_reset_tokens_table.rs` migration exists, or when a
`*_create_users.rs`/`*_create_users_table.rs` migration already creates the
users table (see above). Turso-primary projects are not
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

Mount the password flows the same way: the reset routes are public, the change
routes need `auth_middleware`, and every submission is rate limited.

```rust,ignore
use crate::controllers::password_controller;

let password_reset = routes![
    get("/password/forgot" => password_controller::forgot_password_view),
    get("/password/reset" => password_controller::reset_password_view),
]
// Reset requests send mail and resets run Argon2id.
.route("/password/forgot", post(password_controller::forgot_password_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)))
.route("/password/reset", post(password_controller::reset_password_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)));

let password_change = routes![
    get("/password/change" => password_controller::change_password_view),
]
.route("/password/change", post(password_controller::change_password_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)))
.layer(from_fn(auth_middleware));
```

Add each group to the starter's router in `router()` (`src/main.rs`) with
`.merge_axum(public_auth.into_axum())` and so on, before its
`.layer(rullst::server::from_fn(rullst::security::csrf_middleware))` line so
that CSRF protection covers them. The SaaS starter's `router()` shows the same
routes registered one by one.

`credential_rate_limit` allows ten submissions per minute for each connecting
peer. Behind a reverse proxy, configure `[security] trusted_proxies` so clients
do not share the proxy's budget.

Reset emails go through the `Mail` facade: set `MAIL_DRIVER` and `MAIL_FROM`
for staging and production, and `RULLST_PUBLIC_ORIGIN` to the application's
HTTPS origin (for example `https://app.example.com`), which every reset link
uses. Links are never built from the request's `Host` header. Without
`RULLST_PUBLIC_ORIGIN`, development links point to `http://localhost:3000`,
and staging and production log an error instead of sending a link. In
development the default mail driver logs only metadata, so a debug build
without `MAIL_DRIVER` also prints the reset link to the terminal.

The emails name your product `APP_NAME` (process environment, then `.env`)
or, when it is unset, the `APPLICATION_NAME` constant at the top of
`password_controller.rs`, which starts as the package name. They are written
in the request's language: the locale `LOCALES.layer()` negotiated when the
application uses `rullst::i18n`, otherwise the browser's `Accept-Language`,
matched by language to English, Brazilian Portuguese or Spanish, otherwise
`DEFAULT_MAIL_LOCALE` (English), which you can edit there too.

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
password change and reset, invalid credentials, duplicate email,
expired/tampered/revoked cookies and reset links, CSRF failure, rate limiting,
mail delivery from your provider, and key rotation. Configure a strong `APP_KEY` through a secret manager and HTTPS at the
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
  `User::revoke_sessions` when you deactivate an account. See
  [encrypted sessions](../crates/auth.md#revocable-cookies-the-session-epoch).
- **Change password** (`/password/change`, signed in) asks for the current
  password, the new one and a confirmation. A wrong current password gets the
  generic "The current password is incorrect". The new password follows the
  registration policy. `User::change_password` stores the new hash and
  increments the session epoch in one `UPDATE` that only applies while the
  account is still at the epoch of the asking cookie; that browser receives a
  cookie with the new epoch and every other device is signed out. A reset
  link requested before the change stops working.
- **Forgot password** (`/password/forgot`) answers with the same page, after at
  least 400 ms, whether or not the email has an account. For an account it
  creates a `PasswordResetToken` (32 random bytes from the operating system),
  stores only its SHA-256 digest in `password_reset_tokens` with a 30-minute
  expiry, and emails the link in the background. The table keeps one token per
  account: a new request replaces the previous token, but requests for the
  same account less than 60 seconds apart send nothing.
- **Reset link lifetime**: `RESET_TOKEN_LIFETIME_MINUTES` in
  `src/models/password_reset.rs` (30 as generated) sets it. Change it there:
  a compile-time assertion keeps it from 10 minutes to 24 hours (1440), and
  the stored expiry, the email ("This link expires in 30 minutes.", or
  "2 hours" for whole hours) and the forgot-password answer all follow it.
  Below 10 minutes, mail that a provider greylists or queues for a few minutes
  arrives after its link expired, and the user can only ask again. Above 24
  hours, the link stays a working way into the account for as long as it sits
  in a mailbox that is synchronized to several devices, forwarded or read by
  someone else, while a day already covers a user who reads mail once a day.
  Single use and replacement by a newer request narrow that exposure but do
  not remove it, which is why the
  [OWASP Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html)
  asks for short-lived tokens.
- **Reset password** (`/password/reset?token=…`) shows the form only for a live
  token, without caching and with `Referrer-Policy: no-referrer`. Submitting
  it deletes the token, stores the new hash and increments the session epoch
  in one transaction, so the link works once and every device is signed out.
  The user is not signed in: the page redirects to `/login?reset=done`. Both
  flows email a "password changed" notice.

These controls do not replace application review. Add account verification,
abuse controls, audit policy, MFA/passkeys, per-device session management, and
privacy/retention behavior according to the product's threat model. A reset
link proves only access to the mailbox; for high-value accounts, add a second
factor to recovery.
