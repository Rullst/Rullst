//! SQLite goldens; `golden_tests.rs` checks the encodings, type descriptors
//! and storage-class readings against the corpus recorded from SQLx's
//! `chrono` codecs.

use sqlx::sqlite::{SqliteArgumentsBuffer, SqliteConnectOptions};
use sqlx::{ConnectOptions, Encode, Row, Sqlite};

use super::{Date, DateTime, Time, Timestamp, from_julian_day, from_unix_seconds};

fn encoded<'q, T: Encode<'q, Sqlite>>(value: T) -> String {
    let mut buffer = SqliteArgumentsBuffer::default();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    format!("{buffer:?}")
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

#[tokio::test]
async fn out_of_range_values_and_leap_seconds_read_as_documented() {
    let mut connection = SqliteConnectOptions::new()
        .in_memory(true)
        .connect()
        .await
        .expect("in-memory SQLite");
    // Out of jiff's range (chrono read these): a decode error.
    for sql in [
        "'+10000-01-01 00:00'",
        "253402300800",
        "'-10000-12-31T00:00:00Z'",
    ] {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {sql}")))
            .fetch_one(&mut connection)
            .await
            .expect("select");
        assert!(row.try_get::<Timestamp, _>(0).is_err(), "{sql}");
        assert!(row.try_get::<DateTime, _>(0).is_err(), "{sql}");
    }
    // The end-of-time sentinel names the range and the ways out.
    let row = sqlx::query("SELECT '9999-12-31 23:59:59'")
        .fetch_one(&mut connection)
        .await
        .expect("select");
    let message = row.try_get::<Timestamp, _>(0).unwrap_err().to_string();
    assert!(
        message.contains("9999-12-31T23:59:59 is outside the range of rullst_orm::Timestamp"),
        "{message}"
    );
    assert!(message.contains("Option<Timestamp>"), "{message}");
    let row = sqlx::query("SELECT '+10000-01-01', '2016-12-31T23:59:60.5Z'")
        .fetch_one(&mut connection)
        .await
        .expect("select");
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
