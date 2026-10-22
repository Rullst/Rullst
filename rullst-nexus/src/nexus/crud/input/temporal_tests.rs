use super::*;

fn debug(value: Option<RullstValue>) -> String {
    format!("{value:?}")
}

#[test]
fn dates_are_four_two_two_digits_from_year_one() {
    assert_eq!(date("2024-02-29"), NaiveDate::from_ymd_opt(2024, 2, 29));
    assert_eq!(date("0001-01-01"), NaiveDate::from_ymd_opt(1, 1, 1));
    for invalid in [
        "0000-01-01",
        "2026-02-29",
        "2026-13-01",
        "2026-1-01",
        "26-01-01",
        "2026-01-01-01",
        "2026-01",
        "2026/01/01",
        "",
    ] {
        assert_eq!(date(invalid), None, "{invalid:?}");
    }
}

#[test]
fn times_accept_minutes_seconds_and_up_to_nine_fraction_digits() {
    for (value, expected) in [
        ("00:00", (0, 0, 0, 0)),
        ("23:59", (23, 59, 0, 0)),
        ("09:30:59", (9, 30, 59, 0)),
        ("09:30:00.5", (9, 30, 0, 500_000_000)),
        ("09:30:00.123456", (9, 30, 0, 123_456_000)),
        ("09:30:00.999999999", (9, 30, 0, 999_999_999)),
    ] {
        let (hour, minute, second, nano) = expected;
        assert_eq!(
            time(value),
            NaiveTime::from_hms_nano_opt(hour, minute, second, nano),
            "{value}"
        );
    }
    for invalid in [
        "24:00",
        "12:60",
        "12:30:60",
        "9:30",
        "09:30:00.",
        "09:30:00.1234567890",
        "09:30:00.12a",
        "09:30:00:00",
        "09",
        "",
    ] {
        assert_eq!(time(invalid), None, "{invalid:?}");
    }
}

#[test]
fn local_date_times_have_no_offset() {
    assert_eq!(
        local_date_time("2026-10-15T09:30"),
        NaiveDate::from_ymd_opt(2026, 10, 15).and_then(|date| date.and_hms_opt(9, 30, 0))
    );
    assert_eq!(
        local_date_time("2026-10-15 09:30:15"),
        NaiveDate::from_ymd_opt(2026, 10, 15).and_then(|date| date.and_hms_opt(9, 30, 15))
    );
    for invalid in [
        "2026-10-15T09:30Z",
        "2026-10-15T09:30+01:00",
        "2026-10-15",
        "2026-10-15t09:30",
        "2026-02-30T09:30",
    ] {
        assert_eq!(local_date_time(invalid), None, "{invalid:?}");
    }
}

fn instant(text: &str) -> Option<String> {
    utc(text).map(|value| value.to_rfc3339_opts(SecondsFormat::AutoSi, true))
}

#[test]
fn utc_values_without_an_offset_are_utc_and_offsets_are_converted() {
    for (value, expected) in [
        ("2026-10-15T09:30", "2026-10-15T09:30:00Z"),
        ("2026-10-15 09:30:00.250", "2026-10-15T09:30:00.250Z"),
        ("2026-10-15T09:30:00Z", "2026-10-15T09:30:00Z"),
        ("2026-10-15T09:30:00z", "2026-10-15T09:30:00Z"),
        ("2026-10-15T09:30:00+00:00", "2026-10-15T09:30:00Z"),
        ("2026-10-15T09:30:00-03:00", "2026-10-15T12:30:00Z"),
        ("2026-10-15T09:30+05:45", "2026-10-15T03:45:00Z"),
        // PostgreSQL's text form of a TIMESTAMPTZ.
        ("2026-10-15 11:30:00+02", "2026-10-15T09:30:00Z"),
        (
            "2026-10-15 09:30:00.123456-00",
            "2026-10-15T09:30:00.123456Z",
        ),
        ("2026-10-15T00:30:00+23:59", "2026-10-14T00:31:00Z"),
        // MySQL's DATETIME text, which holds UTC.
        ("2026-10-15 09:30:00.500000", "2026-10-15T09:30:00.500Z"),
    ] {
        assert_eq!(instant(value).as_deref(), Some(expected), "{value}");
    }
    for invalid in [
        "2026-10-15T09:30:00+24:00",
        "2026-10-15T09:30:00+05:60",
        "2026-10-15T09:30:00+0530",
        "2026-10-15T09:30:00+05:30:00",
        "2026-10-15T09:30:00+",
        "2026-10-15T09:30:00-",
        "2026-10-15T09:30:00+5",
        "2026-10-15T24:00Z",
        "2026-10-15",
        "yesterday",
    ] {
        assert_eq!(instant(invalid), None, "{invalid:?}");
    }
}

#[test]
fn normalized_values_carry_their_text_and_typed_value() {
    let normalized = |kind: FieldKind, value: &str| {
        normalize(&kind, value.to_owned()).map(|(text, typed)| (text, debug(typed)))
    };
    assert_eq!(
        normalized(FieldKind::Date, "2026-10-15"),
        Some(("2026-10-15".to_owned(), "Some(Date(2026-10-15))".to_owned()))
    );
    assert_eq!(
        normalized(FieldKind::Time, "09:30"),
        Some(("09:30:00".to_owned(), "Some(Time(09:30:00))".to_owned()))
    );
    assert_eq!(
        normalized(FieldKind::Time, "09:30:00.5"),
        Some((
            "09:30:00.500".to_owned(),
            "Some(Time(09:30:00.500))".to_owned()
        ))
    );
    assert_eq!(
        normalized(FieldKind::DateTime, "2026-10-15T09:30"),
        Some((
            "2026-10-15 09:30:00".to_owned(),
            "Some(NaiveDateTime(2026-10-15T09:30:00))".to_owned()
        ))
    );
    // A text column keeps an offset value as entered, bound as text.
    assert_eq!(
        normalized(FieldKind::DateTime, "2026-10-15T09:30Z"),
        Some(("2026-10-15T09:30Z".to_owned(), "None".to_owned()))
    );
    assert_eq!(
        normalized(FieldKind::DateTimeUtc, "2026-10-15T09:30:00-03:00"),
        Some((
            "2026-10-15T12:30:00+00:00".to_owned(),
            "Some(DateTime(2026-10-15T12:30:00Z))".to_owned()
        ))
    );
    assert_eq!(
        normalized(FieldKind::Text, "09:30"),
        Some(("09:30".to_owned(), "None".to_owned()))
    );
    for (kind, invalid) in [
        (FieldKind::Date, "0000-01-01"),
        (FieldKind::Time, "25:00"),
        (FieldKind::DateTimeUtc, "2026-10-15"),
    ] {
        assert_eq!(normalized(kind, invalid), None, "{invalid}");
    }
}

#[test]
fn inputs_show_values_they_can_hold_exactly() {
    assert_eq!(time_input_value("09:30").as_deref(), Some("09:30:00"));
    assert_eq!(time_input_value("09:30:15").as_deref(), Some("09:30:15"));
    assert_eq!(
        time_input_value("09:30:15.500000").as_deref(),
        Some("09:30:15.500")
    );
    assert_eq!(
        time_input_value("09:30:15.007").as_deref(),
        Some("09:30:15.007")
    );
    assert_eq!(
        time_input_value("09:30:15.000").as_deref(),
        Some("09:30:15")
    );
    for unshowable in ["09:30:15.0005", "09:30:15.123456", "noon", "24:00"] {
        assert_eq!(time_input_value(unshowable), None, "{unshowable}");
    }

    let utc = NexusTimeZone::utc();
    let shown = |text| zoned_input_value(text, &utc);
    let picker = |value: &str| Some(("datetime-local", value.to_string()));
    assert_eq!(
        shown("2026-10-15 11:30:00+02"),
        picker("2026-10-15T09:30:00")
    );
    assert_eq!(
        shown("2026-10-15T09:30:00.123+00:00"),
        picker("2026-10-15T09:30:00.123")
    );
    assert_eq!(
        shown("2026-10-15 09:30:00.500000"),
        picker("2026-10-15T09:30:00.500")
    );
    for unshowable in [
        "2026-10-15 09:30:00.123456+00",
        // The UTC instant falls in year 0, which a picker cannot show.
        "0001-01-01T00:30:00+01:00",
        "legacy",
    ] {
        assert_eq!(shown(unshowable), None, "{unshowable}");
    }
    assert_eq!(shown("9999-12-31T23:59:59Z"), picker("9999-12-31T23:59:59"));
}

#[test]
fn offsets_are_found_only_in_the_time_part() {
    for with_offset in [
        "2026-10-15T09:30Z",
        "2026-10-15 09:30z",
        "2026-10-15T09:30+01:00",
        "2026-10-15 09:30-03",
    ] {
        assert!(has_offset(with_offset), "{with_offset}");
    }
    for local in ["2026-10-15 09:30:00", "2026-10-15T09:30", "2026-10-15", ""] {
        assert!(!has_offset(local), "{local:?}");
    }
}
