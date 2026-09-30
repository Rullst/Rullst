use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "validation_records")]
struct ValidationRecord {
    id: i32,
    name: String,
}

// Columns whose generated helpers would repeat a fixed builder member name:
// the update builder's model reference, `where_raw`, `where_exists`,
// `where_column`, `where_col`, `where_similar`, `order_by_desc` and the
// update builder's `save`.
#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
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

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "secret_customers")]
struct SecretCustomer {
    id: i32,
    name: String,
    tax_id: rullst_orm::SecretString,
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
