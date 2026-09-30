//! Driver-specific DDL for the typed `Blueprint` column helpers.
#![allow(clippy::expect_used)]

use super::{Blueprint, ColumnDefault};

fn ddl(blueprint: &Blueprint, driver: &str) -> String {
    blueprint.build_for_driver(driver).expect("column DDL")
}

#[test]
fn float_columns_keep_double_precision_on_every_driver() {
    let mut blueprint = Blueprint::new();
    blueprint.float("price").not_null();
    blueprint
        .float("discount")
        .default(ColumnDefault::Float(0.5));

    let postgres = ddl(&blueprint, "postgres");
    assert!(
        postgres.contains("price DOUBLE PRECISION NOT NULL"),
        "{postgres}"
    );
    assert!(
        postgres.contains("discount DOUBLE PRECISION DEFAULT 0.5"),
        "{postgres}"
    );
    let mysql = ddl(&blueprint, "mysql");
    assert!(mysql.contains("price DOUBLE NOT NULL"), "{mysql}");
    assert!(mysql.contains("discount DOUBLE DEFAULT 0.5"), "{mysql}");
    let sqlite = ddl(&blueprint, "sqlite");
    assert!(sqlite.contains("price REAL NOT NULL"), "{sqlite}");
    assert!(!postgres.contains("REAL"), "{postgres}");
}

#[test]
fn an_explicitly_replaced_column_type_is_kept() {
    let mut blueprint = Blueprint::new();
    blueprint.float("ratio").col_type = "NUMERIC(10, 4)".to_string();
    for driver in ["postgres", "mysql", "sqlite"] {
        assert!(
            ddl(&blueprint, driver).contains("ratio NUMERIC(10, 4)"),
            "{driver}"
        );
    }
}
