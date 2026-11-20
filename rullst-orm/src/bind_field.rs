//! Field encoding for generated writes.
//!
//! SQLx's MySQL driver sends a UTC instant with the `TIMESTAMP` parameter type.
//! MariaDB turns a `TIMESTAMP` value outside 1970-01-01 00:00:01 to
//! 2038-01-19 03:14:07 UTC into `NULL`, even when the column is the
//! `DATETIME(6)` that `timestamp_tz()` creates to hold such instants: a NOT
//! NULL column then rejects the row and a nullable column silently stores
//! `NULL`. With the MySQL/MariaDB pool, a generated write therefore sends a
//! [`crate::Timestamp`]'s UTC wall time as a [`crate::DateTime`], which is sent
//! as a `DATETIME` parameter. SQLx sets the session time zone to `+00:00`, so
//! MySQL stores the same value either way and reads decode it back to the same
//! instant. Other pools receive the value unchanged.
//!
//! The field type is only known after macro expansion, so the generated code
//! selects the encoding by autoref dispatch:
//! `(&Field(&self.field)).__rullst_encode()` resolves to [`Instant`] for a
//! `Timestamp` or `Option<Timestamp>` field (under any path or type alias) and
//! to [`Unchanged`] for every other field type, which is bound as a clone.

/// What a [`crate::Timestamp`] is bound as on this build's pool.
#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub type TimestampWire = crate::DateTime;
/// What a [`crate::Timestamp`] is bound as on this build's pool.
#[cfg(not(all(feature = "strict-mysql", not(feature = "strict-postgres"))))]
pub type TimestampWire = crate::Timestamp;

/// The parameter for `value` on this build's pool.
#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub fn timestamp_wire(value: crate::Timestamp) -> TimestampWire {
    crate::DateTime::from(crate::temporal::text::utc(*value))
}
/// The parameter for `value` on this build's pool.
#[cfg(not(all(feature = "strict-mysql", not(feature = "strict-postgres"))))]
pub fn timestamp_wire(value: crate::Timestamp) -> TimestampWire {
    value
}

/// Borrows one model field for autoref dispatch.
pub struct Field<'a, T>(pub &'a T);

/// Encodes a UTC instant for the pool ([`timestamp_wire`]).
pub trait Instant {
    type Encoded;
    fn __rullst_encode(&self) -> Self::Encoded;
}

impl Instant for Field<'_, crate::Timestamp> {
    type Encoded = TimestampWire;
    fn __rullst_encode(&self) -> TimestampWire {
        timestamp_wire(*self.0)
    }
}

impl Instant for Field<'_, Option<crate::Timestamp>> {
    type Encoded = Option<TimestampWire>;
    fn __rullst_encode(&self) -> Option<TimestampWire> {
        self.0.map(timestamp_wire)
    }
}

/// Binds any other field as a clone of its value.
pub trait Unchanged {
    type Encoded;
    fn __rullst_encode(&self) -> Self::Encoded;
}

impl<T: Clone> Unchanged for &Field<'_, T> {
    type Encoded = T;
    fn __rullst_encode(&self) -> T {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{Field, Instant as _, Unchanged as _, timestamp_wire};

    type Stamp = crate::Timestamp;

    // Generated code borrows every field the same way, whatever its type.
    #[allow(clippy::needless_borrow)]
    #[test]
    fn instants_take_the_pool_encoding_and_other_fields_pass_through() {
        let instant: crate::Timestamp = "2040-02-29T23:59:59Z".parse().expect("valid instant");
        let aliased: Stamp = instant;
        let wall: crate::DateTime = "2040-02-29T23:59:59".parse().expect("valid date-time");
        let present = Some(instant);
        let missing: Option<crate::Timestamp> = None;
        // The MySQL/MariaDB pool sends the UTC wall time (a `DATETIME`
        // parameter); every other pool sends the instant itself.
        let expected = if cfg!(all(
            feature = "strict-mysql",
            not(feature = "strict-postgres")
        )) {
            format!("{wall:?}")
        } else {
            format!("{instant:?}")
        };
        assert_eq!(format!("{:?}", timestamp_wire(instant)), expected);
        assert_eq!(
            format!("{:?}", (&Field(&instant)).__rullst_encode()),
            expected
        );
        assert_eq!(
            format!("{:?}", (&Field(&aliased)).__rullst_encode()),
            expected
        );
        assert_eq!(
            format!("{:?}", (&Field(&present)).__rullst_encode()),
            format!("Some({expected})")
        );
        assert_eq!(
            format!("{:?}", (&Field(&missing)).__rullst_encode()),
            "None"
        );
        // Every other field, the civil types included, is bound unchanged.
        assert_eq!((&Field(&wall)).__rullst_encode(), wall);
        assert_eq!((&Field(&String::from("title"))).__rullst_encode(), "title");
        assert_eq!((&Field(&7_i64)).__rullst_encode(), 7);
    }
}
