# Tutorial 04: Data Mapper & Repository Pattern 🏗️

For complex domain logic, clean architecture, or DDD (Domain-Driven Design), Rullst ORM supports the **Repository Pattern** alongside Active Record models.

---

## 🛠️ Step 1: Implement the Generic Repository

```rust
use rullst_orm::repository::{Repository, GenericRepository};
use crate::models::User;

pub struct UserRepository {
    repo: GenericRepository<User>,
}

impl UserRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            repo: GenericRepository::new(pool),
        }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, rullst_core::AppError> {
        self.repo.find_one_by("email", email).await
    }
}
```

---

## 💻 Step 2: Use Repositories inside Controllers

```rust
use rullst_core::di::Inject;
use axum::Json;

pub async fn get_user_by_email(
    user_repo: Inject<UserRepository>,
    email: String,
) -> Result<Json<User>, rullst_core::AppError> {
    let user = user_repo.find_by_email(&email).await?
        .ok_or_else(|| rullst_core::AppError::NotFound("User not found".into()))?;
        
    Ok(Json(user))
}
```

---

## 💡 Key Takeaways
- Use **Active Record** for rapid CRUD, prototype features, and standard business entities.
- Use **Repository Pattern** when decoupling domain models from database table schemas or writing unit tests with mock pools.
