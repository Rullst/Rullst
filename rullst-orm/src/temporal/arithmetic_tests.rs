use jiff::{SignedDuration, Span};

use crate::{Date, DateTime, Time, Timestamp};

fn instant(text: &str) -> Timestamp {
    Timestamp::from(text.parse::<jiff::Timestamp>().unwrap())
}

fn date_time(text: &str) -> DateTime {
    DateTime::from(text.parse::<jiff::civil::DateTime>().unwrap())
}

fn date(text: &str) -> Date {
    Date::from(text.parse::<jiff::civil::Date>().unwrap())
}

#[test]
fn timestamp_arithmetic_returns_the_orm_type() {
    let start = instant("2026-10-15T09:30:00Z");
    let later: Timestamp = start.checked_add(SignedDuration::from_mins(90)).unwrap();
    assert_eq!(later, instant("2026-10-15T11:00:00Z"));
    let earlier: Timestamp = start.checked_sub(Span::new().hours(10)).unwrap();
    assert_eq!(earlier, instant("2026-10-14T23:30:00Z"));
    assert!(start.checked_add(Span::new().days(1)).is_err());

    let max = Timestamp::from(jiff::Timestamp::MAX);
    let min = Timestamp::from(jiff::Timestamp::MIN);
    assert!(max.checked_add(SignedDuration::from_nanos(1)).is_err());
    assert!(min.checked_sub(SignedDuration::from_nanos(1)).is_err());
    assert_eq!(start.saturating_add(SignedDuration::MAX).unwrap(), max);
    assert_eq!(start.saturating_sub(SignedDuration::MAX).unwrap(), min);
    assert_eq!(
        start.saturating_add(SignedDuration::from_secs(1)).unwrap(),
        instant("2026-10-15T09:30:01Z")
    );
    assert_eq!(
        start.saturating_sub(SignedDuration::from_secs(1)).unwrap(),
        instant("2026-10-15T09:29:59Z")
    );
}

#[test]
fn timestamp_differences_take_either_type() {
    let start = instant("2026-10-15T09:30:00Z");
    let end = instant("2026-10-15T11:00:00.5Z");
    let gap = SignedDuration::new(5_400, 500_000_000);
    assert_eq!(start.duration_until(end), gap);
    let by_reference = &end;
    assert_eq!(start.duration_until(by_reference), gap);
    assert_eq!(start.duration_until(*end), gap);
    assert_eq!(end.duration_since(start), gap);
    assert_eq!(start.duration_since(end), -gap);
}

#[test]
fn to_rfc3339_is_chronos() {
    for (text, rfc3339) in [
        ("2026-10-15T09:30:00Z", "2026-10-15T09:30:00+00:00"),
        ("2026-10-15T09:30:00.5Z", "2026-10-15T09:30:00.500+00:00"),
        (
            "2026-10-15T09:30:00.000001Z",
            "2026-10-15T09:30:00.000001+00:00",
        ),
        (
            "2026-10-15T09:30:00.1234567Z",
            "2026-10-15T09:30:00.123456700+00:00",
        ),
        (
            "1969-12-31T23:59:59.999999999Z",
            "1969-12-31T23:59:59.999999999+00:00",
        ),
        ("-000044-03-15T12:00:00Z", "-0044-03-15T12:00:00+00:00"),
    ] {
        assert_eq!(instant(text).to_rfc3339(), rfc3339, "{text}");
    }
}

#[test]
fn date_time_arithmetic_returns_the_orm_type() {
    let start = date_time("2026-02-28T22:00:00");
    let later: DateTime = start.checked_add(Span::new().days(1).hours(3)).unwrap();
    assert_eq!(later, date_time("2026-03-02T01:00:00"));
    let earlier: DateTime = start.checked_sub(SignedDuration::from_hours(24)).unwrap();
    assert_eq!(earlier, date_time("2026-02-27T22:00:00"));

    let max = DateTime::from(jiff::civil::DateTime::MAX);
    let min = DateTime::from(jiff::civil::DateTime::MIN);
    assert!(max.checked_add(SignedDuration::from_nanos(1)).is_err());
    assert!(min.checked_sub(SignedDuration::from_nanos(1)).is_err());
    assert_eq!(start.saturating_add(SignedDuration::MAX), max);
    assert_eq!(start.saturating_sub(SignedDuration::MAX), min);
    assert_eq!(
        start.saturating_add(Span::new().months(1)),
        date_time("2026-03-28T22:00:00")
    );
    assert_eq!(
        start.saturating_sub(Span::new().months(1)),
        date_time("2026-01-28T22:00:00")
    );

    let end = date_time("2026-03-01T22:00:00.25");
    let gap = SignedDuration::new(86_400, 250_000_000);
    assert_eq!(start.duration_until(end), gap);
    let by_reference = &end;
    assert_eq!(start.duration_until(by_reference), gap);
    assert_eq!(end.duration_since(start), gap);
    assert_eq!(start.duration_since(end), -gap);

    assert_eq!(start.tomorrow().unwrap(), date_time("2026-03-01T22:00:00"));
    assert_eq!(start.yesterday().unwrap(), date_time("2026-02-27T22:00:00"));
    assert!(max.tomorrow().is_err());
    assert!(min.yesterday().is_err());
}

#[test]
fn date_time_parts_are_the_orm_types() {
    let value = date_time("2026-10-15T09:30:00.5");
    let day: Date = value.date();
    let clock: Time = value.time();
    assert_eq!(day, date("2026-10-15"));
    assert_eq!(
        clock,
        Time::from("09:30:00.5".parse::<jiff::civil::Time>().unwrap())
    );
}

#[test]
fn date_arithmetic_returns_the_orm_type() {
    let start = date("2026-01-31");
    let later: Date = start.checked_add(Span::new().months(1)).unwrap();
    assert_eq!(later, date("2026-02-28"));
    let earlier: Date = start.checked_sub(SignedDuration::from_hours(48)).unwrap();
    assert_eq!(earlier, date("2026-01-29"));

    let max = Date::from(jiff::civil::Date::MAX);
    let min = Date::from(jiff::civil::Date::MIN);
    assert!(max.checked_add(Span::new().days(1)).is_err());
    assert!(min.checked_sub(Span::new().days(1)).is_err());
    assert_eq!(start.saturating_add(Span::new().years(19_998)), max);
    assert_eq!(start.saturating_sub(Span::new().years(19_998)), min);
    assert_eq!(
        start.saturating_add(Span::new().days(1)),
        date("2026-02-01")
    );
    assert_eq!(
        start.saturating_sub(Span::new().days(1)),
        date("2026-01-30")
    );

    assert_eq!(start.tomorrow().unwrap(), date("2026-02-01"));
    assert_eq!(start.yesterday().unwrap(), date("2026-01-30"));
    assert!(max.tomorrow().is_err());
    assert!(min.yesterday().is_err());
}

#[test]
fn date_differences_count_days() {
    let start = date("2026-01-01");
    let end = date("2026-03-01");
    assert_eq!(
        start.duration_until(end),
        SignedDuration::from_hours(59 * 24)
    );
    assert_eq!(
        start.duration_since(end),
        SignedDuration::from_hours(-59 * 24)
    );
    assert_eq!(start.days_until(end), 59);
    let by_reference = &end;
    assert_eq!(start.days_until(by_reference), 59);
    assert_eq!(end.days_until(start), -59);
    assert_eq!(end.days_since(start), 59);
    assert_eq!(start.days_since(*end), -59);
    assert_eq!(
        Date::from(jiff::civil::Date::MIN).days_until(jiff::civil::Date::MAX),
        7_304_483
    );
}

#[test]
fn references_convert_to_jiff() {
    fn civil_date(value: impl Into<jiff::civil::Date>) -> jiff::civil::Date {
        value.into()
    }
    let value = date_time("2026-10-15T09:30:00.5");
    let day = value.date();
    let clock = value.time();
    let at = instant("2026-10-15T09:30:00Z");
    let by_reference = &day;
    assert_eq!(civil_date(by_reference), *day);
    assert_eq!(jiff::civil::DateTime::from(&value), *value);
    assert_eq!(jiff::civil::Time::from(&clock), *clock);
    assert_eq!(jiff::Timestamp::from(&at), *at);
}
