# Tutorial 13: RBAC, Ownership, and IDOR/BOLA Protection 🛡️

`rullst-security` provides role, owner, and tenant checks over a trusted
`UserContext`. The framework cannot infer ownership from a route parameter: the
application must load the resource and pass its stored owner/tenant identifiers
to the guard.

---

## Step 1: Authorize a trusted user context

```rust
use rullst_security::{RbacGuard, SecurityError, UserContext};

pub fn authorize_admin(user: &UserContext) -> Result<(), SecurityError> {
    RbacGuard::authorize(user, "admin")
}
```

Construct `UserContext` only after authentication. Roles, permissions, and
tenant membership must come from trusted server-side state, not request headers
or JSON supplied by the caller.

---

## Step 2: Check the resource record, not request ownership

```rust
use rullst_security::{RbacGuard, SecurityError, UserContext};

pub struct DocumentAccess {
    pub owner_user_id: String,
    pub tenant_id: String,
}

pub fn authorize_document_update(
    user: &UserContext,
    stored: &DocumentAccess,
) -> Result<(), SecurityError> {
    RbacGuard::authorize_tenant_owner_or_role(
        user,
        &stored.tenant_id,
        &stored.owner_user_id,
        "document-editor",
    )
}
```

Load `DocumentAccess` with a parameterized query by the route's document ID. Do
not accept `owner_user_id` or `tenant_id` from the update payload. The tenant
guard is evaluated first and roles — including `admin` — do not bypass a tenant
mismatch.

For particularly sensitive paths, make the database query itself tenant-scoped
and then apply the guard as a second boundary. Return the same not-found/forbidden
shape where revealing resource existence would leak information.

---

## Key takeaways

- `authorize` checks a role (and recognizes the framework's `admin` role).
- `authorize_owner_or_role` is safe only when the owner ID came from trusted
  resource state.
- Use `authorize_tenant_owner_or_role` for tenant-bound resources.
- A helper contributes to IDOR/BOLA prevention only when every parameterized
  resource route invokes it or an equivalent scoped repository policy.
