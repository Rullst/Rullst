//! Dynamic runtime value encapsulation for SQL parameter binding and model mapping.

/// A value bound to a generated or raw query: filter values, `Orm::raw`
/// bindings and tenant IDs.
///
/// An integer literal stays `Int` (`i32`); `i64` and `u32` become `I64`, and
/// `Option<T>` becomes `T`'s value or `Null`. The enum is `#[non_exhaustive]`
/// since v13: match it with a wildcard arm.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum RullstValue {
    String(String),
    Int(i32),
    Float(f64),
    Bool(bool),
    /// A 64-bit integer, bound as `i64` (`BIGINT`). Added in v13.
    I64(i64),
    /// SQL `NULL`, from `None`. It is bound as a text parameter: SQLite and
    /// MySQL/MariaDB store it in any column, while PostgreSQL needs a cast for
    /// another column type (`CAST($1 AS BIGINT)`). A comparison with `NULL`
    /// matches no row, so filter with `where_null` instead. Added in v13.
    Null,
}

type QueryArguments = <crate::RullstDatabase as sqlx::Database>::Arguments;

/// Binds one [`RullstValue`] to a SQLx query. Generated query builders and
/// `Orm::raw` call it, so each variant is bound in this one place.
#[doc(hidden)]
pub trait BindRullstValue: Sized {
    fn bind_rullst_value(self, value: &RullstValue) -> Self;
}

macro_rules! bind_rullst_value {
    ($query:expr, $value:expr) => {
        match $value {
            RullstValue::String(value) => $query.bind(value.clone()),
            RullstValue::Int(value) => $query.bind(*value),
            RullstValue::Float(value) => $query.bind(*value),
            RullstValue::Bool(value) => $query.bind(*value),
            RullstValue::I64(value) => $query.bind(*value),
            RullstValue::Null => $query.bind(None::<String>),
        }
    };
}

impl BindRullstValue for sqlx::query::Query<'_, crate::RullstDatabase, QueryArguments> {
    fn bind_rullst_value(self, value: &RullstValue) -> Self {
        bind_rullst_value!(self, value)
    }
}

impl<O> BindRullstValue for sqlx::query::QueryAs<'_, crate::RullstDatabase, O, QueryArguments> {
    fn bind_rullst_value(self, value: &RullstValue) -> Self {
        bind_rullst_value!(self, value)
    }
}

impl From<&str> for RullstValue {
    fn from(s: &str) -> Self {
        RullstValue::String(s.to_string())
    }
}

impl From<String> for RullstValue {
    fn from(s: String) -> Self {
        RullstValue::String(s)
    }
}

impl From<i32> for RullstValue {
    fn from(i: i32) -> Self {
        RullstValue::Int(i)
    }
}

impl From<f64> for RullstValue {
    fn from(f: f64) -> Self {
        RullstValue::Float(f)
    }
}

impl From<bool> for RullstValue {
    fn from(b: bool) -> Self {
        RullstValue::Bool(b)
    }
}

impl From<i64> for RullstValue {
    fn from(value: i64) -> Self {
        RullstValue::I64(value)
    }
}

impl From<u32> for RullstValue {
    fn from(value: u32) -> Self {
        RullstValue::I64(i64::from(value))
    }
}

/// `Some(value)` converts like `value`; `None` is [`RullstValue::Null`].
impl<T: Into<RullstValue>> From<Option<T>> for RullstValue {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => RullstValue::Null,
        }
    }
}

impl TryFrom<RullstValue> for String {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::String(s) => Ok(s),
            _ => Err("Not a string"),
        }
    }
}

impl TryFrom<RullstValue> for i32 {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::Int(i) => Ok(i),
            _ => Err("Not an i32"),
        }
    }
}

/// Accepts `I64` and widens `Int`.
impl TryFrom<RullstValue> for i64 {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::I64(i) => Ok(i),
            RullstValue::Int(i) => Ok(i64::from(i)),
            _ => Err("Not an i64"),
        }
    }
}

impl TryFrom<RullstValue> for f64 {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::Float(f) => Ok(f),
            _ => Err("Not an f64"),
        }
    }
}

impl TryFrom<RullstValue> for bool {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::Bool(b) => Ok(b),
            _ => Err("Not a bool"),
        }
    }
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod tests;
