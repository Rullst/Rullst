//! The parser against chrono itself: every generated text is read by both, and
//! they must agree on acceptance and value (within jiff's range; a leap second
//! is second 59). The `chrono_*` references are SQLx 0.9's `chrono` decoders,
//! copied.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use chrono::{Offset as _, TimeZone, Utc};
use proptest::prelude::*;

use super::super::test_support::{
    date_time_text, jiff_date, jiff_date_time, jiff_instant, jiff_time, time_text,
};
use super::{DATE_ONLY, DATE_TIME, DATE_TIME_OFFSET, TIME, parse, sqlite_date_time, sqlite_time};

/// sqlx-sqlite 0.9 `decode_datetime_from_text`.
fn chrono_sqlite_date_time(value: &str) -> Option<DateTime<FixedOffset>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt);
    }
    let formats = [
        "%F %T%.f",
        "%F %R",
        "%F %RZ",
        "%F %R%:z",
        "%F %T%.fZ",
        "%F %T%.f%:z",
        "%FT%R",
        "%FT%RZ",
        "%FT%R%:z",
        "%FT%T%.f",
        "%FT%T%.fZ",
        "%FT%T%.f%:z",
    ];
    for format in formats {
        if let Ok(dt) = DateTime::parse_from_str(value, format) {
            return Some(dt);
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
            return Some(Utc.fix().from_utc_datetime(&dt));
        }
    }
    None
}

/// sqlx-sqlite 0.9 `NaiveTime` decoder.
fn chrono_sqlite_time(value: &str) -> Option<NaiveTime> {
    ["%T.f", "%T%.f", "%R", "%RZ", "%T%.fZ", "%R%:z", "%T%.f%:z"]
        .into_iter()
        .find_map(|format| NaiveTime::parse_from_str(value, format).ok())
}

/// Both readers agree: ours accepts exactly what chrono accepts within jiff's
/// range, with the same local time and instant.
fn assert_same_date_time(text: &str) {
    let ours = sqlite_date_time(text);
    let theirs = chrono_sqlite_date_time(text);
    let expected_local = theirs.and_then(|dt| jiff_date_time(dt.naive_local()));
    assert_eq!(
        ours.as_ref().map(|moment| moment.local),
        expected_local,
        "{text:?}: chrono read {theirs:?}"
    );
    let expected_instant = theirs.and_then(|dt| jiff_instant(dt.with_timezone(&Utc)));
    assert_eq!(
        ours.as_ref().and_then(|moment| moment.instant()),
        expected_instant,
        "{text:?}: chrono read {theirs:?}"
    );
}

fn assert_same_time(text: &str) {
    assert_eq!(
        sqlite_time(text),
        chrono_sqlite_time(text).and_then(jiff_time),
        "{text:?}"
    );
}

fn chrono_naive(text: &str, format: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text, format).ok()
}

/// PostgreSQL/MySQL text results: `%Y-%m-%d %H:%M:%S%.f`, with `%#z` on
/// PostgreSQL when the text has an offset.
fn assert_same_server_text(text: &str) {
    let plain = parse(text, DATE_TIME).and_then(|parsed| parsed.date_time());
    assert_eq!(
        plain,
        chrono_naive(text, "%Y-%m-%d %H:%M:%S%.f").and_then(jiff_date_time),
        "{text:?}"
    );
    let with_offset = parse(text, DATE_TIME_OFFSET);
    assert_eq!(
        with_offset.as_ref().and_then(|parsed| parsed.date_time()),
        chrono_naive(text, "%Y-%m-%d %H:%M:%S%.f%#z").and_then(jiff_date_time),
        "{text:?}"
    );
    let chrono_instant = DateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.f%#z").ok();
    let ours = with_offset.and_then(|parsed| {
        let offset = parsed.valid_offset()?;
        super::Moment {
            local: parsed.date_time()?,
            offset,
        }
        .instant()
    });
    assert_eq!(
        ours,
        chrono_instant.and_then(|dt| jiff_instant(dt.with_timezone(&Utc))),
        "{text:?}"
    );
    assert_eq!(
        parse(text, DATE_ONLY).and_then(|parsed| parsed.date()),
        NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .ok()
            .and_then(jiff_date),
        "{text:?}"
    );
    assert_eq!(
        parse(text, TIME).and_then(|parsed| parsed.time()),
        NaiveTime::parse_from_str(text, "%H:%M:%S%.f")
            .ok()
            .and_then(jiff_time),
        "{text:?}"
    );
}

/// Golden examples: what both read, and how.
#[test]
fn sqlite_text_golden_values() {
    let read = |text: &str| {
        let moment = sqlite_date_time(text).expect(text);
        (
            moment.local.to_string(),
            moment.instant().map(|at| at.to_string()),
        )
    };
    assert_eq!(
        read("2026-10-15T09:30:00.123456+00:00"),
        (
            "2026-10-15T09:30:00.123456".to_string(),
            Some("2026-10-15T09:30:00.123456Z".to_string())
        )
    );
    // An offset is applied to the instant and kept in the local time.
    assert_eq!(
        read("2026-10-15 12:30+03:00"),
        (
            "2026-10-15T12:30:00".to_string(),
            Some("2026-10-15T09:30:00Z".to_string())
        )
    );
    // An offset of 24 hours or more is ignored: the text reads as UTC.
    assert_eq!(
        read("2026-10-15 12:30+25:00"),
        (
            "2026-10-15T12:30:00".to_string(),
            Some("2026-10-15T12:30:00Z".to_string())
        )
    );
    // The limit is under 24 hours: `+23:59` applies, `+24:00` does not (in
    // RFC 3339 nor in the other forms).
    assert_eq!(
        read("2026-10-15 23:59-23:59"),
        (
            "2026-10-15T23:59:00".to_string(),
            Some("2026-10-16T23:58:00Z".to_string())
        )
    );
    assert_eq!(
        read("2026-10-15T12:30:00+24:00").1.unwrap(),
        "2026-10-15T12:30:00Z"
    );
    assert_eq!(
        read("2026-10-15T12:30:00-24:00").1.unwrap(),
        "2026-10-15T12:30:00Z"
    );
    assert_eq!(
        read("2026-10-15T12:30:00+23:59").1.unwrap(),
        "2026-10-14T12:31:00Z"
    );
    // Lowercase separators (RFC 3339), a space before the offset, a minus
    // sign, extra fraction digits and a leap second.
    assert_eq!(
        read("2026-10-15t09:30:00z").1.unwrap(),
        "2026-10-15T09:30:00Z"
    );
    assert_eq!(
        read("2026-10-15T09:30 \u{2212}01:00").1.unwrap(),
        "2026-10-15T10:30:00Z"
    );
    assert_eq!(
        read("2026-10-15 09:30:00.1234567891").0,
        "2026-10-15T09:30:00.123456789"
    );
    assert_eq!(read("2016-12-31T23:59:60.5Z").0, "2016-12-31T23:59:59.5");
    assert_eq!(read("2026-1-5 9:3").0, "2026-01-05T09:03:00");
    assert_eq!(read("-0001-01-01 00:00").0, "-000001-01-01T00:00:00");
    for rejected in [
        "",
        "2026-10-15",
        "2026-02-30 00:00",
        "2026-10-15 24:00",
        "2026-10-15 09:30:61",
        "+10000-01-01 00:00",
        "2026-10-15 09:30 extra",
        "2026-10-15T09:30:00.Z",
    ] {
        assert!(sqlite_date_time(rejected).is_none(), "{rejected:?}");
    }
    assert_eq!(
        sqlite_time("08:15:30.f").unwrap().to_string(),
        "08:15:30",
        "SQLx's `%T.f` format matches a literal `.f`"
    );
    assert_eq!(sqlite_time("08:15+02:00").unwrap().to_string(), "08:15:00");
    assert!(sqlite_time("08").is_none());
}

#[test]
fn server_text_golden_values() {
    let instant = |text: &str| {
        let parsed = parse(text, DATE_TIME_OFFSET)?;
        super::Moment {
            local: parsed.date_time()?,
            offset: parsed.valid_offset()?,
        }
        .instant()
    };
    assert_eq!(
        instant("2026-10-15 09:30:00.5+00").unwrap().to_string(),
        "2026-10-15T09:30:00.5Z"
    );
    assert_eq!(
        instant("2026-10-15 09:30:00-03:30").unwrap().to_string(),
        "2026-10-15T13:00:00Z"
    );
    assert!(instant("2026-10-15 09:30:00").is_none());
    assert!(instant("2026-10-15 09:30:00+24").is_none());
    assert_eq!(
        instant("2026-10-15 09:30:00-23:59").unwrap().to_string(),
        "2026-10-16T09:29:00Z"
    );
    assert!(instant("2026-10-15 09:30:00+0").is_none());
    assert_eq!(
        parse("2026-10-15 09:30:00.000001", DATE_TIME)
            .and_then(|parsed| parsed.date_time())
            .unwrap()
            .to_string(),
        "2026-10-15T09:30:00.000001"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    #[test]
    fn sqlite_date_times_read_as_chrono_reads_them(text in date_time_text()) {
        assert_same_date_time(&text);
    }

    #[test]
    fn server_date_times_read_as_chrono_reads_them(text in date_time_text()) {
        assert_same_server_text(&text);
    }

    #[test]
    fn sqlite_times_read_as_chrono_reads_them(text in time_text()) {
        assert_same_time(&text);
        assert_same_server_text(&text);
    }

    #[test]
    fn arbitrary_text_reads_as_chrono_reads_it(text in "[0-9T:+\\-. Zz\u{2212}tf]{0,40}") {
        assert_same_date_time(&text);
        assert_same_time(&text);
        assert_same_server_text(&text);
    }

    /// Every value chrono's SQLx encoder writes reads back, as itself.
    #[test]
    fn written_text_reads_back(
        seconds in -377_705_116_800_i64..=253_402_300_799,
        nanos in prop_oneof![Just(0_u32), 0_u32..1_000_000_000, (0_u32..1_000).prop_map(|ms| ms * 1_000_000)],
    ) {
        let instant = DateTime::<Utc>::from_timestamp(seconds, nanos).unwrap();
        let rfc3339 = instant.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, false);
        assert_same_date_time(&rfc3339);
        let naive = instant.naive_utc().format("%F %T%.f").to_string();
        assert_same_date_time(&naive);
        let time = instant.time().format("%T%.f").to_string();
        assert_same_time(&time);
        let moment = sqlite_date_time(&rfc3339).unwrap();
        prop_assert_eq!(moment.instant(), jiff_instant(instant));
    }
}
