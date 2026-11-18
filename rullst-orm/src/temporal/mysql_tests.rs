//! MySQL/MariaDB goldens; `golden_tests.rs` checks the encodings, type
//! descriptors and binary readings against the corpus recorded from SQLx's
//! `chrono` codecs.

use proptest::prelude::*;
use sqlx::mysql::types::{MySqlTime, MySqlTimeSign};
use sqlx::{Encode, MySql};

use super::{Date, DateTime, Time, Timestamp, date_time_from_binary};

fn encoded<'q, T: Encode<'q, MySql>>(value: T) -> (Vec<u8>, usize) {
    let mut buffer = Vec::new();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    (buffer, value.size_hint())
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
    // The length byte decides: 7 reads no microseconds even if bytes follow,
    // 11 reads up to 8 microsecond bytes.
    assert_eq!(
        read(&[7, 0xea, 0x07, 10, 15, 9, 30, 0, 1, 0, 0, 0]).unwrap(),
        "2026-10-15T09:30:00"
    );
    assert_eq!(
        read(&[11, 0xea, 0x07, 10, 15, 9, 30, 0, 1, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        "2026-10-15T09:30:00.000001"
    );
    assert!(read(&[11, 0xea, 0x07, 10, 15, 9, 30, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    // The zero date and short buffers are errors, never panics.
    assert!(read(&[0]).is_err());
    assert!(read(&[]).is_err());
    assert!(read(&[7, 0xea, 0x07, 10]).is_err());
    assert!(read(&[7, 0xea, 0x07, 10, 15, 9]).is_err());
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
    // chrono's text parser allows whitespace before a number; a space is
    // never a binary length.
    assert_eq!(
        date_time_from_value(b" 2026-10-15 09:30:00")
            .unwrap()
            .to_string(),
        "2026-10-15T09:30:00"
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
    // An instant no `Timestamp` holds names the range and the ways out.
    for bytes in [
        &b"9999-12-31 23:59:59"[..],
        &[7, 0x0f, 0x27, 12, 31, 23, 59, 59][..],
    ] {
        let message = instant_from_value(bytes).unwrap_err().to_string();
        assert!(
            message
                .starts_with("9999-12-31T23:59:59 is outside the range of rullst_orm::Timestamp"),
            "{message}"
        );
        assert!(message.contains("rullst_orm::DateTime"), "{message}");
    }
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
