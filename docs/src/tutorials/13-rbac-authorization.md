# Tutorial 13: RBAC Authorization & IDOR Protection 🛡️

Learn how to enforce Role-Based Access Control (RBAC) and prevent Insecure Direct Object Reference (IDOR) attacks using `rullst-security`.

---

## 🛠️ Step 1: Define User Roles & Permissions

```rust
use rullst_security::{RbacGuard, UserContext};
use rullst_core::AppError;

pub async fn admin_only_dashboard(user: UserContext) -> Result<String, AppError> {
    // Authorize role
    RbacGuard::authorize(&user, "admin").map_err(|e| AppError::Forbidden(e))?;
    
    Ok("Welcome to the Secret Admin Panel".to_string())
}
```

---

## 💻 Step 2: Prevent IDOR / BOLA Attacks

Authorize entity ownership dynamically:

```rust
pub async fn update_document(
    user: UserContext,
    doc_owner_id: &str,
) -> Result<(), AppError> {
    // Ensures current user owns the resource or has 'admin' role
    RbacGuard::authorize_owner_or_role(&user, doc_owner_id, "admin")
        .map_err(|e| AppError::Forbidden(e))?;
    
    // Proceed with update
    Ok(())
}
```

---

## 💡 Key Takeaways
- `RbacGuard::authorize` returns an authorization error if permissions fail.
- `authorize_owner_or_role` prevents IDOR/BOLA vulnerabilities across API endpoints.
