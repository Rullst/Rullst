use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Fixed public methods of the generated update builder. The builder's own
/// state uses the reserved `__rullst_` prefix, so any other column name can be
/// a patch field.
const UPDATE_BUILDER_METHODS: &[&str] = &["save", "save_with_tx"];

#[cfg_attr(test, mutants::skip)]
pub fn generate_update_builder(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let builder = quote::format_ident!("{}UpdateBuilder", name);
    let fields: Vec<_> = parsed
        .normal_fields
        .iter()
        .zip(&parsed.normal_fields_types)
        .filter(|(field, _)| *field != "id" && *field != parsed.tenant_column.as_str())
        .collect();
    let declarations = fields
        .iter()
        .map(|(field, ty)| quote! { #field: Option<#ty> });
    // A column named like a fixed builder method keeps its patch field but gets
    // no setter, which would redefine that method.
    let setters = fields
        .iter()
        .filter(|(field, _)| !UPDATE_BUILDER_METHODS.contains(&field.to_string().as_str()))
        .map(|(field, ty)| {
            quote! {
                pub fn #field(mut self, value: #ty) -> Self {
                    self.#field = Some(value);
                    self
                }
            }
        });
    let changes = fields
        .iter()
        .map(|(field, _)| quote! { || self.#field.is_some() });
    let apply = fields.iter().map(|(field, _)| {
        quote! {
            if let Some(value) = &self.#field {
                candidate.#field = value.clone();
            }
        }
    });
    let inits = fields.iter().map(|(field, _)| quote! { #field: None });
    let (tenant_guard, tenant_clause, tenant_bind) = tenant_scope(parsed);
    let after_fetch = after_fetch_hook(parsed, &quote::format_ident!("candidate"));
    // The setter stays for compatibility, but the soft-delete marker only
    // changes through delete()/restore()/force_delete().
    let soft_delete_guard = match parsed
        .soft_delete_column()
        .and_then(|column| fields.iter().find(|(field, _)| *field == column))
    {
        Some((field, _)) => quote! {
            if self.#field.is_some() {
                return Err(rullst_orm::Error::Validation(
                    "update_partial() cannot change the soft-delete column; use delete() or restore()".to_string()
                ));
            }
        },
        None => quote! {},
    };

    let struct_def = quote! {
        /// Typed logical patch merged into the current row through the normal save lifecycle.
        pub struct #builder<'a> {
            __rullst_model: &'a mut #name,
            #(#declarations),*
        }

        impl<'a> #builder<'a> {
            #(#setters)*

            fn __rullst_has_partial_changes(&self) -> bool { false #(#changes)* }

            /// Applies selected values to a freshly loaded row and performs a full-row save.
            /// Direct calls own the commit. A task-scoped transaction remains caller-owned;
            /// discard/reload the returned model if that enclosing transaction rolls back.
            pub async fn save(self) -> Result<(), rullst_orm::Error> {
                rullst_orm::__transaction_access::ensure_allowed()?;
                if !self.__rullst_has_partial_changes() { return Ok(()); }
                if let Ok(transaction) = rullst_orm::CURRENT_TX.try_with(Clone::clone) {
                    let mut guard = transaction.lock().await;
                    let tx = guard.as_mut().ok_or_else(|| rullst_orm::Error::Internal(
                        "partial update transaction is no longer available".to_string()
                    ))?;
                    return self.save_with_tx(tx).await;
                }
                let mut tx = rullst_orm::Orm::begin_transaction().await?;
                let (candidate, callbacks) = match self.__rullst_apply_partial(&mut tx).await {
                    Ok(result) => result,
                    // A failed rollback must not hide why the update failed.
                    Err(error) => {
                        return match tx.rollback().await {
                            Ok(()) => Err(error),
                            Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                                "partial update failed: {}; rollback also failed: {}",
                                error,
                                rollback_error,
                            ))),
                        };
                    }
                };
                tx.commit().await?;
                *self.__rullst_model = candidate;
                callbacks.commit().await
            }

            /// Applies the patch inside a savepoint of the supplied transaction.
            /// Strict post-commit effects require `Orm::transaction`; raw SQLx transactions
            /// cannot expose their later commit/rollback decision to this API.
            pub async fn save_with_tx(
                self,
                tx: &mut rullst_orm::db::Transaction<'_>,
            ) -> Result<(), rullst_orm::Error> {
                if !self.__rullst_has_partial_changes() { return Ok(()); }
                let (candidate, callbacks) = self.__rullst_apply_partial(tx).await?;
                *self.__rullst_model = candidate;
                callbacks.promote_to_parent().await
            }

            async fn __rullst_apply_partial(
                &self,
                tx: &mut rullst_orm::db::Transaction<'_>,
            ) -> Result<(#name, rullst_orm::post_commit::PostCommitScope), rullst_orm::Error> {
                use rullst_orm::_sqlx::Acquire;
                #soft_delete_guard
                #tenant_guard
                if self.__rullst_model.id == 0 { return Err(rullst_orm::Error::RecordNotFound); }
                let mut savepoint = (&mut **tx).begin().await?;
                let callbacks = rullst_orm::post_commit::PostCommitScope::new();
                let result = callbacks.run(async {
                    let driver = rullst_orm::Orm::driver()?;
                    let mut sql = format!("SELECT * FROM {} WHERE id = ?{}", #table_name, #tenant_clause);
                    if driver == "postgres" || driver == "mysql" { sql.push_str(" FOR UPDATE"); }
                    if driver == "postgres" { sql = rullst_orm::replace_placeholders(&sql); }
                    let query = rullst_orm::_sqlx::query_as::<_, #name>(
                        rullst_orm::_sqlx::AssertSqlSafe(sql.as_str())
                    ).bind(self.__rullst_model.id) #tenant_bind;
                    let row = if let Some(timeout) = rullst_orm::schema::get_query_timeout() {
                        tokio::time::timeout(timeout, query.fetch_optional(&mut *savepoint))
                            .await.map_err(|_| rullst_orm::Error::DatabaseError(
                                "Partial update lookup timed out".to_string()
                            ))??
                    } else {
                        query.fetch_optional(&mut *savepoint).await?
                    };
                    let mut candidate = row.ok_or(rullst_orm::Error::RecordNotFound)?;
                    candidate.__rullst_decrypt_encrypted_fields()?;
                    // The save lifecycle expects the representation every read returns.
                    #after_fetch
                    #(#apply)*
                    candidate.save_with_tx(&mut savepoint).await?;
                    Ok::<_, rullst_orm::Error>(candidate)
                }).await;
                let candidate = match result {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        return match savepoint.rollback().await {
                            Ok(()) => Err(error),
                            Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                                "partial update failed: {}; savepoint rollback also failed: {}",
                                error,
                                rollback_error,
                            ))),
                        };
                    }
                };
                savepoint.commit().await?;
                Ok((candidate, callbacks))
            }
        }
    };
    let method_def = quote! {
        /// Selects logical field changes; saving refreshes the row and runs its full lifecycle.
        pub fn update_partial(&mut self) -> #builder<'_> {
            #builder { __rullst_model: self, #(#inits),* }
        }
    };
    (struct_def, method_def)
}

/// Runs the model's `after_fetch` hook on a row loaded inside a mutation. The
/// transaction stays borrowed, so reentrant ORM access from the hook fails closed.
pub(super) fn after_fetch_hook(parsed: &ParsedModel, row: &syn::Ident) -> TokenStream {
    if parsed.after_fetch.is_empty() {
        return quote! {};
    }
    let method = syn::Ident::new(&parsed.after_fetch, parsed.name.span());
    quote! { rullst_orm::__transaction_access::run(#row.#method()).await?; }
}

fn tenant_scope(parsed: &ParsedModel) -> (TokenStream, String, TokenStream) {
    if parsed.tenant_column.is_empty() {
        return (quote! {}, String::new(), quote! {});
    }
    let column = syn::Ident::new(&parsed.tenant_column, parsed.name.span());
    let Some((_, ty)) = parsed
        .normal_fields
        .iter()
        .zip(&parsed.normal_fields_types)
        .find(|(field, _)| *field == parsed.tenant_column.as_str())
    else {
        // The structured model parser rejects this before code generation.
        return (
            quote! { compile_error!("partial update tenant column is missing"); },
            String::new(),
            quote! {},
        );
    };
    let guard = quote! {
        let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(||
            rullst_orm::Error::Validation("tenant context is required for partial update".to_string())
        )?;
        let expected: #ty = tenant.try_into().map_err(|_|
            rullst_orm::Error::Validation("partial update tenant context type mismatch".to_string())
        )?;
        if self.__rullst_model.#column != expected {
            return Err(rullst_orm::Error::Validation("record is outside the active tenant scope".to_string()));
        }
    };
    (
        guard,
        format!(" AND {} = ?", parsed.tenant_column),
        quote! { .bind(self.__rullst_model.#column.clone()) },
    )
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn columns_named_like_builder_members_still_compile() {
        let input: DeriveInput = parse_quote! {
            struct ChatLog { id: i32, model: String, save: String }
        };
        let parsed = crate::parser::parse(&input).expect("parse model");
        let (builder, method) = super::generate_update_builder(&parsed);
        let builder = builder.to_string();
        assert!(builder.contains("__rullst_model : & 'a mut ChatLog"));
        assert!(builder.contains("model : Option < String >"));
        assert!(builder.contains("pub fn model (mut self"));
        assert!(builder.contains("save : Option < String >"));
        assert!(!builder.contains("pub fn save (mut self"));
        assert!(method.to_string().contains("__rullst_model : self"));
    }

    #[test]
    fn failed_rollbacks_keep_the_partial_update_error() {
        let input: DeriveInput = parse_quote! {
            struct Lesson { id: i32, title: String }
        };
        let parsed = crate::parser::parse(&input).expect("parse model");
        let builder = super::generate_update_builder(&parsed).0.to_string();
        assert!(builder.contains("\"partial update failed: {}; rollback also failed: {}\""));
        assert!(
            builder.contains("\"partial update failed: {}; savepoint rollback also failed: {}\"")
        );
        assert!(!builder.contains("rollback () . await ?"));
    }
}
