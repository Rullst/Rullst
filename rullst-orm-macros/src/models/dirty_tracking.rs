//! Dirty tracking for models that declare a `#[sqlx(skip)]` field of type
//! `rullst_orm::RowSnapshot`: reads remember a fingerprint of every column
//! `save()` updates, and `save()` writes only the columns whose value changed.
//! Models without the field keep the full-row update.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// The columns a persisted `save()` writes, in declaration order: every
/// persisted field except `id`, `#[orm(db_default)]` fields and the
/// soft-delete marker.
pub(crate) fn update_columns(parsed: &ParsedModel) -> Vec<&syn::Ident> {
    let soft_delete_column = parsed.soft_delete_column();
    parsed
        .normal_fields
        .iter()
        .filter(|field| *field != "id")
        .filter(|field| !parsed.db_default_fields.contains(field))
        .filter(|field| soft_delete_column != Some(field.to_string().as_str()))
        .collect()
}

/// Columns that `save()` never writes but the database may change: they are
/// refreshed from the stored row after a tracked update.
fn database_managed_columns(parsed: &ParsedModel) -> Vec<&syn::Ident> {
    let soft_delete_column = parsed.soft_delete_column();
    parsed
        .normal_fields
        .iter()
        .filter(|field| {
            parsed.db_default_fields.contains(field)
                || soft_delete_column == Some(field.to_string().as_str())
        })
        .collect()
}

/// Whether a column holds a JSON document, compared by its canonical value:
/// `#[sqlx(json)]`, a `Json<T>` wrapper or a `serde_json::Value`.
fn is_json_document(parsed: &ParsedModel, column: &syn::Ident) -> bool {
    if parsed.json_fields.iter().any(|(field, _)| field == column) {
        return true;
    }
    parsed
        .normal_fields
        .iter()
        .zip(&parsed.normal_fields_types)
        .find(|(field, _)| *field == column)
        .is_some_and(|(_, ty)| crate::parser::is_json_document_type(ty))
}

/// Whether `save()` of an unchanged record may return before opening a
/// transaction: no `before_save` hook or policy could change or reject it.
fn has_fast_clean_save(parsed: &ParsedModel) -> bool {
    parsed.snapshot_field.is_some() && parsed.before_save.is_empty() && parsed.policy.is_empty()
}

/// The generated tracking API: fingerprints, capture, `dirty_columns()`,
/// `is_dirty()`, `mark_all_dirty()` and the clean-save fast path.
pub(crate) fn generate_methods(parsed: &ParsedModel, tenant_prepare: &TokenStream) -> TokenStream {
    let Some(snapshot) = parsed.snapshot_field.as_ref() else {
        return quote! {};
    };
    let columns = update_columns(parsed);
    let count = columns.len();
    let names = columns.iter().map(ToString::to_string);
    let fingerprints = columns.iter().map(|column| {
        if is_json_document(parsed, column) {
            quote! { rullst_orm::snapshot::fingerprint_json(&self.#column) }
        } else {
            quote! { rullst_orm::snapshot::fingerprint(&self.#column) }
        }
    });
    let fast_path = if has_fast_clean_save(parsed) {
        quote! {
            /// Whether `save()` has nothing to do: a tracked handle without a
            /// changed column and without observers that could change one.
            fn __rullst_save_is_noop(&mut self) -> Result<bool, rullst_orm::Error> {
                if self.id == 0 {
                    return Ok(false);
                }
                let observed = !Self::observers()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .is_empty();
                if observed {
                    return Ok(false);
                }
                #tenant_prepare
                Ok(!self.#snapshot.__any_changed(&self.__rullst_fingerprints()))
            }
        }
    } else {
        quote! {}
    };
    quote! {
        const __RULLST_TRACKED_COLUMNS: [&'static str; #count] = [#(#names),*];

        #[doc(hidden)]
        pub fn __rullst_fingerprints(&self) -> [u64; #count] {
            [#(#fingerprints),*]
        }

        #[doc(hidden)]
        pub fn __rullst_capture_snapshot(&mut self) {
            let fingerprints = self.__rullst_fingerprints();
            self.#snapshot = rullst_orm::RowSnapshot::__from_fingerprints(&fingerprints);
        }

        /// The columns that `save()` would write: those whose value differs
        /// from the one this handle was loaded or last saved with, or every
        /// column for a handle that was not loaded from the database.
        pub fn dirty_columns(&self) -> Vec<&'static str> {
            let fingerprints = self.__rullst_fingerprints();
            Self::__RULLST_TRACKED_COLUMNS
                .iter()
                .zip(fingerprints)
                .enumerate()
                .filter(|(index, (_, fingerprint))| self.#snapshot.__is_changed(*index, *fingerprint))
                .map(|(_, (column, _))| *column)
                .collect()
        }

        /// Whether `save()` has anything to write: always for a new record
        /// (`id == 0`), else when `dirty_columns()` is not empty.
        pub fn is_dirty(&self) -> bool {
            self.id == 0 || self.#snapshot.__any_changed(&self.__rullst_fingerprints())
        }

        /// Forgets the loaded values, so the next `save()` writes every column
        /// as a full-row update, overwriting concurrent changes.
        pub fn mark_all_dirty(&mut self) {
            self.#snapshot = rullst_orm::RowSnapshot::default();
        }

        #fast_path
    }
}

/// Records the fingerprints of a freshly read `target` model.
pub(crate) fn capture(parsed: &ParsedModel, target: &TokenStream) -> TokenStream {
    if parsed.snapshot_field.is_none() {
        return quote! {};
    }
    quote! { #target.__rullst_capture_snapshot(); }
}

/// `save()` / `save_with_tx()` prologue that returns for an unchanged record.
pub(crate) fn clean_save_return(parsed: &ParsedModel) -> TokenStream {
    if !has_fast_clean_save(parsed) {
        return quote! {};
    }
    quote! {
        if self.__rullst_save_is_noop()? {
            return Ok(());
        }
    }
}

/// Keeps the snapshot of a save that may fail and restores it on failure, so a
/// rolled-back write stays dirty.
pub(crate) fn snapshot_guard(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    let Some(snapshot) = parsed.snapshot_field.as_ref() else {
        return (quote! {}, quote! {});
    };
    (
        quote! { let original_snapshot = self.#snapshot.clone(); },
        quote! { self.#snapshot = original_snapshot; },
    )
}

/// The parts of the UPDATE statement a tracked save assembles.
pub(crate) struct TrackedUpdate<'a> {
    pub(crate) assignments: &'a [String],
    pub(crate) bindings: &'a [TokenStream],
    pub(crate) tenant_clause: &'a str,
    pub(crate) tenant_binding: &'a TokenStream,
    pub(crate) rows_check: &'a TokenStream,
}

/// The update branch of a tracked `save_with_tx_internal`: it returns
/// `Ok(false)` when nothing changed, otherwise writes the changed columns and
/// reloads the others from the stored row. Expects `observers` and `conn`.
pub(crate) fn tracked_update(parsed: &ParsedModel, update: &TrackedUpdate<'_>) -> TokenStream {
    let Some(snapshot) = parsed.snapshot_field.as_ref() else {
        return quote! {};
    };
    let columns = update_columns(parsed);
    if columns.is_empty() {
        // Only `id` and database-managed columns: nothing is ever written. A
        // bare `return` would make the `updated` observers after this branch
        // unreachable code in the application (`make:model` starts this way).
        return quote! {
            if !self.#snapshot.__any_changed(&[]) {
                return Ok(false);
            }
        };
    }
    let count = columns.len();
    let indexes: Vec<usize> = (0..count).collect();
    let assignments = update.assignments;
    let bindings = update.bindings;
    let tenant_clause = update.tenant_clause;
    let tenant_binding = update.tenant_binding;
    let rows_check = update.rows_check;
    let table_name = &parsed.table_name;
    let lookup_sql = format!("SELECT * FROM {table_name} WHERE id = ?{tenant_clause}");
    let missing_row = if parsed.tenant_column.is_empty() {
        quote! { return Err(rullst_orm::Error::RecordNotFound); }
    } else {
        quote! {
            return Err(rullst_orm::Error::Validation(
                "record is outside the active tenant scope".to_string()
            ));
        }
    };
    let managed = database_managed_columns(parsed);
    quote! {
        let mut fingerprints = self.__rullst_fingerprints();
        if !self.#snapshot.__any_changed(&fingerprints) {
            return Ok(false);
        }
        for obs in &observers {
            rullst_orm::__transaction_access::run(obs.updating(self)).await?;
        }
        if !observers.is_empty() {
            fingerprints = self.__rullst_fingerprints();
        }
        let written: [bool; #count] = ::core::array::from_fn(|index| {
            self.#snapshot.__is_changed(index, fingerprints[index])
        });
        let mut assignments: Vec<&'static str> = Vec::with_capacity(#count);
        #(
            if written[#indexes] {
                assignments.push(#assignments);
            }
        )*
        if assignments.is_empty() {
            return Ok(false);
        }
        let driver = rullst_orm::Orm::driver()?;
        // PostgreSQL and SQLite return the stored row; MySQL/MariaDB re-read it
        // under the row lock this UPDATE holds.
        let returning = driver != "mysql";
        let mut final_sql = format!(
            "UPDATE {} SET {} WHERE id = ?{}",
            #table_name,
            assignments.join(", "),
            #tenant_clause
        );
        if returning {
            final_sql.push_str(" RETURNING *");
        }
        if driver == "postgres" {
            final_sql = rullst_orm::replace_placeholders(&final_sql);
        }
        if rullst_orm::schema::is_query_log_enabled() {
            println!("[SQL Debug] {:?} | ID: {}", final_sql, self.id);
        }
        let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(final_sql.as_str()));
        #(
            if written[#indexes] {
                query = query #bindings;
            }
        )*
        let query = query.bind(self.id) #tenant_binding;
        let timeout = rullst_orm::schema::get_query_timeout();
        let stored: Option<Self> = if returning {
            let row = if let Some(t) = timeout {
                rullst_orm::_tokio::time::timeout(t, query.fetch_optional(&mut *conn))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
            } else {
                query.fetch_optional(&mut *conn).await?
            };
            match row {
                Some(row) => Some(rullst_orm::_sqlx::FromRow::from_row(&row)?),
                None => None,
            }
        } else {
            let update_result = if let Some(t) = timeout {
                rullst_orm::_tokio::time::timeout(t, query.execute(&mut *conn))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
            } else {
                query.execute(&mut *conn).await?
            };
            #rows_check
            let lookup = rullst_orm::_sqlx::query_as::<_, Self>(
                rullst_orm::_sqlx::AssertSqlSafe(#lookup_sql)
            ).bind(self.id) #tenant_binding;
            if let Some(t) = timeout {
                rullst_orm::_tokio::time::timeout(t, lookup.fetch_optional(&mut *conn))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
            } else {
                lookup.fetch_optional(&mut *conn).await?
            }
        };
        let Some(mut stored) = stored else {
            #missing_row
        };
        stored.__rullst_decrypt_encrypted_fields()?;
        // Columns this save did not write take their stored values, which
        // another writer may have changed since this handle was read.
        #(
            if !written[#indexes] {
                ::core::mem::swap(&mut self.#columns, &mut stored.#columns);
            }
        )*
        #(
            ::core::mem::swap(&mut self.#managed, &mut stored.#managed);
        )*
        self.__rullst_capture_snapshot();
    }
}

#[cfg(test)]
#[path = "dirty_tracking_tests.rs"]
mod tests;
