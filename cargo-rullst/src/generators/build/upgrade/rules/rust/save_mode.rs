//! 13.0.0-alpha.6: every `#[derive(Orm)]` and `#[derive(TursoModel)]` model
//! chooses what `save()` writes to a stored row, a `RowSnapshot` field (only
//! the changed columns) or `#[orm(full_row_save)]` (every column).

use super::super::Rule;
use super::super::catalog::ORM_SAVE_MODE;
use super::{entries, has_key, line};

/// The line of the model's name when its save mode does not compile: a SQLx
/// model with neither or both choices, or a Turso-primary model (`turso`) with
/// a `RowSnapshot` field or without `#[orm(full_row_save)]`.
pub(super) fn finding(item: &syn::ItemStruct, turso: bool) -> Option<(&'static Rule, usize)> {
    let snapshot = item.fields.iter().any(|field| is_row_snapshot(&field.ty));
    let full_row = has_key(&entries(&item.attrs, "orm"), &["full_row_save"]);
    let compiles = if turso {
        full_row && !snapshot
    } else {
        snapshot != full_row
    };
    (!compiles).then(|| (&ORM_SAVE_MODE, line(item.ident.span())))
}

/// `RowSnapshot` by its last path segment, as `#[derive(Orm)]` matches it.
fn is_row_snapshot(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "RowSnapshot" && segment.arguments.is_none())
}
