//! Type-directed date/time widget inference for `#[derive(Nexus)]`.
//!
//! The derive sees field types only as tokens, and a field spelled `Date` or
//! `Timestamp` may be the application's own type. For a field whose spelling
//! gives no widget, the generated code asks the compiler instead:
//! `(&Probe::<FieldType>::new()).__rullst_temporal_kind()` resolves to
//! [`Temporal`] when the field is one of the ORM's date/time types or an
//! `Option` of one, under any path, import or alias, and to [`Other`] for
//! every other type.

use std::marker::PhantomData;

/// The ORM date/time type a field holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalKind {
    /// [`crate::Timestamp`].
    Timestamp,
    /// [`crate::DateTime`].
    DateTime,
    /// [`crate::Date`].
    Date,
    /// [`crate::Time`].
    Time,
}

/// Names a field type for autoref dispatch.
pub struct Probe<T>(PhantomData<T>);

impl<T> Probe<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for Probe<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Implemented for the probe of a date/time field.
pub trait Temporal {
    fn __rullst_temporal_kind(&self) -> Option<TemporalKind>;
}

macro_rules! temporal {
    ($($type:ty => $kind:ident),+ $(,)?) => {
        $(
            impl Temporal for Probe<$type> {
                fn __rullst_temporal_kind(&self) -> Option<TemporalKind> {
                    Some(TemporalKind::$kind)
                }
            }

            impl Temporal for Probe<Option<$type>> {
                fn __rullst_temporal_kind(&self) -> Option<TemporalKind> {
                    Some(TemporalKind::$kind)
                }
            }
        )+
    };
}

temporal! {
    crate::Timestamp => Timestamp,
    crate::DateTime => DateTime,
    crate::Date => Date,
    crate::Time => Time,
}

/// Every other field type: no date/time widget.
pub trait Other {
    fn __rullst_temporal_kind(&self) -> Option<TemporalKind>;
}

impl<T> Other for &Probe<T> {
    fn __rullst_temporal_kind(&self) -> Option<TemporalKind> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{Other as _, Probe, Temporal as _, TemporalKind};

    struct Date;
    type Stamp = crate::Timestamp;

    // Generated code borrows every probe the same way, whatever its type.
    #[allow(clippy::needless_borrow)]
    #[test]
    fn only_the_orm_date_time_types_have_a_temporal_kind() {
        assert_eq!(
            (&Probe::<Stamp>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::Timestamp)
        );
        assert_eq!(
            (&Probe::<Option<crate::Timestamp>>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::Timestamp)
        );
        assert_eq!(
            (&Probe::<crate::DateTime>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::DateTime)
        );
        assert_eq!(
            (&Probe::<Option<crate::DateTime>>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::DateTime)
        );
        assert_eq!(
            (&Probe::<crate::Date>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::Date)
        );
        assert_eq!(
            (&Probe::<Option<crate::Date>>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::Date)
        );
        assert_eq!(
            (&Probe::<crate::Time>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::Time)
        );
        assert_eq!(
            (&Probe::<Option<crate::Time>>::new()).__rullst_temporal_kind(),
            Some(TemporalKind::Time)
        );
        // An application type named like one of them, a jiff type and a
        // chrono type have none.
        assert_eq!((&Probe::<Date>::new()).__rullst_temporal_kind(), None);
        assert_eq!((&Probe::<jiff::civil::Date>::new()).__rullst_temporal_kind(), None);
        assert_eq!((&Probe::<chrono::NaiveDate>::new()).__rullst_temporal_kind(), None);
        assert_eq!((&Probe::<Option<String>>::new()).__rullst_temporal_kind(), None);
        let _ = Probe::<String>::default();
    }
}
