use rullst_orm::schema::{Schema, Migration};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20261008071503_create_notes"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("notes", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.string("title").not_null();
            table.string("body").not_null();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("notes").await
    }
}
