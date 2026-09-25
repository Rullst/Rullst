//! Application-owned SQLite schema, inserted into a CLI-generated migration.
use rullst::db::{Orm, sqlx};
use rullst_orm::{async_trait, error::RullstError, schema::Migration};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "__JOURNEY_MIGRATION_NAME__"
    }

    async fn up(&self) -> Result<(), RullstError> {
        let pool = Orm::pool()?;
        sqlx::query(
            "CREATE TABLE journey_memberships (user_id INTEGER NOT NULL REFERENCES users(id), \
             tenant_id TEXT NOT NULL, PRIMARY KEY (user_id, tenant_id))",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "CREATE TABLE journey_notes (id INTEGER PRIMARY KEY AUTOINCREMENT, \
             tenant_id TEXT NOT NULL, owner_id INTEGER NOT NULL REFERENCES users(id), \
             body TEXT NOT NULL CHECK (length(body) BETWEEN 1 AND 256))",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), RullstError> {
        let pool = Orm::pool()?;
        sqlx::query("DROP TABLE journey_notes")
            .execute(pool)
            .await?;
        sqlx::query("DROP TABLE journey_memberships")
            .execute(pool)
            .await?;
        Ok(())
    }
}
