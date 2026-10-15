#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{BindRullstValue, RullstValue};

#[test]
fn wide_integers_and_options_convert_to_their_own_variants() {
    assert!(matches!(
        RullstValue::from(5_000_000_000_i64),
        RullstValue::I64(5_000_000_000)
    ));
    assert!(matches!(
        RullstValue::from(4_000_000_000_u32),
        RullstValue::I64(4_000_000_000)
    ));
    assert!(matches!(
        RullstValue::from(Some(7_i64)),
        RullstValue::I64(7)
    ));
    assert!(matches!(RullstValue::from(Some(3)), RullstValue::Int(3)));
    assert!(matches!(RullstValue::from(Some("x")), RullstValue::String(ref value) if value == "x"));
    assert!(matches!(RullstValue::from(None::<i32>), RullstValue::Null));
    assert!(matches!(
        RullstValue::from(None::<String>),
        RullstValue::Null
    ));
    // An integer literal keeps its i32 default.
    let literal: RullstValue = 42.into();
    assert!(matches!(literal, RullstValue::Int(42)));
}

#[test]
fn i64_reads_wide_and_narrow_integers_only() {
    assert_eq!(
        i64::try_from(RullstValue::I64(-5_000_000_000)),
        Ok(-5_000_000_000)
    );
    assert_eq!(i64::try_from(RullstValue::Int(-3)), Ok(-3));
    assert_eq!(
        i64::try_from(RullstValue::String("3".to_string())),
        Err("Not an i64")
    );
    assert_eq!(i64::try_from(RullstValue::Null), Err("Not an i64"));
    assert!(i32::try_from(RullstValue::I64(3)).is_err());
}

#[test]
fn every_value_binds_exactly_one_parameter() {
    use sqlx::{Arguments, Execute};

    let values = [
        RullstValue::String("a".to_string()),
        RullstValue::Int(1),
        RullstValue::Float(1.5),
        RullstValue::Bool(true),
        RullstValue::I64(5_000_000_000),
        RullstValue::Null,
    ];
    let mut query = sqlx::query::<crate::RullstDatabase>(sqlx::AssertSqlSafe("SELECT 1"));
    let mut query_as =
        sqlx::query_as::<crate::RullstDatabase, (i32,)>(sqlx::AssertSqlSafe("SELECT 1"));
    for value in &values {
        query = query.bind_rullst_value(value);
        query_as = query_as.bind_rullst_value(value);
    }
    assert_eq!(query.take_arguments().unwrap().unwrap().len(), 6);
    assert_eq!(query_as.take_arguments().unwrap().unwrap().len(), 6);
}
