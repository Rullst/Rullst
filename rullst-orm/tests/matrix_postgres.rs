#![cfg(not(any(feature = "strict-sqlite", feature = "strict-mysql")))]

mod partial_update_contract;
mod support;

// Driver-neutral contracts, also run on SQLite by `driver_contract_sqlite`.
mod driver_contract;

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "pg_users", auditable)]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "pg_tenant_posts", tenant_column = "tenant_id")]
struct TenantPost {
    pub id: i32,
    pub tenant_id: String,
    pub author_id: i32,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "pg_tenant_comments", tenant_column = "tenant_id")]
struct TenantComment {
    pub id: i32,
    pub tenant_id: String,
    pub post_id: i32,
    pub status: String,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "pg_flags")]
struct Flag {
    pub id: i32,
    pub active: bool,
}

#[derive(rullst_orm::Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "pg_account_status", rename_all = "snake_case")]
enum AccountStatus {
    AwaitingReview,
    Active,
}

/// An ORM model whose column is a named PostgreSQL enum.
#[cfg(feature = "strict-postgres")]
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "pg_native_enum_accounts")]
struct NativeEnumAccount {
    pub id: i32,
    pub status: AccountStatus,
    pub previous: Option<AccountStatus>,
}

#[cfg(feature = "strict-postgres")]
#[derive(rullst_orm::Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "pg_tx_account_status", rename_all = "snake_case")]
enum TransactionalAccountStatus {
    Pending,
    Settled,
}

#[cfg(feature = "strict-postgres")]
struct ConflictingAccountStatus;

#[cfg(feature = "strict-postgres")]
impl rullst_orm::DatabaseEnum for ConflictingAccountStatus {
    const TYPE_NAME: &'static str = "pg_account_status";
    const VARIANTS: &'static [&'static str] = &["retired"];
}

#[tokio::test]
async fn test_matrix_postgres_crud() {
    // 1. Inicia o container do PostgreSQL
    let container = match Postgres::default().start().await {
        Ok(c) => c,
        Err(e) => {
            support::handle_container_start_error("PostgreSQL", e);
            return;
        }
    };

    let host_ip = container.get_host().await.expect("Failed to get host IP");
    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");

    let connection_string = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        host_ip, host_port
    );

    // 2. Inicializa o ORM com o Postgres real
    Orm::init(&connection_string)
        .await
        .expect("Orm::init should succeed with Postgres");

    // 3. Cria a tabela (Schema Builder deve funcionar no PG)
    Schema::create("pg_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
        t.string("email").not_null();
    })
    .await
    .expect("create pg_users");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create PostgreSQL audit table");
    let audit_context =
        rullst_orm::audit::AuditContext::system("postgres-matrix").expect("valid audit context");

    // 4. Executa um CRUD básico para provar que a gramática gerada estaticamente funciona
    let mut user = User {
        id: 0,
        name: "Alice PG".into(),
        email: "alice@pg.com".into(),
    };

    // INSERT
    rullst_orm::audit::with_audit_context(audit_context.clone(), user.save())
        .await
        .expect("save new user to postgres");
    assert!(user.id > 0, "id must be assigned after save (RETURNING id)");

    // SELECT
    let found = User::find(user.id)
        .await
        .expect("find query executed")
        .expect("user exists");
    assert_eq!(found.name, "Alice PG");

    // UPDATE
    user.name = "Alice PG Updated".into();
    rullst_orm::audit::with_audit_context(audit_context.clone(), user.save())
        .await
        .expect("update user in postgres");

    let updated = User::find(user.id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Alice PG Updated");
    let audit_id: (i32,) = sqlx::query_as(
        "SELECT id FROM rullst_audits WHERE model_type = $1 AND model_id = $2 AND event = 'updated' ORDER BY id DESC LIMIT 1",
    )
    .bind("pg_users")
    .bind(user.id)
    .fetch_one(Orm::pool().expect("PostgreSQL pool"))
    .await
    .expect("read PostgreSQL audit revision");
    user = rullst_orm::audit::with_audit_context(
        audit_context.clone(),
        user.restore_revision(audit_id.0, "matrix rollback"),
    )
    .await
    .expect("restore PostgreSQL revision");
    assert_eq!(user.name, "Alice PG");

    // DELETE
    rullst_orm::audit::with_audit_context(audit_context, user.delete())
        .await
        .expect("delete executed");

    let not_found = User::find(user.id).await.unwrap();
    assert!(not_found.is_none());

    support::exercise_outbox().await;
    support::exercise_large_audit_payload().await;
    support::migrations::exercise_foreign_migrations_table().await;
    partial_update_contract::exercise().await;
    exercise_tenant_subqueries().await;
    exercise_boolean_columns().await;

    #[cfg(feature = "strict-postgres")]
    exercise_native_enum().await;
    #[cfg(not(feature = "strict-postgres"))]
    exercise_dynamic_pool_enum_refusal().await;
    driver_contract::exercise().await;
}

/// `Blueprint::boolean` creates a native PostgreSQL boolean, so `bool` model
/// fields insert, decode, filter and take their default.
async fn exercise_boolean_columns() {
    Schema::create("pg_flags", |table: &mut Blueprint| {
        table.id();
        table
            .boolean("active")
            .not_null()
            .default(rullst_orm::schema::ColumnDefault::Integer(0));
    })
    .await
    .expect("create PostgreSQL boolean table");

    let mut flag = Flag {
        id: 0,
        active: true,
    };
    flag.save().await.expect("PostgreSQL bool insert");
    let stored = Flag::find(flag.id)
        .await
        .expect("PostgreSQL bool decode")
        .expect("stored flag");
    assert!(stored.active);
    sqlx::query("INSERT INTO pg_flags DEFAULT VALUES")
        .execute(Orm::pool().expect("PostgreSQL pool"))
        .await
        .expect("insert the boolean default");
    let active = Flag::query()
        .where_eq("active", true)
        .count()
        .await
        .expect("PostgreSQL bool filter");
    let inactive = Flag::query()
        .where_eq("active", false)
        .count()
        .await
        .expect("PostgreSQL bool default filter");
    assert_eq!((active, inactive), (1, 1));

    Schema::drop_if_exists("pg_flags")
        .await
        .expect("drop PostgreSQL boolean table");
}

/// Tenant scope, typed CTEs and EXISTS subqueries must share one `$n` sequence.
async fn exercise_tenant_subqueries() {
    let pool = Orm::pool().expect("PostgreSQL pool");
    for statement in [
        "CREATE TABLE pg_tenant_posts (id SERIAL PRIMARY KEY, tenant_id TEXT NOT NULL, author_id INTEGER NOT NULL)",
        "CREATE TABLE pg_tenant_comments (id SERIAL PRIMARY KEY, tenant_id TEXT NOT NULL, post_id INTEGER NOT NULL, status TEXT NOT NULL)",
        "CREATE TABLE pg_tenant_authors (id INTEGER PRIMARY KEY, region TEXT NOT NULL)",
        "INSERT INTO pg_tenant_posts (id, tenant_id, author_id) VALUES (1, 'acme', 1), (2, 'acme', 2), (3, 'other', 1), (4, 'acme', 1)",
        "INSERT INTO pg_tenant_comments (tenant_id, post_id, status) VALUES ('acme', 1, 'open'), ('acme', 2, 'open'), ('other', 3, 'open'), ('acme', 4, 'published'), ('other', 4, 'open')",
        "INSERT INTO pg_tenant_authors (id, region) VALUES (1, 'eu'), (2, 'us')",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("seed PostgreSQL tenant subquery fixture");
    }

    let open_comment = || {
        TenantComment::query()
            .where_column("pg_tenant_comments.post_id", "pg_tenant_posts.id")
            .where_eq("status", "open")
    };
    let (ids, count, deleted, remaining) = rullst_orm::with_tenant("acme", async {
        let query = TenantPost::query()
            .with_cte(
                "published_comments",
                TenantComment::query().where_eq("status", "published"),
            )
            .with_cte(
                "open_comments",
                TenantComment::query()
                    .where_exists(TenantComment::query().where_eq("status", "open"))
                    .where_eq("status", "open"),
            )
            .join_constrained("pg_tenant_authors", |join| {
                join.on("pg_tenant_authors.id", "=", "pg_tenant_posts.author_id")
                    .on_eq("pg_tenant_authors.region", "eu")
            })
            .where_exists(open_comment());
        let ids = query
            .clone()
            .order_by("pg_tenant_posts.id")
            .pluck_i32("pg_tenant_posts.id")
            .await
            .expect("PostgreSQL tenant EXISTS with CTEs and JOIN");
        let count = query.count().await.expect("PostgreSQL ordered count");
        let deleted = TenantPost::query()
            .where_exists(open_comment())
            .delete_all()
            .await
            .expect("PostgreSQL delete_all with an embedded subquery");
        let remaining = TenantPost::query()
            .order_by("id")
            .pluck_i32("id")
            .await
            .expect("remaining PostgreSQL tenant rows");
        (ids, count, deleted, remaining)
    })
    .await;

    assert_eq!(ids, vec![1]);
    assert_eq!(count, 1);
    assert_eq!(deleted, 2);
    assert_eq!(remaining, vec![4]);
}

#[cfg(feature = "strict-postgres")]
async fn exercise_native_enum() {
    Schema::create("pg_native_enum_accounts", |table: &mut Blueprint| {
        table.id();
        table.native_enum::<AccountStatus>("status").not_null();
    })
    .await
    .expect("PostgreSQL named enum schema should be created");

    sqlx::query("INSERT INTO pg_native_enum_accounts (status) VALUES ($1)")
        .bind(AccountStatus::AwaitingReview)
        .execute(Orm::pool().expect("PostgreSQL pool"))
        .await
        .expect("PostgreSQL enum should encode");
    let stored = sqlx::query_scalar::<_, AccountStatus>(
        "SELECT status FROM pg_native_enum_accounts WHERE id = 1",
    )
    .fetch_one(Orm::pool().expect("PostgreSQL pool"))
    .await
    .expect("PostgreSQL enum should decode");
    assert_eq!(stored, AccountStatus::AwaitingReview);
    exercise_native_enum_filters().await;

    let drift = Schema::create("pg_native_enum_conflict", |table: &mut Blueprint| {
        table.id();
        table
            .native_enum::<ConflictingAccountStatus>("status")
            .not_null();
    })
    .await;
    assert!(
        matches!(drift, Err(rullst_orm::Error::Validation(_))),
        "an existing PostgreSQL enum with different labels must fail closed"
    );

    // Enum DDL joins the managed transaction: a rollback also removes the
    // type, and dropping it after its table in one transaction cannot wait on
    // that transaction's own table lock from a second connection.
    let rolled_back = rullst_orm::Orm::transaction(|_| {
        Box::pin(async {
            Schema::create("pg_tx_enum_accounts", |table: &mut Blueprint| {
                table.id();
                table
                    .native_enum::<TransactionalAccountStatus>("status")
                    .not_null();
            })
            .await?;
            Err::<(), rullst_orm::Error>(rullst_orm::Error::Validation(
                "roll back the enum schema".to_string(),
            ))
        })
    })
    .await;
    assert!(rolled_back.is_err());
    assert_eq!(
        postgres_type_count("pg_tx_account_status").await,
        0,
        "a rolled-back Schema::create must not leave its enum type behind"
    );

    let dropped = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        rullst_orm::Orm::transaction(|_| {
            Box::pin(async {
                Schema::drop_if_exists("pg_native_enum_accounts").await?;
                Schema::drop_native_enum::<AccountStatus>().await
            })
        }),
    )
    .await
    .expect("dropping the enum after its table in one transaction must not hang");
    dropped.expect("PostgreSQL enum table and type should be dropped together");
    assert_eq!(postgres_type_count("pg_account_status").await, 0);
}

/// Builder filters bind enum values as text; PostgreSQL has no `enum = text`
/// operator, so enum columns must cast their markers to the named type.
#[cfg(feature = "strict-postgres")]
async fn exercise_native_enum_filters() {
    sqlx::query("ALTER TABLE pg_native_enum_accounts ADD COLUMN previous pg_account_status")
        .execute(Orm::pool().expect("PostgreSQL pool"))
        .await
        .expect("add nullable enum column");
    let mut active = NativeEnumAccount {
        id: 0,
        status: AccountStatus::Active,
        previous: Some(AccountStatus::AwaitingReview),
    };
    active.save().await.expect("save a model with enum columns");

    let by_eq = NativeEnumAccount::query()
        .where_eq("status", AccountStatus::Active)
        .get()
        .await
        .expect("where_eq on a named enum column");
    assert_eq!(
        by_eq.iter().map(|row| row.id).collect::<Vec<_>>(),
        [active.id]
    );
    let by_helper = NativeEnumAccount::query()
        .where_status(AccountStatus::AwaitingReview)
        .get()
        .await
        .expect("generated where_<column> on a named enum column");
    assert_eq!(by_helper.len(), 1);
    let by_in = NativeEnumAccount::query()
        .where_in(
            "pg_native_enum_accounts.status",
            vec![AccountStatus::Active, AccountStatus::AwaitingReview],
        )
        .order_by("id")
        .count()
        .await
        .expect("qualified where_in on a named enum column");
    assert_eq!(by_in, 2);
    let by_previous = NativeEnumAccount::query()
        .where_eq("previous", AccountStatus::AwaitingReview)
        .or_where_not_eq("status", AccountStatus::Active)
        .count()
        .await
        .expect("nullable enum and OR filters");
    assert_eq!(by_previous, 2);
    let ordered = NativeEnumAccount::query()
        .where_gt("status", AccountStatus::AwaitingReview)
        .pluck_i32("id")
        .await
        .expect("enum ordering comparison");
    assert_eq!(ordered, [active.id]);
    active.delete().await.expect("delete the enum row");
}

#[cfg(feature = "strict-postgres")]
async fn postgres_type_count(type_name: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM pg_type WHERE typname = $1")
        .bind(type_name)
        .fetch_one(Orm::pool().expect("PostgreSQL pool"))
        .await
        .expect("inspect PostgreSQL types")
}

#[cfg(not(feature = "strict-postgres"))]
async fn exercise_dynamic_pool_enum_refusal() {
    let unsupported = Schema::create("pg_any_native_enum", |table: &mut Blueprint| {
        table.id();
        table.native_enum::<AccountStatus>("status").not_null();
    })
    .await;
    assert!(
        matches!(
            unsupported,
            Err(rullst_orm::Error::Validation(message))
                if message.contains("strict-postgres")
        ),
        "SQLx Any must fail before creating an unreadable PostgreSQL custom type"
    );
}
