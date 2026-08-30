use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "validation_records")]
struct ValidationRecord {
    id: i32,
    name: String,
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
