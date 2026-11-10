use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm)]
#[orm(table = "validation_records")]
struct ValidationRecord {
    id: i32,
    name: String,
}

// Columns whose generated helpers would repeat a fixed builder member name:
// the update builder's model reference, `where_raw`, `where_exists`,
// `where_column`, `where_col`, `where_similar`, `order_by_desc` and the
// update builder's `save`.
#[derive(Clone, Debug, rullst_orm::Orm)]
#[orm(table = "chat_logs")]
struct ChatLog {
    id: i32,
    model: String,
    raw: String,
    desc: String,
    exists: String,
    column: String,
    col: String,
    similar: String,
    save: String,
}

#[test]
fn columns_named_like_generated_helpers_compile_and_keep_fixed_methods() {
    let query = ChatLog::query()
        .where_model("gpt")
        .where_raw("raw = ?", vec!["payload"])
        .where_eq("exists", "yes")
        .order_by_desc("desc");
    assert!(query.errors.is_empty(), "{:?}", query.errors);
    assert_eq!(
        query.to_sql(),
        "SELECT * FROM chat_logs WHERE ((model = ?) AND (raw = ?) AND (exists = ?)) ORDER BY desc DESC LIMIT 1000"
    );

    let mut log = ChatLog {
        id: 1,
        model: "gpt".to_string(),
        raw: String::new(),
        desc: String::new(),
        exists: String::new(),
        column: String::new(),
        col: String::new(),
        similar: String::new(),
        save: String::new(),
    };
    let _patch = log
        .update_partial()
        .model("claude".to_string())
        .raw(String::new());
}

#[test]
fn joins_and_vector_helpers_reject_dynamic_sql_fragments_that_are_not_safe() {
    let injected_join = ValidationRecord::query().join(
        "other_records",
        "validation_records.id",
        "= 1; DROP TABLE validation_records; --",
        "other_records.id",
    );
    assert!(
        injected_join
            .errors
            .iter()
            .any(|error| error.to_string().contains("invalid operator"))
    );

    let constrained_join = ValidationRecord::query().join_constrained("other_records", |join| {
        join.on("validation_records.id", "OR 1=1", "other_records.id")
    });
    assert!(
        constrained_join
            .errors
            .iter()
            .any(|error| error.to_string().contains("invalid operator"))
    );

    let invalid_vector = ValidationRecord::query()
        .order_by_similarity("embedding", vec![])
        .where_similar("embedding", vec![f64::NAN], -1.0);
    assert!(invalid_vector.errors.len() >= 3);
}

#[derive(Clone, Debug, rullst_orm::Orm)]
#[orm(table = "secret_customers")]
struct SecretCustomer {
    id: i32,
    name: String,
    tax_id: rullst_orm::SecretString,
}

/// Columns the database manages: read and filtered, never written by the ORM.
#[derive(Clone, Debug, rullst_orm::Orm)]
#[orm(table = "managed_accounts")]
struct ManagedAccount {
    id: i32,
    #[orm(db_default)]
    is_admin: bool,
    #[orm(db_default)]
    session_epoch: i64,
}

#[test]
fn database_managed_columns_stay_filterable() {
    let query = ManagedAccount::query()
        .where_eq("is_admin", true)
        .where_eq("managed_accounts.session_epoch", 0_i64)
        .order_by("session_epoch")
        .group_by("is_admin")
        .select(&["id", "is_admin", "session_epoch"]);
    assert!(query.errors.is_empty(), "{:?}", query.errors);
    assert_eq!(
        query.to_sql(),
        "SELECT id, is_admin, session_epoch FROM managed_accounts WHERE ((is_admin = ?) AND (managed_accounts.session_epoch = ?)) GROUP BY is_admin ORDER BY session_epoch ASC LIMIT 1000"
    );
}

#[derive(Clone, Debug, Default, rullst_orm::Orm)]
#[orm(table = "cached_records")]
struct CachedRecord {
    id: i32,
    #[allow(dead_code)]
    #[sqlx(default, skip)]
    session_cache: String,
    // `#[orm(skip)]` alone would leave SQLx reading a missing column.
    #[allow(dead_code)]
    #[orm(skip)]
    #[sqlx(skip)]
    draft: String,
    // Read from the row, but left out of writes and of the query builder.
    #[allow(dead_code)]
    #[orm(skip)]
    member_status: String,
}

fn rejects_secret_string(query: &SecretCustomerQueryBuilder) -> bool {
    matches!(
        query.errors.first(),
        Some(rullst_orm::Error::Validation(message)) if message.contains("SecretString")
    )
}

#[test]
fn secret_string_columns_cannot_be_filtered_ordered_or_grouped() {
    // Each write encrypts with a fresh nonce: a plaintext filter would never match.
    for query in [
        SecretCustomer::query().where_eq("tax_id", "123"),
        SecretCustomer::query().where_in("tax_id", vec!["123"]),
        SecretCustomer::query().where_like("secret_customers.tax_id", "%1%"),
        SecretCustomer::query().where_tax_id("123"),
        SecretCustomer::query().order_by("tax_id"),
        SecretCustomer::query().group_by("tax_id"),
    ] {
        assert!(rejects_secret_string(&query), "{:?}", query.errors);
    }
    // Column names are case-insensitive in SQL, and so are the guards.
    let mixed_case = SecretCustomer::query().where_eq("Tax_Id", "123");
    assert!(
        rejects_secret_string(&mixed_case),
        "{:?}",
        mixed_case.errors
    );
    for (query, column, unread) in [
        (
            CachedRecord::query().where_eq("SESSION_CACHE", "x"),
            "SESSION_CACHE",
            true,
        ),
        (
            CachedRecord::query().select(&["cached_records.Session_Cache"]),
            "Session_Cache",
            true,
        ),
        (CachedRecord::query().order_by("draft"), "draft", true),
        (
            CachedRecord::query().where_eq("Member_Status", "active"),
            "Member_Status",
            false,
        ),
        (
            CachedRecord::query().group_by("cached_records.member_status"),
            "member_status",
            false,
        ),
    ] {
        // `#[orm(skip)]` does not stop `FromRow` from reading the column, so
        // its error does not claim the table lacks it.
        let expected = if unread {
            format!(
                "Validation error: column `{column}` is declared `#[sqlx(skip)]`, so the model does not read it from the database and the query builder cannot use it in WHERE / ORDER BY / GROUP BY / SELECT; if the table has this column and it must be filterable, declare it `#[orm(db_default)]` instead: it is read and filtered, and ORM writes leave it to the database"
            )
        } else {
            format!(
                "Validation error: column `{column}` is declared `#[orm(skip)]`, which leaves it out of generated writes and of the query builder but not out of the model's reads, so it cannot be used in WHERE / ORDER BY / GROUP BY / SELECT; to filter a column that the table has, declare it `#[orm(db_default)]` instead: it is read and filtered, and ORM writes leave it to the database"
            )
        };
        assert_eq!(
            query
                .errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [expected],
        );
    }
    // The SQLx codec decrypts a selected column while decoding the model.
    let selected = SecretCustomer::query().select(&["id", "name", "tax_id"]);
    assert!(selected.errors.is_empty(), "{:?}", selected.errors);
}

#[tokio::test]
async fn secret_string_columns_cannot_be_plucked() {
    for column in ["tax_id", "secret_customers.tax_id"] {
        let plucked = SecretCustomer::query().pluck_string(column).await;
        assert!(
            matches!(&plucked, Err(rullst_orm::Error::Validation(message)) if message.contains("SecretString")),
            "{plucked:?}"
        );
    }
}
