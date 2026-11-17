//! MySQL/MariaDB against SQLx's `chrono` codecs: the same binary parameters,
//! type descriptors and compatible column types, and the same binary reads.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use proptest::prelude::*;
use sqlx::mysql::MySqlTypeInfo;
use sqlx::mysql::types::{MySqlTime, MySqlTimeSign};
use sqlx::{Encode, MySql, Type};

use super::super::test_support::{
    chrono_date, chrono_date_time, chrono_instant, chrono_time, date, date_time, instant,
    jiff_date_time, jiff_time, time,
};
use super::{Date, DateTime, Time, Timestamp, date_time_from_binary, time_from_mysql_time};

fn encoded<'q, T: Encode<'q, MySql>>(value: T) -> (Vec<u8>, usize) {
    let mut buffer = Vec::new();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    (buffer, value.size_hint())
}

#[test]
fn type_descriptors_and_compatible_columns_match_chrono() {
    fn same<A: Type<MySql>, B: Type<MySql>>(others: &[MySqlTypeInfo]) {
        assert_eq!(A::type_info(), B::type_info());
        assert_eq!(A::type_info().to_string(), B::type_info().to_string());
        for other in others {
            assert_eq!(A::compatible(other), B::compatible(other), "{other}");
        }
    }
    let others = [
        <chrono::DateTime<Utc> as Type<MySql>>::type_info(),
        <NaiveDateTime as Type<MySql>>::type_info(),
        <NaiveDate as Type<MySql>>::type_info(),
        <NaiveTime as Type<MySql>>::type_info(),
        <String as Type<MySql>>::type_info(),
        <i64 as Type<MySql>>::type_info(),
        <u8 as Type<MySql>>::type_info(),
        <Vec<u8> as Type<MySql>>::type_info(),
    ];
    same::<Timestamp, chrono::DateTime<Utc>>(&others);
    same::<DateTime, NaiveDateTime>(&others);
    same::<Date, NaiveDate>(&others);
    same::<Time, NaiveTime>(&others);
}

#[test]
fn golden_parameters() {
    let at: Timestamp = "2040-02-29T23:59:59.5Z".parse().unwrap();
    assert_eq!(
        encoded(at).0,
        [11, 0xf8, 0x07, 2, 29, 23, 59, 59, 0x20, 0xa1, 0x07, 0x00]
    );
    let midnight: DateTime = "2026-10-15T00:00:00".parse().unwrap();
    assert_eq!(encoded(midnight), (vec![4, 0xea, 0x07, 10, 15], 5));
    let whole: DateTime = "2026-10-15T09:30:00".parse().unwrap();
    assert_eq!(encoded(whole), (vec![7, 0xea, 0x07, 10, 15, 9, 30, 0], 8));
    // A sub-microsecond fraction still sends the (zero) microseconds.
    let tiny: DateTime = "2026-10-15T00:00:00.0000005".parse().unwrap();
    assert_eq!(
        encoded(tiny),
        (vec![11, 0xea, 0x07, 10, 15, 0, 0, 0, 0, 0, 0, 0], 12)
    );
    let day: Date = "2026-10-15".parse().unwrap();
    assert_eq!(encoded(day), (vec![4, 0xea, 0x07, 10, 15], 5));
    let clock: Time = "09:30:00".parse().unwrap();
    assert_eq!(encoded(clock), (vec![8, 0, 0, 0, 0, 0, 9, 30, 0], 9));
    let fine: Time = "09:30:00.000001".parse().unwrap();
    assert_eq!(
        encoded(fine),
        (vec![12, 0, 0, 0, 0, 0, 9, 30, 0, 1, 0, 0, 0], 13)
    );
    // MySQL has no negative years.
    let ancient: Date = "-000001-01-01".parse().unwrap();
    let mut buffer = Vec::new();
    assert!(Encode::<MySql>::encode_by_ref(&ancient, &mut buffer).is_err());
}

/// sqlx-mysql 0.9's binary `NaiveDateTime` decoder, with its panics on short
/// buffers as `None`.
fn chrono_binary(buf: &[u8]) -> Option<NaiveDateTime> {
    let (&len, rest) = buf.split_first()?;
    let date = match rest.len() {
        0 => return None,
        1..=3 => return None,
        _ => NaiveDate::from_ymd_opt(
            i32::from(u16::from_le_bytes([rest[0], rest[1]])),
            u32::from(rest[2]),
            u32::from(rest[3]),
        )?,
    };
    if len <= 4 {
        return date.and_hms_opt(0, 0, 0);
    }
    let time = rest.get(4..)?;
    let (&hour, time) = time.split_first()?;
    let (&minute, time) = time.split_first()?;
    let (&second, time) = time.split_first()?;
    let micros = if len - 4 > 3 {
        if time.len() > 8 {
            return None;
        }
        let mut le = [0; 8];
        le[..time.len()].copy_from_slice(time);
        u32::try_from(u64::from_le_bytes(le)).ok()?
    } else {
        0
    };
    let time = NaiveTime::from_hms_micro_opt(
        u32::from(hour),
        u32::from(minute),
        u32::from(second),
        micros,
    )?;
    Some(date.and_time(time))
}

/// Both write the same bytes (and size hint, when `hint`), or both refuse
/// the value (a negative year).
fn same_parameter<'q, A: Encode<'q, MySql>, B: Encode<'q, MySql>>(ours: A, theirs: B, hint: bool) {
    let mut our_bytes = Vec::new();
    let mut their_bytes = Vec::new();
    let our_result = ours.encode_by_ref(&mut our_bytes).is_ok();
    let their_result = theirs.encode_by_ref(&mut their_bytes).is_ok();
    assert_eq!(our_result, their_result);
    if our_result {
        assert_eq!(our_bytes, their_bytes);
        if hint {
            assert_eq!(ours.size_hint(), theirs.size_hint());
        }
    }
}

#[test]
fn golden_reads() {
    let read = |bytes: &[u8]| date_time_from_binary(bytes).map(|value| value.to_string());
    assert_eq!(
        read(&[4, 0xea, 0x07, 10, 15]).unwrap(),
        "2026-10-15T00:00:00"
    );
    assert_eq!(
        read(&[11, 0xea, 0x07, 10, 15, 9, 30, 0, 1, 0, 0, 0]).unwrap(),
        "2026-10-15T09:30:00.000001"
    );
    // A leap second (1,000,000 microseconds or more at second 59).
    assert_eq!(
        read(&[11, 0xe0, 0x07, 12, 31, 23, 59, 59, 0x60, 0xe3, 0x16, 0x00]).unwrap(),
        "2016-12-31T23:59:59.5"
    );
    // The zero date and short buffers are errors, never panics.
    assert!(read(&[0]).is_err());
    assert!(read(&[]).is_err());
    assert!(read(&[7, 0xea, 0x07, 10]).is_err());
    assert!(read(&[7, 0xea, 0x07, 10, 15, 9]).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn parameters_are_chronos(at in instant(), civil in date_time(), day in date(), clock in time()) {
        same_parameter(at, chrono_instant(at), false);
        same_parameter(civil, chrono_date_time(civil), true);
        same_parameter(day, chrono_date(*day), true);
        same_parameter(clock, chrono_time(*clock), true);
    }

    #[test]
    fn binary_reads_are_chronos(bytes in prop_oneof![
        prop::collection::vec(any::<u8>(), 0..16),
        (prop_oneof![Just(4_u8), Just(7), Just(11)], 0_u16..=10_000, 0_u8..=13, 0_u8..=32,
            0_u8..=24, 0_u8..=60, 0_u8..=60, any::<u32>())
            .prop_map(|(len, year, month, day, hour, minute, second, micros)| {
                let mut bytes = vec![len];
                bytes.extend_from_slice(&year.to_le_bytes());
                bytes.extend_from_slice(&[month, day, hour, minute, second]);
                bytes.extend_from_slice(&(micros % 2_100_000).to_le_bytes());
                bytes.truncate(usize::from(len) + 1);
                bytes
            }),
    ]) {
        prop_assert_eq!(
            date_time_from_binary(&bytes).ok(),
            chrono_binary(&bytes).and_then(jiff_date_time)
        );
    }

    #[test]
    fn times_read_as_chrono_reads_them(
        negative in any::<bool>(),
        hours in 0_u32..=840,
        minutes in 0_u8..=60,
        seconds in 0_u8..=60,
        micros in 0_u32..=1_000_000,
    ) {
        let sign = if negative { MySqlTimeSign::Negative } else { MySqlTimeSign::Positive };
        if let Ok(value) = MySqlTime::new(sign, hours, minutes, seconds, micros) {
            prop_assert_eq!(
                time_from_mysql_time(&value),
                NaiveTime::try_from(value).ok().and_then(jiff_time)
            );
        }
    }
}

/// What the `Decode` implementations receive: the row's bytes, with the
/// binary protocol's length prefix or the text protocol's characters.
#[test]
fn values_in_either_protocol() {
    use super::{
        date_from_value, date_time_from_value, instant_from_value, mysql_time_from_binary,
        time_from_value,
    };

    let binary = [11, 0xea, 0x07, 10, 15, 9, 30, 0, 1, 0, 0, 0];
    assert_eq!(
        date_time_from_value(&binary).unwrap().to_string(),
        "2026-10-15T09:30:00.000001"
    );
    assert_eq!(
        date_time_from_value(b"2026-10-15 09:30:00.000001").unwrap(),
        date_time_from_value(&binary).unwrap()
    );
    assert_eq!(
        instant_from_value(&binary).unwrap().to_string(),
        "2026-10-15T09:30:00.000001Z"
    );
    assert_eq!(
        instant_from_value(b"2040-02-29 23:59:59")
            .unwrap()
            .to_string(),
        "2040-02-29T23:59:59Z"
    );
    assert_eq!(
        date_from_value(&[4, 0xea, 0x07, 10, 15])
            .unwrap()
            .to_string(),
        "2026-10-15"
    );
    assert_eq!(
        date_from_value(b"2026-10-15").unwrap().to_string(),
        "2026-10-15"
    );
    assert_eq!(
        time_from_value(&[12, 0, 0, 0, 0, 0, 9, 30, 0, 1, 0, 0, 0])
            .unwrap()
            .to_string(),
        "09:30:00.000001"
    );
    assert_eq!(time_from_value(&[0]).unwrap().to_string(), "00:00:00");
    // The sign is ignored, as chrono ignored it.
    assert_eq!(
        time_from_value(&[8, 1, 0, 0, 0, 0, 9, 30, 0])
            .unwrap()
            .to_string(),
        "09:30:00"
    );
    assert_eq!(
        time_from_value(b"09:30:00.5").unwrap().to_string(),
        "09:30:00.5"
    );

    // The zero date, wrong lengths, bad text and out-of-range values are
    // errors, never panics.
    for bytes in [
        &[0][..],
        &[4, 0xea, 0x07][..],
        &[][..],
        b"0000-00-00 00:00:00",
        b"10000-01-01 00:00:00",
        &[0xff][..],
    ] {
        assert!(date_time_from_value(bytes).is_err(), "{bytes:?}");
        assert!(instant_from_value(bytes).is_err(), "{bytes:?}");
    }
    assert!(instant_from_value(b"9999-12-31 23:00:00").is_err());
    assert!(date_from_value(&[0]).is_err());
    assert!(date_from_value(b"0000-00-00").is_err());
    for bytes in [
        &[8, 0, 0, 0, 0, 0, 9, 30][..],
        &[8, 2, 0, 0, 0, 0, 9, 30, 0][..],
        &[8, 0, 1, 0, 0, 0, 0, 0, 0][..],
        &[8, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0][..],
        &[12, 0, 0, 0, 0, 0, 9, 30, 0, 0x40, 0x42, 0x0f, 0][..],
        &[8, 1, 0, 0, 0, 0, 0, 0, 0][..],
        b"25:00:00",
        b"-01:00:00",
    ] {
        assert!(time_from_value(bytes).is_err(), "{bytes:?}");
    }
    assert!(mysql_time_from_binary(&[]).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// The binary `TIME` reader returns what `MySqlTime` encodes.
    #[test]
    fn binary_times_read_as_mysql_time_writes_them(
        negative in any::<bool>(),
        hours in 0_u32..=838,
        minutes in 0_u8..=59,
        seconds in 0_u8..=59,
        micros in prop_oneof![Just(0_u32), 0_u32..=999_999],
    ) {
        use super::mysql_time_from_binary;

        let sign = if negative { MySqlTimeSign::Negative } else { MySqlTimeSign::Positive };
        if let Ok(value) = MySqlTime::new(sign, hours, minutes, seconds, micros) {
            let mut bytes = Vec::new();
            let _ = Encode::<MySql>::encode_by_ref(&value, &mut bytes).unwrap();
            prop_assert_eq!(mysql_time_from_binary(&bytes).unwrap(), value);
        }
    }
}
