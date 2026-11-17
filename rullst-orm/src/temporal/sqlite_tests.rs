//! SQLite against SQLx's `chrono` codecs: the same argument values, type
//! descriptors and compatible column types, and the same reads of every
//! storage class through a real in-memory database.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use proptest::prelude::*;
use sqlx::sqlite::{SqliteArgumentsBuffer, SqliteConnectOptions, SqliteTypeInfo};
use sqlx::{ConnectOptions, Encode, Row, Sqlite, SqliteConnection, Type};

use super::super::test_support::{
    chrono_date_time, chrono_instant, date_time, instant, jiff_date, jiff_date_time, jiff_instant,
    jiff_time,
};
use super::{Date, DateTime, Time, Timestamp, from_julian_day, from_unix_seconds};

fn encoded<'q, T: Encode<'q, Sqlite>>(value: T) -> String {
    let mut buffer = SqliteArgumentsBuffer::default();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    format!("{buffer:?}")
}

/// Every SQLite type descriptor SQLx produces.
fn type_infos() -> Vec<SqliteTypeInfo> {
    vec![
        <str as Type<Sqlite>>::type_info(),
        <i64 as Type<Sqlite>>::type_info(),
        <i32 as Type<Sqlite>>::type_info(),
        <f64 as Type<Sqlite>>::type_info(),
        <bool as Type<Sqlite>>::type_info(),
        <Vec<u8> as Type<Sqlite>>::type_info(),
        <NaiveDateTime as Type<Sqlite>>::type_info(),
        <NaiveDate as Type<Sqlite>>::type_info(),
        <NaiveTime as Type<Sqlite>>::type_info(),
    ]
}

#[test]
fn type_descriptors_and_compatible_columns_match_chrono() {
    assert_eq!(
        <Timestamp as Type<Sqlite>>::type_info(),
        <chrono::DateTime<Utc> as Type<Sqlite>>::type_info()
    );
    assert_eq!(
        <DateTime as Type<Sqlite>>::type_info(),
        <NaiveDateTime as Type<Sqlite>>::type_info()
    );
    assert_eq!(
        <Date as Type<Sqlite>>::type_info(),
        <NaiveDate as Type<Sqlite>>::type_info()
    );
    assert_eq!(
        <Time as Type<Sqlite>>::type_info(),
        <NaiveTime as Type<Sqlite>>::type_info()
    );
    for type_info in type_infos() {
        let name = type_info.to_string();
        assert_eq!(
            <Timestamp as Type<Sqlite>>::compatible(&type_info),
            <chrono::DateTime<Utc> as Type<Sqlite>>::compatible(&type_info),
            "{name}"
        );
        assert_eq!(
            <DateTime as Type<Sqlite>>::compatible(&type_info),
            <NaiveDateTime as Type<Sqlite>>::compatible(&type_info),
            "{name}"
        );
        assert_eq!(
            <Date as Type<Sqlite>>::compatible(&type_info),
            <NaiveDate as Type<Sqlite>>::compatible(&type_info),
            "{name}"
        );
        assert_eq!(
            <Time as Type<Sqlite>>::compatible(&type_info),
            <NaiveTime as Type<Sqlite>>::compatible(&type_info),
            "{name}"
        );
    }
}

#[test]
fn golden_text() {
    let at: Timestamp = "2026-10-15T09:30:00.5Z".parse().unwrap();
    assert_eq!(
        encoded(at),
        r#"SqliteArgumentsBuffer([Text("2026-10-15T09:30:00.500+00:00")])"#
    );
    let civil: DateTime = "2026-10-15T09:30:00.000001".parse().unwrap();
    assert_eq!(
        encoded(civil),
        r#"SqliteArgumentsBuffer([Text("2026-10-15 09:30:00.000001")])"#
    );
    let date: Date = "-000044-03-15".parse().unwrap();
    assert_eq!(
        encoded(date),
        r#"SqliteArgumentsBuffer([Text("-0044-03-15")])"#
    );
    let time: Time = "23:59:59.000000007".parse().unwrap();
    assert_eq!(
        encoded(time),
        r#"SqliteArgumentsBuffer([Text("23:59:59.000000007")])"#
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn writes_are_chronos(at in instant(), civil in date_time()) {
        prop_assert_eq!(encoded(at), encoded(chrono_instant(at)));
        let naive = chrono_date_time(civil);
        prop_assert_eq!(encoded(civil), encoded(naive));
        prop_assert_eq!(encoded(Date::from(civil.date())), encoded(naive.date()));
        prop_assert_eq!(encoded(Time::from(civil.time())), encoded(naive.time()));
    }

    #[test]
    fn integers_read_as_unix_seconds(seconds in any::<i64>()) {
        let theirs = chrono::DateTime::<Utc>::from_timestamp(seconds, 0);
        let ours = from_unix_seconds(seconds).and_then(|moment| moment.instant());
        prop_assert_eq!(ours, theirs.and_then(jiff_instant));
    }

    #[test]
    fn reals_read_as_julian_days(day in prop_oneof![
        -1.0e7_f64..1.0e7,
        2_440_587.5_f64..2_500_000.0,
        any::<f64>(),
    ]) {
        // sqlx-sqlite 0.9 `decode_datetime_from_float`.
        let seconds = (day - 2_440_587.5) * 86_400.0;
        let theirs = seconds.is_finite().then(|| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (whole, nanos) = (seconds.trunc() as i64, (seconds.fract() * 1E9).abs() as u32);
            chrono::DateTime::<Utc>::from_timestamp(whole, nanos)
        }).flatten();
        let ours = from_julian_day(day);
        prop_assert_eq!(
            ours.as_ref().and_then(|moment| moment.instant()),
            theirs.and_then(jiff_instant)
        );
        prop_assert_eq!(
            ours.map(|moment| moment.local),
            theirs.and_then(|at| jiff_date_time(at.naive_utc()))
        );
    }
}

/// Decodes `SELECT ?` with the value bound as `sql` (a literal), through both.
async fn read_both(connection: &mut SqliteConnection, sql: &str) {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {sql}")))
        .fetch_one(&mut *connection)
        .await
        .expect("select");
    let instant = row.try_get::<Timestamp, _>(0).ok().map(|at| *at);
    let chrono_instant = row.try_get::<chrono::DateTime<Utc>, _>(0).ok();
    assert_eq!(instant, chrono_instant.and_then(jiff_instant), "{sql}");
    let civil = row.try_get::<DateTime, _>(0).ok().map(|at| *at);
    let naive = row.try_get::<NaiveDateTime, _>(0).ok();
    assert_eq!(civil, naive.and_then(jiff_date_time), "{sql}");
    let date = row.try_get::<Date, _>(0).ok().map(|at| *at);
    let naive_date = row.try_get::<NaiveDate, _>(0).ok();
    assert_eq!(date, naive_date.and_then(jiff_date), "{sql}");
    let time = row.try_get::<Time, _>(0).ok().map(|at| *at);
    let naive_time = row.try_get::<NaiveTime, _>(0).ok();
    assert_eq!(time, naive_time.and_then(jiff_time), "{sql}");
}

#[tokio::test]
async fn every_storage_class_reads_as_chrono_reads_it() {
    let mut connection = SqliteConnectOptions::new()
        .in_memory(true)
        .connect()
        .await
        .expect("in-memory SQLite");
    for sql in [
        "'2026-10-15T09:30:00.123456+00:00'",
        "'2026-10-15 12:30:00+03:00'",
        "'2026-10-15 09:30'",
        "'2026-10-15'",
        "'09:30:00.5'",
        "'2016-12-31T23:59:60Z'",
        "'not a date'",
        "1792056600",
        "-62135596800",
        "253402300800",
        "2461328.89583333",
        "0.5",
        "x'00'",
        "NULL",
    ] {
        read_both(&mut connection, sql).await;
    }
    // Out of jiff's range: chrono reads these, ours report a decode error.
    for sql in [
        "'+10000-01-01 00:00'",
        "253402300800",
        "'-10000-12-31T00:00:00Z'",
    ] {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {sql}")))
            .fetch_one(&mut connection)
            .await
            .expect("select");
        assert!(row.try_get::<chrono::DateTime<Utc>, _>(0).is_ok(), "{sql}");
        assert!(row.try_get::<Timestamp, _>(0).is_err(), "{sql}");
        assert!(row.try_get::<DateTime, _>(0).is_err(), "{sql}");
    }
    let row = sqlx::query("SELECT '+10000-01-01', '2016-12-31T23:59:60.5Z'")
        .fetch_one(&mut connection)
        .await
        .expect("select");
    assert!(row.try_get::<NaiveDate, _>(0).is_ok());
    assert!(row.try_get::<Date, _>(0).is_err());
    // A leap second reads as second 59.
    assert_eq!(
        row.try_get::<Timestamp, _>(1)
            .expect("leap second")
            .to_string(),
        "2016-12-31 23:59:59.500 UTC"
    );

    let row = sqlx::query("SELECT CAST(? AS TEXT)")
        .bind(Timestamp::from(
            jiff::Timestamp::from_second(1_792_056_600).unwrap(),
        ))
        .fetch_one(&mut connection)
        .await
        .expect("bind");
    assert_eq!(row.get::<String, _>(0), "2026-10-15T09:30:00+00:00");
}

/// Integers and reals near the ends of the civil range read as civil values
/// even where no `Timestamp` exists, as chrono's `NaiveDateTime` read them.
#[test]
fn integers_and_reals_reach_the_civil_edges() {
    for (moment, local) in [
        (from_unix_seconds(253_402_300_799), "9999-12-31T23:59:59"),
        (
            from_unix_seconds(-377_705_116_800),
            "-009999-01-01T00:00:00",
        ),
        (from_julian_day(5_373_484.0), "9999-12-31T12:00:00"),
    ] {
        let moment = moment.expect("a civil value");
        assert_eq!(moment.local.to_string(), local);
        assert_eq!(moment.instant(), None, "{local} is no Timestamp");
    }
    assert!(from_unix_seconds(253_402_300_800).is_none());
    assert!(from_unix_seconds(-377_705_116_801).is_none());
    assert_eq!(
        from_unix_seconds(253_402_207_200)
            .and_then(|moment| moment.instant())
            .map(|at| at.to_string())
            .as_deref(),
        Some("9999-12-30T22:00:00Z")
    );
    assert_eq!(
        from_julian_day(2_440_587.5)
            .and_then(|moment| moment.instant())
            .map(|at| at.to_string())
            .as_deref(),
        Some("1970-01-01T00:00:00Z")
    );
}
