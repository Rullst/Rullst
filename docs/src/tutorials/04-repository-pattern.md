# Tutorial 04: Data Mapper & Repository Pattern 🏗️

For domain-heavy code, Rullst exposes a small `Repository<T>` contract. The
framework does not invent SQL for this trait: your adapter owns its pool,
queries, transactions, and error type. This keeps persistence behavior explicit
and makes an in-memory implementation straightforward in unit tests.

`GenericRepository<T>` is currently only a zero-state marker/helper. It does
not accept a pool and does not provide methods such as `find_one_by`.

---

## Step 1: Implement a PostgreSQL repository

```rust,no_run
use rullst_orm::{async_trait, sqlx, FromRow, Repository};

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Clone)]
pub struct PgUserRepository {
    pool: sqlx::PgPool,
}

impl PgUserRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_email(
        &self,
        email: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }
}

#[async_trait]
impl Repository<User> for PgUserRepository {
    type Id = i64;
    type Error = sqlx::Error;

    async fn find_by_id(&self, id: i64) -> Result<Option<User>, Self::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_all(&self) -> Result<Vec<User>, Self::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn save(&self, user: &User) -> Result<(), Self::Error> {
        sqlx::query(
            "INSERT INTO users (id, name, email) VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO UPDATE SET name = $2, email = $3",
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.email)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), Self::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

All values are parameterized. SQL identifiers and the query shape remain
application-owned source code.

This adapter targets PostgreSQL (`PgPool`, `$1` placeholders) and a `users`
table with `BIGINT` ids. Inside a generated project, `rullst::db::Orm::pool()?`
returns the application's pool as `RullstPool`, which is `sqlx::SqlitePool`
with the SQLite starter and `sqlx::PgPool` with PostgreSQL: clone it into the
adapter and write the queries for that database.

---

## Step 2: Inject the concrete adapter through Axum state

```rust,ignore
use std::sync::Arc;
use rullst::web::axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EmailQuery {
    email: String,
}

pub async fn get_user_by_email(
    State(repository): State<Arc<PgUserRepository>>,
    Query(query): Query<EmailQuery>,
) -> Result<Json<User>, StatusCode> {
    repository
        .find_by_email(&query.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
```

`rullst::Router` has no `with_state`: build this route on an `axum::Router`,
call `.with_state(Arc::new(repository))` and mount it with `merge_axum`, as
[Tutorial 08](08-routing-and-middlewares.md#step-3-share-application-state)
shows. Returning `Json<User>` also requires `User` to derive
`serde::Serialize`.
Production handlers should map internal database errors to an application error
without returning query or credential details to clients.

---

## Key takeaways

- Use derived Active Record models for direct typed CRUD.
- Use `Repository<T>` when the domain needs an explicit persistence boundary.
- The repository implementation, not the trait, determines the supported
  backend and SQL dialect.
- Add tenant/owner predicates inside repository queries where the resource is
  tenant- or user-scoped; dependency injection is not an authorization check.
