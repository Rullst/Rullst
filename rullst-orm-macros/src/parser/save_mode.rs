//! What `save()` writes to a stored row is an explicit choice of each model
//! (since 13.0.0-alpha.6): only the changed columns, through a `#[sqlx(skip)]`
//! field of type `RowSnapshot`, or every column, through
//! `#[orm(full_row_save)]`. A full-row update overwrites a column that another
//! request changed after this handle was read, so a model never gets it
//! without asking.
//!
//! The derive entry points run this check after [`super::parse`], which reads
//! the grammar only; the code generators assume a model without a
//! `RowSnapshot` field writes full rows.

use super::ParsedModel;

/// Fails unless the model chose exactly one save mode its backend supports:
/// SQLx models a `RowSnapshot` field or `#[orm(full_row_save)]`, Turso-primary
/// models (`turso`) `#[orm(full_row_save)]`, because they always write every
/// column.
pub fn check_save_mode(parsed: &ParsedModel, turso: bool) -> Result<(), syn::Error> {
    let name = &parsed.name;
    match (parsed.snapshot_field.as_ref(), parsed.full_row_save) {
        (Some(field), _) if turso => Err(syn::Error::new(
            field.span(),
            "Turso-primary models do not track changed columns: their `save()` writes every column. \
             Remove this `RowSnapshot` field and declare `#[orm(full_row_save)]` on the struct",
        )),
        (None, false) if turso => Err(syn::Error::new(
            name.span(),
            format!(
                "`{name}` is a Turso-primary model: its `save()` writes every column, so a change \
                 that another request made to another column is lost. Declare \
                 `#[orm(full_row_save)]` on the struct to accept that"
            ),
        )),
        (Some(field), true) => Err(syn::Error::new(
            field.span(),
            format!(
                "`{name}` declares both a `RowSnapshot` field and `#[orm(full_row_save)]`; keep one: \
                 the field makes `save()` write only the changed columns, the attribute every column"
            ),
        )),
        (None, false) => Err(syn::Error::new(
            name.span(),
            format!(
                "`{name}` must choose what `save()` writes to a stored row. Add one of these lines:\n\
                 \x20   #[sqlx(skip)] pub snapshot: rullst_orm::RowSnapshot,\n\
                 \x20       as a field: only the columns this record changed (recommended)\n\
                 \x20   #[orm(full_row_save)]\n\
                 \x20       on the struct: every column, so a change that another request made to \
                 another column is lost"
            ),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::check_save_mode;
    use syn::{DeriveInput, parse_quote};

    fn check(input: DeriveInput, turso: bool) -> Result<(), String> {
        let parsed = crate::parser::parse(&input).expect("model parses");
        check_save_mode(&parsed, turso).map_err(|error| error.to_string())
    }

    fn tracked() -> DeriveInput {
        parse_quote! {
            struct Profile { id: i32, name: String, #[sqlx(skip)] snapshot: rullst_orm::RowSnapshot }
        }
    }

    fn full_row() -> DeriveInput {
        parse_quote! {
            #[orm(table = "profiles", full_row_save)]
            struct Profile { id: i32, name: String }
        }
    }

    fn undecided() -> DeriveInput {
        parse_quote! { struct Profile { id: i32, name: String } }
    }

    fn both() -> DeriveInput {
        parse_quote! {
            #[orm(full_row_save)]
            struct Profile { id: i32, #[sqlx(skip)] snapshot: RowSnapshot }
        }
    }

    #[test]
    fn sqlx_models_accept_a_row_snapshot_or_full_row_saves() {
        assert_eq!(check(tracked(), false), Ok(()));
        assert_eq!(check(full_row(), false), Ok(()));
        let parsed = crate::parser::parse(&full_row()).expect("model parses");
        assert!(parsed.full_row_save);
        assert!(
            !crate::parser::parse(&tracked())
                .expect("parses")
                .full_row_save
        );
    }

    #[test]
    fn a_sqlx_model_without_a_choice_shows_both_one_line_fixes() {
        let error = check(undecided(), false).expect_err("no save mode");
        assert_eq!(
            error,
            "`Profile` must choose what `save()` writes to a stored row. Add one of these lines:\n    \
             #[sqlx(skip)] pub snapshot: rullst_orm::RowSnapshot,\n        as a field: only the \
             columns this record changed (recommended)\n    #[orm(full_row_save)]\n        on the \
             struct: every column, so a change that another request made to another column is lost"
        );
    }

    #[test]
    fn a_sqlx_model_cannot_choose_both() {
        assert_eq!(
            check(both(), false),
            Err(
                "`Profile` declares both a `RowSnapshot` field and `#[orm(full_row_save)]`; keep \
                 one: the field makes `save()` write only the changed columns, the attribute every \
                 column"
                    .to_string()
            )
        );
    }

    #[test]
    fn turso_models_declare_full_row_saves_and_no_row_snapshot() {
        assert_eq!(check(full_row(), true), Ok(()));
        let snapshot = "Turso-primary models do not track changed columns: their `save()` writes \
                        every column. Remove this `RowSnapshot` field and declare \
                        `#[orm(full_row_save)]` on the struct";
        assert_eq!(check(tracked(), true), Err(snapshot.to_string()));
        assert_eq!(check(both(), true), Err(snapshot.to_string()));
        assert_eq!(
            check(undecided(), true),
            Err(
                "`Profile` is a Turso-primary model: its `save()` writes every column, so a change \
                 that another request made to another column is lost. Declare \
                 `#[orm(full_row_save)]` on the struct to accept that"
                    .to_string()
            )
        );
    }

    #[test]
    fn full_row_save_is_a_bare_flag_given_once() {
        let duplicate: DeriveInput = parse_quote! {
            #[orm(full_row_save, full_row_save)]
            struct Profile { id: i32 }
        };
        assert!(crate::parser::parse(&duplicate).is_err());
        let valued: DeriveInput = parse_quote! {
            #[orm(full_row_save = true)]
            struct Profile { id: i32 }
        };
        assert!(crate::parser::parse(&valued).is_err());
    }
}
