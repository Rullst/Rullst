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
