//! `DateTimeUtc` values shown and read in the administrator's time zone.

use super::super::{FormMode, validate_form_values};
use super::*;
use crate::nexus::types::FieldMeta;

fn entry() -> RegistryEntry {
    RegistryEntry {
        table: "events",
        label: "Events",
        icon: "E",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields: vec![
            FieldMeta::new("starts_at", "Starts at", FieldKind::DateTimeUtc),
            FieldMeta::new("ends_at", "Ends at", FieldKind::DateTimeUtc),
            FieldMeta::new("local_at", "Local at", FieldKind::DateTime),
            FieldMeta::new("note", "Note", FieldKind::Text),
        ],
    }
}

fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn picker(value: &str) -> Option<(&'static str, String)> {
    Some(("datetime-local", value.to_string()))
}

#[test]
fn stored_instants_are_shown_as_wall_times_of_a_fixed_offset() {
    let brasilia = NexusTimeZone::fixed_offset(-180).unwrap();
    assert_eq!(
        zoned_input_value("2026-10-15T12:30:00+00:00", &brasilia),
        picker("2026-10-15T09:30:00")
    );
    // PostgreSQL's text and a MySQL DATETIME (UTC without an offset).
    assert_eq!(
        zoned_input_value("2026-10-15 14:30:00.250+02", &brasilia),
        picker("2026-10-15T09:30:00.250")
    );
    assert_eq!(
        zoned_input_value("2026-10-15 12:30:00", &brasilia),
        picker("2026-10-15T09:30:00")
    );
    // The wall time, not the instant, must lie in years 1 to 9999.
    assert_eq!(zoned_input_value("0001-01-01T02:00:00Z", &brasilia), None);
    assert_eq!(
        zoned_input_value("0001-01-01T03:00:00Z", &brasilia),
        picker("0001-01-01T00:00:00")
    );
    let ahead = NexusTimeZone::fixed_offset(60).unwrap();
    assert_eq!(zoned_input_value("9999-12-31T23:30:00Z", &ahead), None);
    assert_eq!(
        zoned_input_value("9999-12-31T22:30:00Z", &ahead),
        picker("9999-12-31T23:30:00")
    );
}

#[cfg(feature = "tz")]
#[test]
fn named_zones_show_the_later_occurrence_of_a_fold_as_its_utc_instant() {
    let sao_paulo = NexusTimeZone::named("America/Sao_Paulo").unwrap();
    assert_eq!(
        zoned_input_value("2026-10-15T12:30:00Z", &sao_paulo),
        picker("2026-10-15T09:30:00")
    );
    // Daylight saving time in December 2018 (UTC-2).
    assert_eq!(
        zoned_input_value("2018-12-01T12:00:00Z", &sao_paulo),
        picker("2018-12-01T10:00:00")
    );
    let new_york = NexusTimeZone::named("America/New_York").unwrap();
    // 01:30 on 1 November 2026 happens twice: EDT first, then EST.
    assert_eq!(
        zoned_input_value("2026-11-01T05:30:00Z", &new_york),
        picker("2026-11-01T01:30:00")
    );
    assert_eq!(
        zoned_input_value("2026-11-01T06:30:00Z", &new_york),
        Some(("text", "2026-11-01T06:30:00Z".to_string()))
    );
}

#[test]
fn submitted_wall_times_are_read_in_the_zone_and_other_values_are_kept() {
    let brasilia = NexusTimeZone::fixed_offset(-180).unwrap();
    let localized = localize_form_values(
        &entry(),
        pairs(&[
            ("starts_at", "2026-10-15T09:30"),
            ("ends_at", "2026-10-15 10:45:30.125"),
            ("local_at", "2026-10-15T09:30"),
            ("note", "2026-10-15T09:30"),
            ("_token", "csrf"),
        ]),
        &brasilia,
    );
    assert_eq!(
        localized,
        Ok(pairs(&[
            ("starts_at", "2026-10-15T12:30:00Z"),
            ("ends_at", "2026-10-15T13:45:30.125Z"),
            ("local_at", "2026-10-15T09:30"),
            ("note", "2026-10-15T09:30"),
            ("_token", "csrf"),
        ]))
    );
    // A value with an offset, an empty or malformed value is left for
    // validation, which reads or rejects it as before.
    let kept = pairs(&[
        ("starts_at", "2026-10-15T09:30-02:00"),
        ("ends_at", ""),
        ("starts_at", "soon"),
    ]);
    assert_eq!(
        localize_form_values(&entry(), kept.clone(), &brasilia),
        Ok(kept)
    );
    assert_eq!(
        localize_form_values(
            &entry(),
            pairs(&[("starts_at", "2026-10-15T09:30")]),
            &NexusTimeZone::utc()
        ),
        Ok(pairs(&[("starts_at", "2026-10-15T09:30:00Z")]))
    );
}

#[test]
fn localized_values_validate_and_store_the_utc_instant() {
    let brasilia = NexusTimeZone::fixed_offset(-180).unwrap();
    let entry = entry();
    let localized = localize_form_values(
        &entry,
        pairs(&[("starts_at", "2026-10-15T23:30")]),
        &brasilia,
    )
    .unwrap();
    let values = validate_form_values(&entry, localized, FormMode::Update).unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(
        values[0].value.as_deref(),
        Some("2026-10-16T02:30:00+00:00")
    );
}

#[cfg(feature = "tz")]
#[test]
fn a_wall_time_in_a_daylight_saving_gap_is_a_typed_error() {
    let new_york = NexusTimeZone::named("America/New_York").unwrap();
    let error = localize_form_values(
        &entry(),
        pairs(&[("note", "x"), ("ends_at", "2026-03-08T02:30")]),
        &new_york,
    )
    .unwrap_err();
    assert_eq!(
        error,
        FormInputError::NonexistentLocalTime {
            field: "ends_at",
            zone: "America/New_York".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "field `ends_at` names a time that does not exist in America/New_York: clocks skip it at a daylight-saving change"
    );
    // A fold is read as its first occurrence (EDT).
    assert_eq!(
        localize_form_values(
            &entry(),
            pairs(&[("ends_at", "2026-11-01T01:30")]),
            &new_york
        ),
        Ok(pairs(&[("ends_at", "2026-11-01T05:30:00Z")]))
    );
}
