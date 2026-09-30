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

#[test]
fn boolean_columns_are_native_booleans_on_postgres() {
    let mut blueprint = Blueprint::new();
    blueprint
        .boolean("active")
        .not_null()
        .default(ColumnDefault::Integer(1));
    blueprint
        .boolean("archived")
        .default(ColumnDefault::Integer(0));

    let postgres = ddl(&blueprint, "postgres");
    assert!(
        postgres.contains("active BOOLEAN NOT NULL DEFAULT TRUE"),
        "{postgres}"
    );
    assert!(
        postgres.contains("archived BOOLEAN DEFAULT FALSE"),
        "{postgres}"
    );
    for driver in ["mysql", "sqlite"] {
        let sql = ddl(&blueprint, driver);
        assert!(sql.contains("active INTEGER NOT NULL DEFAULT 1"), "{sql}");
        assert!(sql.contains("archived INTEGER DEFAULT 0"), "{sql}");
    }

    let mut invalid = Blueprint::new();
    invalid.boolean("flag").default(ColumnDefault::Integer(2));
    assert!(invalid.build_for_driver("postgres").is_err());
    assert!(invalid.build_for_driver("sqlite").is_ok());
}
