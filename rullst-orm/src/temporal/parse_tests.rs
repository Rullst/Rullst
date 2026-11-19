//! Golden examples of the parser; `golden_tests.rs` checks it against the
//! corpus recorded from chrono's own parsers.

use super::{DATE_TIME, DATE_TIME_OFFSET, parse, sqlite_date_time, sqlite_time};

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
    // Only the RFC 3339 reader takes a lowercase `t` and `z`, so these test
    // its own range checks.
    assert_eq!(
        read("2016-12-31t23:59:60z").1.unwrap(),
        "2016-12-31T23:59:59Z"
    );
    assert_eq!(
        read("2026-10-15t23:59:59z").1.unwrap(),
        "2026-10-15T23:59:59Z"
    );
    assert_eq!(read("2026-1-5 9:3").0, "2026-01-05T09:03:00");
    assert_eq!(read("-0001-01-01 00:00").0, "-000001-01-01T00:00:00");
    for rejected in [
        "",
        "2026-10-15",
        "2026-02-30 00:00",
        "2026-10-15 24:00",
        "2026-10-15 09:30:61",
        "2026-10-15T09:30:61Z",
        "2026-10-15t09:30:61z",
        "2026-10-15t24:00:00z",
        "2026-10-15t09:60:00z",
        "2026-10-15T12:30:00+05:",
        "2026-10-15T12:30:00+05:3",
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
