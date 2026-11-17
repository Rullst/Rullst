use super::{Date, DateTime, Time, Timestamp};
use crate::RullstValue;

fn instant() -> Timestamp {
    "2026-10-15T09:30:00.5Z".parse().expect("valid instant")
}

#[test]
fn display_and_from_str_use_jiff_forms() {
    assert_eq!(instant().to_string(), "2026-10-15T09:30:00.5Z");
    assert_eq!(format!("{:?}", instant()), "2026-10-15T09:30:00.5Z");
    assert_eq!(
        "2026-10-15T12:30:00+03:00".parse::<Timestamp>().unwrap(),
        "2026-10-15T09:30:00Z".parse::<Timestamp>().unwrap()
    );
    assert!("2026-10-15T09:30:00".parse::<Timestamp>().is_err());

    let civil: DateTime = "2026-10-15 09:30:00.25".parse().unwrap();
    assert_eq!(civil.to_string(), "2026-10-15T09:30:00.25");
    assert_eq!(format!("{civil:?}"), "2026-10-15T09:30:00.25");
    let date: Date = "2026-10-15".parse().unwrap();
    assert_eq!(date.to_string(), "2026-10-15");
    assert_eq!(format!("{date:?}"), "2026-10-15");
    let time: Time = "09:30:00.000001".parse().unwrap();
    assert_eq!(time.to_string(), "09:30:00.000001");
    assert_eq!(format!("{time:?}"), "09:30:00.000001");
    assert!("25:00".parse::<Time>().is_err());
}

#[test]
fn serde_is_jiffs_representation() {
    let json = serde_json::to_string(&instant()).unwrap();
    assert_eq!(json, "\"2026-10-15T09:30:00.5Z\"");
    assert_eq!(
        json,
        serde_json::to_string(&jiff::Timestamp::from(instant())).unwrap()
    );
    // chrono's serde form (`.500Z`) still reads.
    let read: Timestamp = serde_json::from_str("\"2026-10-15T09:30:00.500Z\"").unwrap();
    assert_eq!(read, instant());

    let civil: DateTime = serde_json::from_str("\"2026-10-15T09:30:00.500\"").unwrap();
    assert_eq!(
        serde_json::to_string(&civil).unwrap(),
        "\"2026-10-15T09:30:00.5\""
    );
    let date: Date = serde_json::from_str("\"2026-10-15\"").unwrap();
    assert_eq!(serde_json::to_string(&date).unwrap(), "\"2026-10-15\"");
    let time: Time = serde_json::from_str("\"09:30:00.500\"").unwrap();
    assert_eq!(serde_json::to_string(&time).unwrap(), "\"09:30:00.5\"");
}

#[test]
fn conversions_and_deref_reach_the_jiff_value() {
    let jiff_instant = jiff::Timestamp::from_millisecond(1_792_056_600_500).unwrap();
    assert_eq!(Timestamp::from(jiff_instant), instant());
    assert_eq!(jiff::Timestamp::from(instant()), jiff_instant);
    assert_eq!(instant().as_millisecond(), 1_792_056_600_500);

    let civil = jiff::civil::date(2026, 10, 15).at(9, 30, 0, 0);
    assert_eq!(DateTime::from(civil).hour(), 9);
    assert_eq!(jiff::civil::DateTime::from(DateTime::from(civil)), civil);
    let date = jiff::civil::date(2026, 10, 15);
    assert_eq!(Date::from(date).day(), 15);
    assert_eq!(jiff::civil::Date::from(Date::from(date)), date);
    let time = jiff::civil::time(9, 30, 0, 7);
    assert_eq!(Time::from(time).subsec_nanosecond(), 7);
    assert_eq!(jiff::civil::Time::from(Time::from(time)), time);
}

#[test]
fn defaults_are_chronos() {
    use super::test_support::{chrono_date, chrono_date_time, chrono_instant, chrono_time};

    assert_eq!(Timestamp::default().to_string(), "1970-01-01T00:00:00Z");
    assert_eq!(DateTime::default().to_string(), "1970-01-01T00:00:00");
    assert_eq!(Date::default().to_string(), "1970-01-01");
    assert_eq!(Time::default().to_string(), "00:00:00");
    assert_eq!(
        chrono_instant(Timestamp::default()),
        chrono::DateTime::<chrono::Utc>::default()
    );
    assert_eq!(
        chrono_date_time(DateTime::default()),
        chrono::NaiveDateTime::default()
    );
    assert_eq!(chrono_date(*Date::default()), chrono::NaiveDate::default());
    assert_eq!(chrono_time(*Time::default()), chrono::NaiveTime::default());
}

#[test]
fn now_reads_the_clock() {
    let before = jiff::Timestamp::now();
    let now = Timestamp::now();
    assert!(*now >= before);
    assert!(*now <= jiff::Timestamp::now());
}

#[test]
fn ordering_follows_time() {
    let later = Timestamp::from(
        instant()
            .checked_add(jiff::SignedDuration::from_nanos(1))
            .unwrap(),
    );
    assert!(instant() < later);
    let mut values = [later, instant()];
    values.sort();
    assert_eq!(values, [instant(), later]);
}

#[test]
fn each_type_becomes_its_value_variant() {
    assert!(matches!(RullstValue::from(instant()), RullstValue::Timestamp(at) if at == instant()));
    let civil = DateTime::from(jiff::civil::date(2026, 10, 15).at(9, 30, 0, 0));
    assert!(matches!(RullstValue::from(civil), RullstValue::CivilDateTime(at) if at == civil));
    let date = Date::from(jiff::civil::date(2026, 10, 15));
    assert!(matches!(RullstValue::from(date), RullstValue::CivilDate(at) if at == date));
    let time = Time::from(jiff::civil::time(9, 30, 0, 0));
    assert!(matches!(RullstValue::from(time), RullstValue::CivilTime(at) if at == time));
    assert!(matches!(
        RullstValue::from(None::<Timestamp>),
        RullstValue::Null
    ));
    assert!(matches!(
        RullstValue::from(Some(date)),
        RullstValue::CivilDate(at) if at == date
    ));
}
