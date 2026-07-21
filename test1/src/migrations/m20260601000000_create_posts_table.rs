use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_posts_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("posts", |table| {
            table.id();
            table.string("title").not_null();
            table.string("slug").not_null();
            table.string("content").not_null();
            table.timestamps();
        }).await?;

        // Seed initial blog posts
        let pool = rullst::db::Orm::pool();
        rullst::db::sqlx::query(
            "INSERT INTO posts (id, title, slug, content, created_at, updated_at) VALUES 
             (1, 'Announcing Rullst: The Ultimate Rust Framework', 'announcing-rullst', 'We are thrilled to announce Rullst, a new full-stack framework combining Axum, HTMX, and SQLite/Postgres for lightning-fast applications.', datetime('now'), datetime('now')),
             (2, 'The Power of WebAssembly Islands', 'power-of-wasm-islands', 'Wasm Islands give you the speed of server-side HTML combined with high-fidelity Wasm interactivity when needed.', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("posts").await
    }
}
