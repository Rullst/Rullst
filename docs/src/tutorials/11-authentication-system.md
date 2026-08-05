# Tutorial 11: Authentication Scaffolding 🔒

Scaffold a complete authentication system (Login, Registration, Logout, Passwords, Session/JWT) in a single command.

---

## 🛠️ Step 1: Run the Auth Generator

```bash
cargo rullst auth
```

For headless JSON APIs (no HTML views):
```bash
cargo rullst auth --api
```

This automatically generates:
- **Model:** `src/models/user.rs` (with `argon2`/`bcrypt` password hashing)
- **Migration:** `migrations/<timestamp>_create_users_table.sql`
- **Controllers:** `src/controllers/auth_controller.rs`
- **Views:** `views/auth/login.html` & `views/auth/register.html`

---

## 💻 Step 2: Test Registration & Login

Start dev mode:
```bash
cargo rullst dev
```

Navigate to `http://localhost:3000/register` to create your first user account!

---

## 💡 Key Takeaways
- Password hashing is enforced using Argon2id with memory-hard parameters.
- User sessions are stored securely in HTTP-only, SameSite cookies.
