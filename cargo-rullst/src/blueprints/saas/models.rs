// cargo-rullst/src/blueprints/saas/models.rs — Database models and migrations for SaaS blueprint.

pub fn get_models_and_migrations() -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let user_model = r##"use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }
}

impl NexusModel for User {
    fn nexus_table() -> &'static str { "users" }
    fn nexus_label() -> &'static str { "Users" }
    fn nexus_icon() -> &'static str { "👥" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "email", label: "Email", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "password_hash", label: "Password Hash", kind: FieldKind::Text, hidden: true, readonly: false },
            FieldMeta { name: "oauth_provider", label: "OAuth Provider", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "oauth_id", label: "OAuth ID", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;
    manifest.push(("src/models/user.rs", user_model.to_string()));

    let subscription_model = r##"use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "subscriptions")]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub customer_id: String,
    pub subscription_id: String,
    pub plan_id: String,
    pub status: String,
    pub ends_at: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Subscription {
    fn nexus_table() -> &'static str { "subscriptions" }
    fn nexus_label() -> &'static str { "Subscriptions" }
    fn nexus_icon() -> &'static str { "💳" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "user_id", label: "User ID", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "customer_id", label: "Customer ID", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "subscription_id", label: "Subscription ID", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "plan_id", label: "Plan ID", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "ends_at", label: "Ends At", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}

impl Subscription {
    pub async fn find_by_subscription_id(subscription_id: &str) -> Result<Option<Self>, rullst_orm::error::RullstError> {
        let pool = rullst::db::Orm::pool()?;
        rullst::db::sqlx::query_as("SELECT * FROM subscriptions WHERE subscription_id = $1")
            .bind(subscription_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }
}
"##;
    manifest.push(("src/models/subscription.rs", subscription_model.to_string()));

    let billing_customer_model = r##"use rullst::db::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "billing_customers")]
pub struct BillingCustomer {
    pub id: i32,
    pub user_id: i32,
    pub email: String,
    pub customer_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    manifest.push((
        "src/models/billing_customer.rs",
        billing_customer_model.to_string(),
    ));

    let models_mod = r##"pub mod user;
pub mod subscription;
pub mod billing_customer;
"##;
    manifest.push(("src/models/mod.rs", models_mod.to_string()));

    // Migrations
    let m1 = r##"use rullst::db::{Orm, sqlx};
use rullst_orm::schema::{Schema, Migration};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_users_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("users", |table| {
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }).await?;
        sqlx::query("CREATE UNIQUE INDEX users_email_unique ON users(email)")
            .execute(Orm::pool()?)
            .await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("users").await
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000000_create_users_table.rs",
        m1.to_string(),
    ));

    let m3 = r##"use rullst::db::{Orm, sqlx};
use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000002_create_subscriptions_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("billing_customers", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.string("email").not_null();
            table.string("customer_id").nullable();
            table.timestamps();
        }).await?;
        Schema::create("subscriptions", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.string("customer_id").not_null();
            table.string("subscription_id").not_null();
            table.string("plan_id").not_null();
            table.string("status").not_null();
            table.integer("ends_at").nullable();
            table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        sqlx::query(
            "CREATE UNIQUE INDEX billing_customers_email_unique ON billing_customers(email)",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "CREATE UNIQUE INDEX subscriptions_subscription_id_unique ON subscriptions(subscription_id)",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("subscriptions").await?;
        Schema::drop_if_exists("billing_customers").await
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000002_create_subscriptions_table.rs",
        m3.to_string(),
    ));

    let migrations_mod = r##"// Generated by Rullst.
pub mod m20260601000000_create_users_table;
pub mod m20260601000002_create_subscriptions_table;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_users_table::MigrationImpl),
        Box::new(m20260601000002_create_subscriptions_table::MigrationImpl),
    ]
}
"##;
    manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

    manifest
}
