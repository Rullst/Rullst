//! Field encoding for generated writes.
//!
//! SQLx's MySQL driver sends a `DateTime<Utc>` as a `TIMESTAMP` parameter.
//! MariaDB turns a `TIMESTAMP` value outside 1970-01-01 00:00:01 to
//! 2038-01-19 03:14:07 UTC into `NULL`, even when the column is the
//! `DATETIME(6)` that `timestamp_tz()` creates to hold such instants: a NOT
//! NULL column then rejects the row and a nullable column silently stores
//! `NULL`. With the MySQL/MariaDB pool, a generated write therefore sends the
//! instant's UTC wall time as a `NaiveDateTime`, which SQLx sends as a
//! `DATETIME` parameter. SQLx sets the session time zone to `+00:00`, so
//! MySQL stores the same value either way and reads decode it back to the
//! same `DateTime<Utc>`. Other pools receive the value unchanged.
//!
//! The field type is only known after macro expansion, so the generated code
//! selects the encoding by autoref dispatch:
//! `(&Field(&self.field)).__rullst_encode()` resolves to [`Instant`] for a
//! `DateTime<Utc>` or `Option<DateTime<Utc>>` field (under any path or type
//! alias) and to [`Unchanged`] for every other field type, which is bound as
//! a clone, as before.

use chrono::{DateTime, Utc};

/// What a UTC instant is bound as on this build's pool.
#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub type Wire = chrono::NaiveDateTime;
/// What a UTC instant is bound as on this build's pool.
#[cfg(not(all(feature = "strict-mysql", not(feature = "strict-postgres"))))]
pub type Wire = DateTime<Utc>;

/// The parameter for `value` on this build's pool.
#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub fn wire(value: DateTime<Utc>) -> Wire {
    value.naive_utc()
}
/// The parameter for `value` on this build's pool.
#[cfg(not(all(feature = "strict-mysql", not(feature = "strict-postgres"))))]
pub fn wire(value: DateTime<Utc>) -> Wire {
    value
}

/// Borrows one model field for autoref dispatch.
pub struct Field<'a, T>(pub &'a T);

/// Encodes a UTC instant for the pool ([`wire`]).
pub trait Instant {
    type Encoded;
    fn __rullst_encode(&self) -> Self::Encoded;
}

impl Instant for Field<'_, DateTime<Utc>> {
    type Encoded = Wire;
    fn __rullst_encode(&self) -> Wire {
        wire(*self.0)
    }
}

impl Instant for Field<'_, Option<DateTime<Utc>>> {
    type Encoded = Option<Wire>;
    fn __rullst_encode(&self) -> Option<Wire> {
        self.0.map(wire)
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
    use super::{Field, Instant as _, Unchanged as _, wire};
    use chrono::{DateTime, TimeZone, Utc};

    type Stamp = DateTime<Utc>;

    fn after_2038() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2040, 2, 29, 23, 59, 59)
            .single()
            .expect("valid instant")
    }

    // Generated code borrows every field the same way, whatever its type.
    #[allow(clippy::needless_borrow)]
    #[test]
    fn instants_take_the_pool_encoding_and_other_fields_pass_through() {
        let instant = after_2038();
        let aliased: Stamp = instant;
        let present = Some(instant);
        let missing: Option<DateTime<Utc>> = None;
        let text = String::from("title");
        let number = 7_i64;
        assert_eq!((&Field(&instant)).__rullst_encode(), wire(instant));
        assert_eq!((&Field(&aliased)).__rullst_encode(), wire(instant));
        assert_eq!((&Field(&present)).__rullst_encode(), Some(wire(instant)));
        assert_eq!((&Field(&missing)).__rullst_encode(), None);
        assert_eq!((&Field(&text)).__rullst_encode(), "title");
        assert_eq!((&Field(&number)).__rullst_encode(), 7);
    }

    /// The MySQL/MariaDB pool sends the UTC wall time (a `DATETIME`
    /// parameter); every other pool sends the instant itself.
    #[test]
    fn the_wire_value_is_the_utc_wall_time_on_mysql_and_the_instant_elsewhere() {
        let instant = after_2038();
        let encoded = format!("{:?}", wire(instant));
        if cfg!(all(
            feature = "strict-mysql",
            not(feature = "strict-postgres")
        )) {
            assert_eq!(encoded, format!("{:?}", instant.naive_utc()));
        } else {
            assert_eq!(encoded, format!("{instant:?}"));
        }
    }
}
