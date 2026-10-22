//! Generated guards for columns that generated query-builder methods must not
//! filter, order, group or select: skipped fields, which are not database
//! columns for the ORM (`#[orm(db_default)]` declares a column the database
//! manages, which stays filterable), randomized `#[orm(encrypted)]` ciphertext
//! and `SecretString` envelopes. A `SecretString` column may still be selected, because its SQLx
//! codec decrypts it when the model is decoded. Unquoted SQL identifiers are
//! case-insensitive, so the guards compare column names ignoring ASCII case.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_column_guards(parsed: &ParsedModel) -> TokenStream {
    let skipped_columns = parsed.skipped_fields.iter().map(ToString::to_string);
    let encrypted_columns = parsed
        .encrypted_fields
        .iter()
        .map(|field| field.name.to_string());
    let secret_columns = parsed.secret_fields.iter().map(ToString::to_string);
    quote! {
        const SKIPPED_COLUMNS: &'static [&'static str] = &[#(#skipped_columns),*];
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
                self.errors.push(rullst_orm::Error::Validation(format!(
                    "column `{}` is declared `#[orm(skip)]` / `#[sqlx(skip)]`, so it is not a database column for the ORM and must not be used in WHERE / ORDER BY / GROUP BY / SELECT; declare a column that the database manages `#[orm(db_default)]` to read and filter it without writing it",
                    column
                )));
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
            "so it is not a database column for the ORM and must not be used in WHERE / ORDER BY / GROUP BY / SELECT; declare a column that the database manages `#[orm(db_default)]`"
        ));
    }
}
