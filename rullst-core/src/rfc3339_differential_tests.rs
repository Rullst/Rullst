//! Differential tests against chrono 0.4.45, run before chrono left
//! rullst-core.

use super::*;
use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use proptest::prelude::*;

fn instant() -> impl Strategy<Value = (i64, u32)> {
    let seconds = prop_oneof![
        4 => -2_208_988_800_i64..4_133_980_800,
        1 => -62_167_219_300_i64..-62_167_219_100,
        1 => Timestamp::MIN.as_second()..=Timestamp::MAX.as_second(),
    ];
    let nanos = prop_oneof![
        Just(0_u32),
        (0_u32..1_000).prop_map(|millis| millis * 1_000_000),
        (0_u32..1_000_000).prop_map(|micros| micros * 1_000),
        0_u32..1_000_000_000,
    ];
    (seconds, nanos).prop_filter("jiff range", |(seconds, nanos)| {
        Timestamp::new(*seconds, *nanos as i32).is_ok()
    })
}

fn timestamp(seconds: i64, nanos: u32) -> Timestamp {
    Timestamp::new(seconds, nanos as i32).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 20_000, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn formatting_matches_chrono((seconds, nanos) in instant(), offset in -86_399_i32..86_400) {
        let ours = timestamp(seconds, nanos);
        let theirs = DateTime::<Utc>::from_timestamp(seconds, nanos).unwrap();
        prop_assert_eq!(unix_parts(ours), (theirs.timestamp(), theirs.timestamp_subsec_nanos()));
        prop_assert_eq!(format_utc(ours, Fraction::Auto), theirs.to_rfc3339_opts(SecondsFormat::AutoSi, true));
        prop_assert_eq!(format_utc(ours, Fraction::Millis), theirs.to_rfc3339_opts(SecondsFormat::Millis, true));
        prop_assert_eq!(format_utc(ours, Fraction::Nanos), theirs.to_rfc3339_opts(SecondsFormat::Nanos, true));
        prop_assert_eq!(serde_json::to_string(&theirs).unwrap(), format!("\"{}\"", format_utc(ours, Fraction::Auto)));
        prop_assert_eq!(utc_date(ours), theirs.format("%Y-%m-%d").to_string());
        prop_assert_eq!(format_with_offset(ours, 0), theirs.to_rfc3339());
        let zone = FixedOffset::east_opt(offset).unwrap();
        prop_assert_eq!(format_with_offset(ours, offset), theirs.with_timezone(&zone).to_rfc3339());
        // What chrono wrote, the lenient parser reads back.
        prop_assert_eq!(parse_timestamp(&theirs.to_rfc3339()), Some(ours));
        prop_assert_eq!(parse_timestamp(&serde_json::to_string(&theirs).unwrap().replace('"', "")), Some(ours));
        let text = theirs.to_rfc3339();
        prop_assert_eq!(parse_unix_seconds(&text), DateTime::parse_from_rfc3339(&text).ok().map(|time| time.timestamp()));
    }

    #[test]
    fn unix_millis_match_chrono(millis in prop_oneof![
        any::<i64>(),
        -8_334_601_228_800_010_i64..-8_334_601_228_799_990,
        8_210_266_876_799_990_i64..8_210_266_876_800_010,
        -100_000_000_000_000_i64..300_000_000_000_000,
    ]) {
        let theirs = DateTime::<Utc>::from_timestamp_millis(millis)
            .map(|time| time.to_rfc3339_opts(SecondsFormat::Millis, true));
        prop_assert_eq!(format_unix_millis_utc(millis, Fraction::Millis), theirs);
    }

    #[test]
    fn strict_parsing_matches_chrono(text in rfc3339_like()) {
        let theirs = DateTime::parse_from_rfc3339(&text).ok().map(|time| time.timestamp());
        prop_assert_eq!(parse_unix_seconds(&text), theirs, "{}", text);
    }
}

/// Strings shaped like RFC 3339, with mutations in every position.
fn rfc3339_like() -> impl Strategy<Value = String> {
    let digits = |count: usize| proptest::collection::vec(prop_oneof![9 => 0_u8..=9, 1 => 0_u8..=12], count)
        .prop_map(|values| values.iter().map(|value| if *value > 9 { 'x' } else { char::from(b'0' + value) }).collect::<String>());
    let separator = prop::sample::select(vec!["T", "t", " ", "_", "TT"]);
    let fraction = prop_oneof![
        3 => Just(String::new()),
        3 => "\\.[0-9]{1,12}",
        1 => Just(".".to_string()),
        1 => "[.,][0-9a-z]{0,3}",
    ];
    let offset = prop_oneof![
        2 => Just("Z".to_string()),
        1 => Just("z".to_string()),
        4 => ("[+\\-\u{2212}]", 0_u32..=26, 0_u32..=70).prop_map(|(sign, hours, minutes)| format!("{sign}{hours:02}:{minutes:02}")),
        1 => "[+\\-][0-9]{1,4}",
        1 => "[+\\-][0-9]{2}:[0-9]{1,3}",
        1 => Just(String::new()),
        1 => "[A-Za-z :\\[\\]]{1,4}",
    ];
    let month = prop_oneof![4 => 1_u32..=12, 1 => 0_u32..=99];
    let day = prop_oneof![4 => 1_u32..=31, 1 => 0_u32..=99];
    let hour = prop_oneof![4 => 0_u32..=23, 1 => 0_u32..=99];
    let minute = prop_oneof![4 => 0_u32..=59, 1 => 0_u32..=99];
    let second = prop_oneof![4 => 0_u32..=59, 1 => 58_u32..=62, 1 => 0_u32..=99];
    (
        digits(4),
        month,
        day,
        separator,
        hour,
        minute,
        second,
        fraction,
        offset,
        prop::option::weighted(0.05, "[ :/x-]{1,2}"),
    )
        .prop_map(
            |(year, month, day, separator, hour, minute, second, fraction, offset, noise)| {
                let mut text = format!(
                    "{year}-{month:02}-{day:02}{separator}{hour:02}:{minute:02}:{second:02}{fraction}{offset}"
                );
                if let Some(noise) = noise {
                    let at = text.len() / 2;
                    if text.is_char_boundary(at) {
                        text.insert_str(at, &noise);
                    }
                }
                text
            },
        )
}
