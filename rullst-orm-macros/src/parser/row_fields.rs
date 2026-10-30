//! How the model's generated `FromRow` reads each field.

/// One struct field in declaration order, as `FromRow` reads it.
pub struct RowField {
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub read: RowRead,
}

/// How `FromRow` fills a field, mirroring SQLx's `#[derive(FromRow)]` for the
/// options `#[derive(Orm)]` accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowRead {
    /// `#[sqlx(skip)]` fields and relations, which are never columns:
    /// `Default::default()` without reading the row.
    Default,
    /// A column named after the field. `json` decodes it through SQLx `Json`
    /// (`Some(true)` for `json(nullable)`); `default` fills a column missing
    /// from the row with `Default::default()`.
    Column { json: Option<bool>, default: bool },
}

impl RowRead {
    pub(super) fn of(skipped: bool, relation: bool, json: Option<bool>, default: bool) -> Self {
        if skipped || relation {
            Self::Default
        } else {
            Self::Column { json, default }
        }
    }
}
