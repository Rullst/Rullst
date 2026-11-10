//! Generated guards for columns that generated query-builder methods must not
//! filter, order, group or select: skipped fields, which are not database
//! columns for the ORM (`#[orm(db_default)]` declares a column the database
//! manages, which stays filterable), randomized `#[orm(encrypted)]` ciphertext
//! and `SecretString` envelopes. A `SecretString` column may still be selected, because its SQLx
//! codec decrypts it when the model is decoded. Unquoted SQL identifiers are
//! case-insensitive, so the guards compare column names ignoring ASCII case.
//!
//! The error for a skipped field names its option: `#[orm(skip)]` leaves the
//! field out of generated writes and of the builder but not out of the
//! model's reads, while `#[sqlx(skip)]` also stops `FromRow` from reading it.

use crate::parser::{ParsedModel, RowRead};
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_column_guards(parsed: &ParsedModel) -> TokenStream {
    let skipped_columns = parsed.skipped_fields.iter().map(ToString::to_string);
    // Skipped fields that `FromRow` does not read either (`#[sqlx(skip)]`).
    let unread_columns = parsed
        .row_fields
        .iter()
        .filter(|field| {
            field.read == RowRead::Default && parsed.skipped_fields.contains(&field.name)
        })
        .map(|field| field.name.to_string());
    let encrypted_columns = parsed
        .encrypted_fields
        .iter()
        .map(|field| field.name.to_string());
    let secret_columns = parsed.secret_fields.iter().map(ToString::to_string);
    quote! {
        const SKIPPED_COLUMNS: &'static [&'static str] = &[#(#skipped_columns),*];
        const UNREAD_SKIPPED_COLUMNS: &'static [&'static str] = &[#(#unread_columns),*];
        const ENCRYPTED_COLUMNS: &'static [&'static str] = &[#(#encrypted_columns),*];
        const SECRET_COLUMNS: &'static [&'static str] = &[#(#secret_columns),*];

        fn is_skipped_column(column: &str) -> bool {
            let column = column.rsplit('.').next().unwrap_or(column);
            Self::SKIPPED_COLUMNS.iter().any(|c| c.eq_ignore_ascii_case(column))
        }

        /// The declared name of an `#[orm(encrypted)]` column; its envelopes
        /// are bound to that name.
        fn encrypted_column(column: &str) -> Option<&'static str> {
            let column = column.rsplit('.').next().unwrap_or(column);
            Self::ENCRYPTED_COLUMNS
                .iter()
                .copied()
                .find(|candidate| candidate.eq_ignore_ascii_case(column))
        }

        fn is_secret_column(column: &str) -> bool {
            let column = column.rsplit('.').next().unwrap_or(column);
            Self::SECRET_COLUMNS.iter().any(|c| c.eq_ignore_ascii_case(column))
        }

        /// Guards WHERE / ORDER BY / GROUP BY columns.
        fn reject_skipped_column(&mut self, column: &str) -> bool {
            self.__rullst_reject_column(column, false)
        }

        /// Guards an explicitly selected column.
        fn __rullst_reject_selected_column(&mut self, column: &str) -> bool {
            self.__rullst_reject_column(column, true)
        }

        fn __rullst_reject_column(&mut self, column: &str, selected: bool) -> bool {
            let column = column.rsplit('.').next().unwrap_or(column);
            if Self::is_skipped_column(column) {
                let unread = Self::UNREAD_SKIPPED_COLUMNS.iter().any(|c| c.eq_ignore_ascii_case(column));
                let message = if unread {
                    format!(
                        "column `{}` is declared `#[sqlx(skip)]`, so the model does not read it from the database and the query builder cannot use it in WHERE / ORDER BY / GROUP BY / SELECT; if the table has this column and it must be filterable, declare it `#[orm(db_default)]` instead: it is read and filtered, and ORM writes leave it to the database",
                        column
                    )
                } else {
                    format!(
                        "column `{}` is declared `#[orm(skip)]`, which leaves it out of generated writes and of the query builder but not out of the model's reads, so it cannot be used in WHERE / ORDER BY / GROUP BY / SELECT; to filter a column that the table has, declare it `#[orm(db_default)]` instead: it is read and filtered, and ORM writes leave it to the database",
                        column
                    )
                };
                self.errors.push(rullst_orm::Error::Validation(message));
                true
            } else if Self::encrypted_column(column).is_some() {
                self.errors.push(rullst_orm::Error::Validation(format!(
                    "column `{}` uses randomized `#[orm(encrypted)]` storage and cannot be used in WHERE / ORDER BY / GROUP BY / SELECT; query a separate blind-index column instead",
                    column
                )));
                true
            } else if !selected && Self::is_secret_column(column) {
                self.errors.push(rullst_orm::Error::Validation(format!(
                    "column `{}` stores a randomized `SecretString` envelope and cannot be used in WHERE / ORDER BY / GROUP BY; query a separate blind-index column instead",
                    column
                )));
                true
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_column_guards;

    #[test]
    fn skipped_fields_are_guarded_and_db_default_columns_stay_filterable() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Member {
                id: i32,
                #[orm(db_default)]
                is_admin: bool,
                #[orm(skip)]
                display_badge: String,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let generated = generate_column_guards(&parsed).to_string();
        assert!(generated.contains("SKIPPED_COLUMNS"), "{generated}");
        assert!(generated.contains("\"display_badge\""), "{generated}");
        assert!(!generated.contains("\"is_admin\""), "{generated}");
        assert!(generated.contains(
            "which leaves it out of generated writes and of the query builder but not out of the model's reads"
        ));
        assert!(generated.contains("declare it `#[orm(db_default)]` instead"));
    }

    #[test]
    fn only_sqlx_skipped_fields_are_listed_as_unread() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Member {
                id: i32,
                #[orm(skip)]
                member_status: String,
                #[sqlx(skip)]
                cache: String,
                #[orm(skip)]
                #[sqlx(skip)]
                draft: String,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let generated = generate_column_guards(&parsed).to_string();
        let unread = generated
            .split("UNREAD_SKIPPED_COLUMNS : & 'static [& 'static str] = & [")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("the unread list is generated");
        assert_eq!(unread.trim(), "\"cache\" , \"draft\"", "{generated}");
    }
}
