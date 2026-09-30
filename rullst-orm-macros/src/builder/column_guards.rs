//! Generated guards for columns that generated query-builder methods must not
//! filter, order, group or select: skipped fields, which the table does not
//! have, and randomized `#[orm(encrypted)]` ciphertext.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_column_guards(parsed: &ParsedModel) -> TokenStream {
    let skipped_columns = parsed.skipped_fields.iter().map(ToString::to_string);
    let encrypted_columns = parsed
        .encrypted_fields
        .iter()
        .map(|field| field.name.to_string());
    quote! {
        const SKIPPED_COLUMNS: &'static [&'static str] = &[#(#skipped_columns),*];
        const ENCRYPTED_COLUMNS: &'static [&'static str] = &[#(#encrypted_columns),*];

        fn is_skipped_column(column: &str) -> bool {
            let column = column.rsplit('.').next().unwrap_or(column);
            Self::SKIPPED_COLUMNS.iter().any(|c| *c == column)
        }

        fn reject_skipped_column(&mut self, column: &str) -> bool {
            let column = column.rsplit('.').next().unwrap_or(column);
            if Self::is_skipped_column(column) {
                self.errors.push(rullst_orm::Error::Validation(format!(
                    "column `{}` is declared with `#[orm(skip)]` / `#[sqlx(skip)]` and does not exist in the table; it must not be used in WHERE / ORDER BY / GROUP BY / SELECT",
                    column
                )));
                true
            } else if Self::ENCRYPTED_COLUMNS.iter().any(|candidate| *candidate == column) {
                self.errors.push(rullst_orm::Error::Validation(format!(
                    "column `{}` uses randomized `#[orm(encrypted)]` storage and cannot be used in WHERE / ORDER BY / GROUP BY / SELECT; query a separate blind-index column instead",
                    column
                )));
                true
            } else {
                false
            }
        }
    }
}
