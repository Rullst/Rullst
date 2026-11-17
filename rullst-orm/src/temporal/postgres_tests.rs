//! PostgreSQL against SQLx's `chrono` codecs: the same binary parameters,
//! type descriptors (OIDs) and compatible types, and the same binary reads
//! (text reads are covered by the parser's tests).

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use proptest::prelude::*;
use sqlx::postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo};
use sqlx::{Encode, Postgres, Type};

use super::super::test_support::{
    chrono_date, chrono_date_time, chrono_instant, chrono_time, date, date_time, instant,
    jiff_date, jiff_date_time, jiff_instant, jiff_time, time,
};
use super::{
    Date, DateTime, Time, Timestamp, date_from_days, date_time_from_micros, date_time_from_text,
    instant_from_micros, instant_from_text, time_from_micros,
};

fn encoded<'q, T: Encode<'q, Postgres>>(value: T) -> (Vec<u8>, usize) {
    let mut buffer = PgArgumentBuffer::default();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    (buffer.to_vec(), value.size_hint())
}

fn epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

#[test]
fn type_descriptors_and_compatible_types_match_chrono() {
    fn same<A: Type<Postgres> + PgHasArrayType, B: Type<Postgres> + PgHasArrayType>(
        others: &[PgTypeInfo],
    ) {
        assert_eq!(A::type_info(), B::type_info());
        assert_eq!(A::array_type_info(), B::array_type_info());
        for other in others {
            assert_eq!(A::compatible(other), B::compatible(other), "{other:?}");
        }
    }
    let others = [
        <chrono::DateTime<Utc> as Type<Postgres>>::type_info(),
        <NaiveDateTime as Type<Postgres>>::type_info(),
        <NaiveDate as Type<Postgres>>::type_info(),
        <NaiveTime as Type<Postgres>>::type_info(),
        <String as Type<Postgres>>::type_info(),
        <i64 as Type<Postgres>>::type_info(),
        <i32 as Type<Postgres>>::type_info(),
    ];
    same::<Timestamp, chrono::DateTime<Utc>>(&others);
    same::<DateTime, NaiveDateTime>(&others);
    same::<Date, NaiveDate>(&others);
    same::<Time, NaiveTime>(&others);
}

#[test]
fn golden_parameters() {
    // 2000-01-01T00:00:00.000001Z is one microsecond.
    let at: Timestamp = "2000-01-01T00:00:00.000001Z".parse().unwrap();
    assert_eq!(encoded(at), (1_i64.to_be_bytes().to_vec(), 8));
    // Before 2000 a sub-microsecond part is truncated toward 2000.
    let before: Timestamp = "1999-12-31T23:59:59.9999995Z".parse().unwrap();
    assert_eq!(encoded(before), (0_i64.to_be_bytes().to_vec(), 8));
    let civil: DateTime = "1999-12-31T23:59:59.999998".parse().unwrap();
    assert_eq!(encoded(civil), ((-2_i64).to_be_bytes().to_vec(), 8));
    let day: Date = "1999-12-31".parse().unwrap();
    assert_eq!(encoded(day), ((-1_i32).to_be_bytes().to_vec(), 4));
    let clock: Time = "00:00:01.0000019".parse().unwrap();
    assert_eq!(encoded(clock), (1_000_001_i64.to_be_bytes().to_vec(), 8));
}

#[test]
fn golden_reads() {
    assert_eq!(
        instant_from_micros(-1).unwrap().to_string(),
        "1999-12-31T23:59:59.999999Z"
    );
    assert_eq!(instant_from_micros(i64::MAX), None, "PostgreSQL `infinity`");
    assert_eq!(
        date_time_from_micros(86_400_000_000).unwrap().to_string(),
        "2000-01-02T00:00:00"
    );
    assert_eq!(date_time_from_micros(i64::MIN), None, "`-infinity`");
    assert_eq!(date_from_days(-1).unwrap().to_string(), "1999-12-31");
    assert_eq!(date_from_days(i32::MAX), None);
    // `24:00:00` and negative values wrap into one day, as chrono added them.
    assert_eq!(time_from_micros(86_400_000_000).to_string(), "00:00:00");
    assert_eq!(time_from_micros(-1).to_string(), "23:59:59.999999");
    assert_eq!(
        instant_from_text("2026-10-15 09:30:00+00")
            .unwrap()
            .to_string(),
        "2026-10-15T09:30:00Z"
    );
    assert_eq!(
        date_time_from_text("2026-10-15 09:30:00+05")
            .unwrap()
            .to_string(),
        "2026-10-15T09:30:00",
        "the offset of a `+` text is dropped"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn parameters_are_chronos(at in instant(), civil in date_time(), day in date(), clock in time()) {
        prop_assert_eq!(encoded(at), encoded(chrono_instant(at)));
        prop_assert_eq!(encoded(civil), encoded(chrono_date_time(civil)));
        prop_assert_eq!(encoded(day), encoded(chrono_date(*day)));
        prop_assert_eq!(encoded(clock), encoded(chrono_time(*clock)));
    }

    #[test]
    fn binary_reads_are_chronos(
        micros in prop_oneof![
            any::<i64>(),
            -400_000_000_000_000_000_i64..400_000_000_000_000_000,
            -1_000_000_i64..1_000_000,
        ],
        days in any::<i32>(),
    ) {
        let theirs = epoch().checked_add_signed(Duration::microseconds(micros));
        prop_assert_eq!(date_time_from_micros(micros), theirs.and_then(jiff_date_time));
        prop_assert_eq!(
            instant_from_micros(micros),
            theirs.and_then(|at| jiff_instant(at.and_utc()))
        );
        let date = epoch().date().checked_add_signed(Duration::days(i64::from(days)));
        prop_assert_eq!(date_from_days(days), date.and_then(jiff_date));
        let clock = NaiveTime::default().overflowing_add_signed(Duration::microseconds(micros)).0;
        prop_assert_eq!(Some(time_from_micros(micros)), jiff_time(clock));
    }

    /// What our encoder writes, chrono's decoder reads as the same value
    /// (truncated to the microsecond), and the reverse.
    #[test]
    fn round_trips_through_chrono(at in instant()) {
        let (bytes, _) = encoded(at);
        let micros = i64::from_be_bytes(bytes.try_into().unwrap());
        let theirs = epoch().checked_add_signed(Duration::microseconds(micros)).unwrap();
        prop_assert_eq!(instant_from_micros(micros), jiff_instant(theirs.and_utc()));
    }
}

/// What the `Decode` implementations receive: the format and the bytes.
#[test]
fn values_in_either_format() {
    use sqlx::postgres::PgValueFormat::{Binary, Text};

    use super::{date_from_value, date_time_from_value, instant_from_value, time_from_value};

    let instant = instant_from_value(Binary, &1_000_000_i64.to_be_bytes()).unwrap();
    assert_eq!(instant.to_string(), "2000-01-01T00:00:01Z");
    let instant = instant_from_value(Text, b"2000-01-01 00:00:01.5+00").unwrap();
    assert_eq!(instant.to_string(), "2000-01-01T00:00:01.5Z");
    let civil = date_time_from_value(Binary, &(-1_i64).to_be_bytes()).unwrap();
    assert_eq!(civil.to_string(), "1999-12-31T23:59:59.999999");
    let civil = date_time_from_value(Text, b"1999-12-31 23:59:59").unwrap();
    assert_eq!(civil.to_string(), "1999-12-31T23:59:59");
    let date = date_from_value(Binary, &366_i32.to_be_bytes()).unwrap();
    assert_eq!(date.to_string(), "2001-01-01");
    assert_eq!(date_from_value(Text, b"2001-01-01").unwrap(), date);
    let time = time_from_value(Binary, &3_600_000_001_i64.to_be_bytes()).unwrap();
    assert_eq!(time.to_string(), "01:00:00.000001");
    assert_eq!(time_from_value(Text, b"01:00:00.000001").unwrap(), time);

    // Wrong lengths, bad text, out-of-range values and `infinity` are errors.
    assert!(instant_from_value(Binary, &[0; 4]).is_err());
    assert!(instant_from_value(Binary, &i64::MAX.to_be_bytes()).is_err());
    assert!(instant_from_value(Text, b"infinity").is_err());
    assert!(instant_from_value(Text, &[0xff]).is_err());
    assert!(date_time_from_value(Binary, &[0; 9]).is_err());
    assert!(date_time_from_value(Binary, &i64::MIN.to_be_bytes()).is_err());
    assert!(date_time_from_value(Text, b"10000-01-01 00:00:00").is_err());
    assert!(date_from_value(Binary, &[0; 8]).is_err());
    assert!(date_from_value(Binary, &i32::MAX.to_be_bytes()).is_err());
    assert!(date_from_value(Text, b"2001-02-29").is_err());
    assert!(time_from_value(Binary, &[0; 4]).is_err());
    assert!(time_from_value(Text, b"25:00:00").is_err());
}
