//! The administrator's time zone, in which Nexus shows and reads
//! [`FieldKind::DateTimeUtc`](super::FieldKind::DateTimeUtc) form values.

use std::fmt;

use rullst_orm::chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};

/// The time zone of the `datetime-local` inputs of `DateTimeUtc` fields.
///
/// A `datetime-local` input has no zone. Nexus shows a stored instant as its
/// wall time in this zone, reads a submitted value without an offset as a
/// wall time in this zone and stores the UTC instant; the field label names
/// the zone. A value with `Z`, `±HH:MM` or `±HH` keeps its own offset. The
/// default is UTC. Select it with [`Nexus::with_time_zone`](super::Nexus::with_time_zone).
///
/// With the `tz` feature, [`NexusTimeZone::named`] follows an IANA zone's
/// daylight-saving rules:
/// - a wall time that a gap skips (clocks move forward) does not exist and
///   is refused with a `422` validation error naming the zone;
/// - a wall time that a fold repeats (clocks move back) is read as its first,
///   earlier instant. A stored instant at the later occurrence is shown in a
///   text input as its UTC value (`2026-11-01T06:30:00Z`), so saving the form
///   unchanged keeps it; type an offset to choose that occurrence.
///
/// ```
/// use rullst_nexus::NexusTimeZone;
///
/// assert_eq!(NexusTimeZone::utc().name(), "UTC");
/// assert_eq!(NexusTimeZone::fixed_offset(-180)?.name(), "UTC-03:00");
/// assert!(NexusTimeZone::fixed_offset(24 * 60).is_err());
/// # Ok::<(), rullst_nexus::NexusTimeZoneError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusTimeZone {
    name: String,
    rules: Rules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rules {
    /// Seconds east of UTC.
    Fixed(i32),
    #[cfg(feature = "tz")]
    Named(jiff::tz::TimeZone),
}

impl Default for NexusTimeZone {
    fn default() -> Self {
        Self::utc()
    }
}

impl NexusTimeZone {
    /// UTC, the default.
    pub fn utc() -> Self {
        Self {
            name: "UTC".to_string(),
            rules: Rules::Fixed(0),
        }
    }

    /// A fixed offset of `minutes_east` minutes from UTC (`-180` is UTC−3),
    /// with no daylight-saving changes. Its name is `UTC±HH:MM`.
    ///
    /// # Errors
    /// [`NexusTimeZoneError::OffsetOutOfRange`] for 24 hours or more.
    pub fn fixed_offset(minutes_east: i32) -> Result<Self, NexusTimeZoneError> {
        let minutes = minutes_east.unsigned_abs();
        if minutes >= 24 * 60 {
            return Err(NexusTimeZoneError::OffsetOutOfRange {
                minutes: minutes_east,
            });
        }
        let sign = if minutes_east < 0 { '-' } else { '+' };
        Ok(Self {
            name: format!("UTC{sign}{:02}:{:02}", minutes / 60, minutes % 60),
            rules: Rules::Fixed(minutes_east * 60),
        })
    }

    /// The IANA time zone `name`, such as `America/Sao_Paulo`, with its
    /// daylight-saving rules. Requires the `tz` feature, which uses `jiff`
    /// like Core's `scheduler-tz`: zone rules come from the system database
    /// (`/usr/share/zoneinfo`) when it has the zone, otherwise from the copy
    /// bundled with the feature.
    ///
    /// # Errors
    /// [`NexusTimeZoneError::UnknownZone`] when the database has no such zone.
    #[cfg(feature = "tz")]
    pub fn named(name: impl Into<String>) -> Result<Self, NexusTimeZoneError> {
        let name = name.into();
        match jiff::tz::TimeZone::get(&name) {
            Ok(zone) => Ok(Self {
                name,
                rules: Rules::Named(zone),
            }),
            Err(_) => Err(NexusTimeZoneError::UnknownZone { name }),
        }
    }

    /// The name shown next to `DateTimeUtc` fields: `UTC`, `UTC±HH:MM` or
    /// the IANA name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The offset from UTC, in seconds, of `instant` in this zone.
    // Only a named zone's offset depends on the instant.
    #[cfg_attr(not(feature = "tz"), allow(unused_variables))]
    pub(crate) fn offset_at(&self, instant: &DateTime<Utc>) -> Option<i32> {
        match &self.rules {
            Rules::Fixed(seconds) => Some(*seconds),
            #[cfg(feature = "tz")]
            Rules::Named(zone) => Some(zone.to_offset(timestamp(instant)?).seconds()),
        }
    }

    /// The instant of the wall time `local` in this zone: the earlier one
    /// when a fold repeats it.
    ///
    /// # Errors
    /// [`NexusTimeZoneError::NonexistentLocalTime`] when a gap skips it.
    pub(crate) fn instant_of(
        &self,
        local: &NaiveDateTime,
    ) -> Result<DateTime<Utc>, NexusTimeZoneError> {
        let offset = match &self.rules {
            Rules::Fixed(seconds) => Some(*seconds),
            #[cfg(feature = "tz")]
            Rules::Named(zone) => named_offset(zone, local),
        };
        offset
            .and_then(|seconds| {
                local
                    .and_utc()
                    .checked_sub_signed(TimeDelta::seconds(i64::from(seconds)))
            })
            .ok_or_else(|| NexusTimeZoneError::NonexistentLocalTime {
                local: local.format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
                zone: self.name.clone(),
            })
    }
}

/// The offset of the wall time `local` in `zone`: `None` in a gap, the
/// earlier instant's offset in a fold.
#[cfg(feature = "tz")]
fn named_offset(zone: &jiff::tz::TimeZone, local: &NaiveDateTime) -> Option<i32> {
    use jiff::tz::{AmbiguousOffset, Offset};
    let civil = Offset::UTC.to_datetime(timestamp(&local.and_utc())?);
    match zone.to_ambiguous_timestamp(civil).offset() {
        AmbiguousOffset::Unambiguous { offset } => Some(offset.seconds()),
        // The offset before the change: clocks were ahead, so the instant is
        // the earlier of the two.
        AmbiguousOffset::Fold { before, .. } => Some(before.seconds()),
        AmbiguousOffset::Gap { .. } => None,
    }
}

#[cfg(feature = "tz")]
fn timestamp(instant: &DateTime<Utc>) -> Option<jiff::Timestamp> {
    let nanosecond = i32::try_from(instant.timestamp_subsec_nanos()).ok()?;
    jiff::Timestamp::new(instant.timestamp(), nanosecond).ok()
}

/// A [`NexusTimeZone`] that cannot be built, or a local time it cannot read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NexusTimeZoneError {
    /// A fixed offset of 24 hours or more, in minutes east of UTC.
    OffsetOutOfRange { minutes: i32 },
    /// The time-zone database has no zone of this name.
    UnknownZone { name: String },
    /// A wall time that a daylight-saving gap skips in `zone`.
    NonexistentLocalTime { local: String, zone: String },
}

impl fmt::Display for NexusTimeZoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OffsetOutOfRange { minutes } => write!(
                f,
                "a fixed UTC offset must be less than 24 hours, not {minutes} minutes"
            ),
            Self::UnknownZone { name } => write!(f, "unknown IANA time zone `{name}`"),
            Self::NonexistentLocalTime { local, zone } => write!(
                f,
                "{local} does not exist in {zone}: clocks skip it at a daylight-saving change"
            ),
        }
    }
}

impl std::error::Error for NexusTimeZoneError {}

#[cfg(test)]
#[path = "time_zone_tests.rs"]
mod tests;
