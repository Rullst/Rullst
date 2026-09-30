//! Detects values that `serde_json` would encode ambiguously.
//!
//! `serde_json` writes NaN and infinite floats as `null`, and it writes
//! `None`, `()` and unit structs as `null` too, so `Some(())`, `Some(None)`
//! and `Some(f64::NAN)` all read back as `None`. `#[memoize]` uses this probe
//! to run such calls uncached instead of sharing one cache key between
//! different arguments or returning a changed cached result.

use serde::ser::{self, Serialize, Serializer};
use std::fmt;

/// True when `serde_json` encodes `value` without conflating it with another
/// value of the same type. Support for the `#[memoize]` expansion only.
pub fn is_exact_json<T: Serialize + ?Sized>(value: &T) -> bool {
    value.serialize(Probe { in_some: false }).is_ok()
}

#[derive(Debug)]
struct Inexact;

impl fmt::Display for Inexact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("value has no unambiguous JSON encoding")
    }
}

impl std::error::Error for Inexact {}

impl ser::Error for Inexact {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Inexact
    }
}

/// Walks a value without producing output. `in_some` is set directly inside
/// `Some(..)` (or a newtype there), where a JSON `null` would read as `None`.
#[derive(Clone, Copy)]
struct Probe {
    in_some: bool,
}

type Checked = Result<(), Inexact>;

impl Probe {
    const NESTED: Self = Self { in_some: false };

    fn null(self) -> Checked {
        if self.in_some { Err(Inexact) } else { Ok(()) }
    }

    fn float(value: f64) -> Checked {
        if value.is_finite() {
            Ok(())
        } else {
            Err(Inexact)
        }
    }
}

macro_rules! exact_scalars {
    ($($method:ident: $ty:ty),* $(,)?) => {
        $(fn $method(self, _value: $ty) -> Checked { Ok(()) })*
    };
}

impl Serializer for Probe {
    type Ok = ();
    type Error = Inexact;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    exact_scalars!(
        serialize_bool: bool,
        serialize_i8: i8,
        serialize_i16: i16,
        serialize_i32: i32,
        serialize_i64: i64,
        serialize_i128: i128,
        serialize_u8: u8,
        serialize_u16: u16,
        serialize_u32: u32,
        serialize_u64: u64,
        serialize_u128: u128,
        serialize_char: char,
        serialize_str: &str,
        serialize_bytes: &[u8],
    );

    fn serialize_f32(self, value: f32) -> Checked {
        Self::float(f64::from(value))
    }

    fn serialize_f64(self, value: f64) -> Checked {
        Self::float(value)
    }

    fn serialize_none(self) -> Checked {
        self.null()
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Checked {
        value.serialize(Self { in_some: true })
    }

    fn serialize_unit(self) -> Checked {
        self.null()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Checked {
        self.null()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
    ) -> Checked {
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Checked {
        // Newtype structs are transparent in JSON.
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Checked {
        value.serialize(Self::NESTED)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self, Inexact> {
        Ok(Self::NESTED)
    }
}

macro_rules! nested_elements {
    ($($trait:ident :: $method:ident),* $(,)?) => {
        $(impl ser::$trait for Probe {
            type Ok = ();
            type Error = Inexact;

            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Checked {
                value.serialize(Self::NESTED)
            }

            fn end(self) -> Checked {
                Ok(())
            }
        })*
    };
}

nested_elements!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field,
);

macro_rules! nested_fields {
    ($($trait:ident),* $(,)?) => {
        $(impl ser::$trait for Probe {
            type Ok = ();
            type Error = Inexact;

            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                _key: &'static str,
                value: &T,
            ) -> Checked {
                value.serialize(Self::NESTED)
            }

            fn end(self) -> Checked {
                Ok(())
            }
        })*
    };
}

nested_fields!(SerializeStruct, SerializeStructVariant);

impl ser::SerializeMap for Probe {
    type Ok = ();
    type Error = Inexact;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Checked {
        key.serialize(Self::NESTED)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Checked {
        value.serialize(Self::NESTED)
    }

    fn end(self) -> Checked {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::is_exact_json;
    use std::collections::BTreeMap;

    #[derive(serde::Serialize)]
    struct Unit;

    #[derive(serde::Serialize)]
    struct Reading {
        value: f32,
        note: Option<()>,
    }

    #[derive(serde::Serialize)]
    struct Wrapper(Option<i32>);

    #[test]
    fn values_that_json_would_conflate_are_detected() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(!is_exact_json(&value));
            assert!(!is_exact_json(&Some(value)));
            assert!(!is_exact_json(&vec![1.0, value]));
        }
        assert!(!is_exact_json(&f32::NAN));
        assert!(!is_exact_json(&Some(())));
        assert!(!is_exact_json(&Some(None::<i32>)));
        assert!(!is_exact_json(&Some(Unit)));
        assert!(!is_exact_json(&Some(Wrapper(None))));
        assert!(!is_exact_json(&Reading {
            value: f32::INFINITY,
            note: None,
        }));
        assert!(!is_exact_json(&Reading {
            value: 1.0,
            note: Some(()),
        }));
        assert!(!is_exact_json(&BTreeMap::from([("a", f64::NAN)])));
    }

    #[test]
    fn ordinary_values_are_exact() {
        assert!(is_exact_json(&1.5_f64));
        assert!(is_exact_json(&-0.0_f64));
        assert!(is_exact_json(&None::<f64>));
        assert!(is_exact_json(&()));
        assert!(is_exact_json(&Unit));
        assert!(is_exact_json(&Some(Some(3))));
        assert!(is_exact_json(&Some(vec![None::<i32>])));
        assert!(is_exact_json(&Wrapper(None)));
        assert!(is_exact_json(&Reading {
            value: 2.0,
            note: None,
        }));
        assert!(is_exact_json(&("text", u128::MAX, 'c')));
        assert!(is_exact_json(&BTreeMap::from([("a", 1.0)])));
    }
}
