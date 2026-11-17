use super::{NexusTimeZone, NexusTimeZoneError};
use crate::nexus::instant::Instant;
use jiff::civil::DateTime;

fn local(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// The instant of a UTC wall time written `…Z`.
fn instant(text: &str) -> Instant {
    Instant::at_offset(local(text.strip_suffix('Z').unwrap()), 0).unwrap()
}

#[test]
fn utc_is_the_default_and_reads_wall_times_unchanged() {
    let zone = NexusTimeZone::default();
    assert_eq!(zone, NexusTimeZone::utc());
    assert_eq!(zone.name(), "UTC");
    assert_eq!(zone.offset_at(&instant("2026-10-15T09:30:00Z")), Some(0));
    assert_eq!(
        zone.instant_of(&local("2026-10-15T09:30:00.250")),
        Ok(instant("2026-10-15T09:30:00.250Z"))
    );
}

#[test]
fn fixed_offsets_are_named_and_bounded() {
    let brasilia = NexusTimeZone::fixed_offset(-180).unwrap();
    assert_eq!(brasilia.name(), "UTC-03:00");
    assert_eq!(
        brasilia.offset_at(&instant("2026-10-15T12:30:00Z")),
        Some(-10_800)
    );
    assert_eq!(
        brasilia.instant_of(&local("2026-10-15T09:30:00")),
        Ok(instant("2026-10-15T12:30:00Z"))
    );
    let india = NexusTimeZone::fixed_offset(330).unwrap();
    assert_eq!(india.name(), "UTC+05:30");
    assert_eq!(
        india.instant_of(&local("2026-10-15T09:30:00")),
        Ok(instant("2026-10-15T04:00:00Z"))
    );
    assert_eq!(NexusTimeZone::fixed_offset(0).unwrap().name(), "UTC+00:00");
    assert_eq!(
        NexusTimeZone::fixed_offset(1439).unwrap().name(),
        "UTC+23:59"
    );
    assert_eq!(
        NexusTimeZone::fixed_offset(-1439).unwrap().name(),
        "UTC-23:59"
    );
    for minutes in [1440, -1440, i32::MIN] {
        let error = NexusTimeZone::fixed_offset(minutes).unwrap_err();
        assert_eq!(error, NexusTimeZoneError::OffsetOutOfRange { minutes });
    }
    assert_eq!(
        NexusTimeZoneError::OffsetOutOfRange { minutes: 1440 }.to_string(),
        "a fixed UTC offset must be less than 24 hours, not 1440 minutes"
    );
}

#[cfg(feature = "tz")]
#[test]
fn named_zones_follow_their_daylight_saving_rules() {
    let sao_paulo = NexusTimeZone::named("America/Sao_Paulo").unwrap();
    assert_eq!(sao_paulo.name(), "America/Sao_Paulo");
    assert_eq!(
        sao_paulo.offset_at(&instant("2026-10-15T12:30:00Z")),
        Some(-10_800)
    );
    assert_eq!(
        sao_paulo.instant_of(&local("2026-10-15T09:30:00")),
        Ok(instant("2026-10-15T12:30:00Z"))
    );
    // Brazil observed daylight saving time until 2019: on 4 November 2018
    // clocks moved from 00:00 to 01:00, and on 18 February 2018 from 00:00
    // back to 23:00 of the 17th.
    assert_eq!(
        sao_paulo.offset_at(&instant("2018-12-01T12:00:00Z")),
        Some(-7_200)
    );
    let gap = sao_paulo.instant_of(&local("2018-11-04T00:30:00"));
    assert_eq!(
        gap,
        Err(NexusTimeZoneError::NonexistentLocalTime {
            local: "2018-11-04T00:30:00".to_string(),
            zone: "America/Sao_Paulo".to_string(),
        })
    );
    assert_eq!(
        gap.unwrap_err().to_string(),
        "2018-11-04T00:30:00 does not exist in America/Sao_Paulo: clocks skip it at a daylight-saving change"
    );
    // The repeated 23:30 is read as its first, daylight-saving instant.
    assert_eq!(
        sao_paulo.instant_of(&local("2018-02-17T23:30:00")),
        Ok(instant("2018-02-18T01:30:00Z"))
    );

    let new_york = NexusTimeZone::named("America/New_York").unwrap();
    assert!(new_york.instant_of(&local("2026-03-08T02:30:00")).is_err());
    assert_eq!(
        new_york.instant_of(&local("2026-03-08T03:00:00")),
        Ok(instant("2026-03-08T07:00:00Z"))
    );
    assert_eq!(
        new_york.instant_of(&local("2026-11-01T01:30:00")),
        Ok(instant("2026-11-01T05:30:00Z"))
    );
    assert_eq!(
        new_york.offset_at(&instant("2026-11-01T06:30:00Z")),
        Some(-18_000)
    );

    let unknown = NexusTimeZone::named("Brasilia").unwrap_err();
    assert_eq!(
        unknown,
        NexusTimeZoneError::UnknownZone {
            name: "Brasilia".to_string()
        }
    );
    assert_eq!(unknown.to_string(), "unknown IANA time zone `Brasilia`");
}

#[cfg(feature = "tz")]
#[test]
fn named_zones_keep_refusing_wall_times_past_the_last_jiff_instant() {
    let new_york = NexusTimeZone::named("America/New_York").unwrap();
    assert_eq!(
        new_york.instant_of(&local("9999-12-30T16:59:59.999")),
        Ok(instant("9999-12-30T21:59:59.999Z"))
    );
    // Read as UTC, these wall times are later than 9999-12-30T22:00Z, which
    // jiff 0.2 could not represent either: Nexus refused them, and still does.
    assert_eq!(
        new_york.instant_of(&local("9999-12-30T22:00:01.5")),
        Err(NexusTimeZoneError::NonexistentLocalTime {
            local: "9999-12-30T22:00:01.500".to_string(),
            zone: "America/New_York".to_string(),
        })
    );
    // An instant past it has no named-zone offset.
    assert_eq!(new_york.offset_at(&instant("9999-12-30T22:00:01Z")), None);
    assert_eq!(
        new_york.offset_at(&instant("9999-12-30T21:00:00Z")),
        Some(-18_000)
    );
    // A fixed offset has neither limit.
    let fixed = NexusTimeZone::fixed_offset(-300).unwrap();
    assert_eq!(
        fixed.offset_at(&instant("9999-12-31T23:00:00Z")),
        Some(-18_000)
    );
    assert_eq!(
        fixed.instant_of(&local("9999-12-31T23:00:00")),
        Ok(Instant::at_offset(local("9999-12-31T23:00:00"), -18_000).unwrap())
    );
}
