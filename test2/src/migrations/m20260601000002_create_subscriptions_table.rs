use rullst::db::schema::{Schema, Blueprint, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000002_create_subscriptions_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("subscriptions", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.string("customer_id").not_null();
            table.string("subscription_id").not_null();
            table.string("plan_id").not_null();
            table.string("status").not_null();
            table.integer("ends_at").nullable();
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("subscriptions").await
    }
}
