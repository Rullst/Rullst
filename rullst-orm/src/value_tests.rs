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
        RullstValue::DateTime(instant()),
        RullstValue::NaiveDateTime(instant().naive_utc()),
        RullstValue::Date(instant().date_naive()),
        RullstValue::Time(instant().time()),
    ];
    let mut query = sqlx::query::<crate::RullstDatabase>(sqlx::AssertSqlSafe("SELECT 1"));
    let mut query_as =
        sqlx::query_as::<crate::RullstDatabase, (i32,)>(sqlx::AssertSqlSafe("SELECT 1"));
    for value in &values {
        query = query.bind_rullst_value(value);
        query_as = query_as.bind_rullst_value(value);
    }
    assert_eq!(query.take_arguments().unwrap().unwrap().len(), 10);
    assert_eq!(query_as.take_arguments().unwrap().unwrap().len(), 10);
}

fn instant() -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc
        .with_ymd_and_hms(2026, 10, 15, 9, 30, 0)
        .single()
        .unwrap()
        + chrono::Duration::milliseconds(500)
}

#[test]
fn chrono_values_convert_to_their_own_variants() {
    let at = instant();
    assert!(matches!(RullstValue::from(at), RullstValue::DateTime(value) if value == at));
    assert!(
        matches!(RullstValue::from(at.naive_utc()), RullstValue::NaiveDateTime(value) if value == at.naive_utc())
    );
    assert!(
        matches!(RullstValue::from(at.date_naive()), RullstValue::Date(value) if value == at.date_naive())
    );
    assert!(matches!(RullstValue::from(at.time()), RullstValue::Time(value) if value == at.time()));
    assert!(matches!(
        RullstValue::from(Some(at)),
        RullstValue::DateTime(_)
    ));
    assert!(matches!(
        RullstValue::from(None::<chrono::NaiveDate>),
        RullstValue::Null
    ));
}

/// The `Any` pool binds the ISO-8601 text SQLx writes to SQLite.
#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
#[test]
fn the_any_pool_binds_dates_as_iso_text() {
    let at = instant();
    assert_eq!(
        super::temporal::date_time(&at),
        "2026-10-15T09:30:00.500+00:00"
    );
    assert_eq!(
        super::temporal::naive_date_time(&at.naive_utc()),
        "2026-10-15 09:30:00.500"
    );
    assert_eq!(super::temporal::date(&at.date_naive()), "2026-10-15");
    assert_eq!(super::temporal::time(&at.time()), "09:30:00.500");
}

/// Strict pools bind SQLx's own `chrono` codecs; a UTC instant goes through
/// `__bind_field::wire` (its UTC wall time on MySQL/MariaDB).
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
#[test]
fn strict_pools_bind_dates_natively() {
    let at = instant();
    assert_eq!(
        super::temporal::date_time(&at),
        crate::__bind_field::wire(at)
    );
    assert_eq!(
        super::temporal::naive_date_time(&at.naive_utc()),
        at.naive_utc()
    );
    assert_eq!(super::temporal::date(&at.date_naive()), at.date_naive());
    assert_eq!(super::temporal::time(&at.time()), at.time());
}
