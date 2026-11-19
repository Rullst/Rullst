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
        RullstValue::Timestamp("2026-10-15T09:30:00.5Z".parse().unwrap()),
        RullstValue::CivilDateTime("2026-10-15T09:30:00.5".parse().unwrap()),
        RullstValue::CivilDate("2026-10-15".parse().unwrap()),
        RullstValue::CivilTime("09:30:00.5".parse().unwrap()),
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

/// Strict pools bind each type with its own codec; a UTC instant goes through
/// `__bind_field::timestamp_wire` (its UTC wall time on MySQL/MariaDB).
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
#[test]
fn strict_pools_bind_dates_natively() {
    let at: crate::Timestamp = "2026-10-15T09:30:00.5Z".parse().unwrap();
    assert_eq!(
        format!("{:?}", super::temporal::timestamp(at)),
        format!("{:?}", crate::__bind_field::timestamp_wire(at))
    );
    let civil: crate::DateTime = "2026-10-15T09:30:00.5".parse().unwrap();
    assert_eq!(super::temporal::civil_date_time(civil), civil);
    let day: crate::Date = "2026-10-15".parse().unwrap();
    assert_eq!(super::temporal::civil_date(day), day);
    let clock: crate::Time = "09:30:00.5".parse().unwrap();
    assert_eq!(super::temporal::civil_time(clock), clock);
}

/// The parameters a query receives for one value, in a form tests compare.
fn bound(value: &RullstValue) -> String {
    use sqlx::Execute;

    let mut query = sqlx::query::<crate::RullstDatabase>(sqlx::AssertSqlSafe("SELECT 1"))
        .bind_rullst_value(value);
    // MySQL/MariaDB refuse a negative year, whichever type holds it.
    let Ok(arguments) = query.take_arguments() else {
        return "refused".to_string();
    };
    let arguments = arguments.unwrap();
    #[cfg(any(
        feature = "strict-postgres",
        feature = "strict-mysql",
        feature = "strict-sqlite"
    ))]
    return format!("{arguments:?}");
    #[cfg(not(any(
        feature = "strict-postgres",
        feature = "strict-mysql",
        feature = "strict-sqlite"
    )))]
    return format!("{:?}", arguments.values.0);
}

/// On a strict pool each variant binds its type's own parameter (the
/// golden corpora check those encodings against SQLx's `chrono` codecs).
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
mod same_parameters_as_the_codecs {
    use proptest::prelude::*;

    use super::bound;
    use crate::RullstValue;
    use crate::temporal::test_support::{date, date_time, instant, time};

    fn bound_directly<T>(value: T) -> String
    where
        T: for<'q> sqlx::Encode<'q, crate::RullstDatabase>
            + sqlx::Type<crate::RullstDatabase>
            + Send
            + 'static,
    {
        use sqlx::Execute;
        let mut query =
            sqlx::query::<crate::RullstDatabase>(sqlx::AssertSqlSafe("SELECT 1")).bind(value);
        match query.take_arguments() {
            Ok(arguments) => format!("{:?}", arguments.unwrap()),
            Err(_) => "refused".to_string(),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn for_every_value(at in instant(), civil in date_time(), day in date(), clock in time()) {
            prop_assert_eq!(
                bound(&RullstValue::Timestamp(at)),
                bound_directly(crate::__bind_field::timestamp_wire(at))
            );
            prop_assert_eq!(bound(&RullstValue::CivilDateTime(civil)), bound_directly(civil));
            prop_assert_eq!(bound(&RullstValue::CivilDate(day)), bound_directly(day));
            prop_assert_eq!(bound(&RullstValue::CivilTime(clock)), bound_directly(clock));
        }
    }

    #[test]
    fn and_never_the_null_parameter() {
        let at: crate::Timestamp = "2026-10-15T09:30:00.5Z".parse().unwrap();
        assert_ne!(
            bound(&RullstValue::Timestamp(at)),
            bound(&RullstValue::Null)
        );
        let day: crate::Date = "2026-10-15".parse().unwrap();
        assert_ne!(
            bound(&RullstValue::CivilDate(day)),
            bound(&RullstValue::Null)
        );
    }
}

/// The `Any` pool binds the same ISO-8601 text for the jiff-based variants.
#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
#[test]
fn the_any_pool_binds_jiff_dates_as_the_same_iso_text() {
    let at: crate::Timestamp = "2026-10-15T09:30:00.5Z".parse().unwrap();
    assert_eq!(
        bound(&RullstValue::Timestamp(at)),
        r#"[Text("2026-10-15T09:30:00.500+00:00")]"#
    );
    let civil: crate::DateTime = "2026-10-15T09:30:00.5".parse().unwrap();
    assert_eq!(
        bound(&RullstValue::CivilDateTime(civil)),
        r#"[Text("2026-10-15 09:30:00.500")]"#
    );
    let day: crate::Date = "2026-10-15".parse().unwrap();
    assert_eq!(
        bound(&RullstValue::CivilDate(day)),
        r#"[Text("2026-10-15")]"#
    );
    let clock: crate::Time = "09:30:00.5".parse().unwrap();
    assert_eq!(
        bound(&RullstValue::CivilTime(clock)),
        r#"[Text("09:30:00.500")]"#
    );
}
