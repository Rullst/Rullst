# Tutorial 03: Active Record CRUD Operations 🗄️

`rullst-orm` provides expressive Active Record primitives for rapid CRUD development, handling 90% of standard business logic without boilerplate.

---

## 🛠️ Step 1: Define an Active Record Model

In `src/models/user.rs`:

```rust
use rullst_orm::prelude::*;

#[derive(Debug, Serialize, Deserialize, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: PrimaryKey<i64>,
    pub name: String,
    pub email: String,
    pub created_at: DateTime,
}
```

---

## 💻 Step 2: Perform CRUD Operations

### Create
```bash
let user = User::create(json!({
    "name": "Alice Developer",
    "email": "alice@example.com"
})).await?;
```

### Read / Find by ID
```rust
let user = User::find(1).await?;
let active_users = User::where_clause("email LIKE ?", vec!["%@example.com"]).await?;
```

### Update
```rust
let mut user = User::find(1).await?;
user.name = "Alice Smith".to_string();
user.save().await?;
```

### Delete
```rust
let user = User::find(1).await?;
user.delete().await?;
```

---

## 💡 Key Takeaways
- Active Record methods (`find`, `create`, `save`, `delete`) execute borrow-checker safe queries over standard SQLx pools.
- Custom scopes and soft deletes can be configured using attributes like `#[orm(soft_delete)]`.
