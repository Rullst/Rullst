use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

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
    let setters = fields.iter().map(|(field, ty)| {
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

    let struct_def = quote! {
        /// Typed logical patch merged into the current row through the normal save lifecycle.
        pub struct #builder<'a> {
            model: &'a mut #name,
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
                    Err(error) => {
                        tx.rollback().await?;
                        return Err(error);
                    }
                };
                tx.commit().await?;
                *self.model = candidate;
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
                *self.model = candidate;
                callbacks.promote_to_parent().await
            }

            async fn __rullst_apply_partial(
                &self,
                tx: &mut rullst_orm::db::Transaction<'_>,
            ) -> Result<(#name, rullst_orm::post_commit::PostCommitScope), rullst_orm::Error> {
                use rullst_orm::_sqlx::Acquire;
                #tenant_guard
                if self.model.id == 0 { return Err(rullst_orm::Error::RecordNotFound); }
                let mut savepoint = (&mut **tx).begin().await?;
                let callbacks = rullst_orm::post_commit::PostCommitScope::new();
                let result = callbacks.run(async {
                    let driver = rullst_orm::Orm::driver()?;
                    let mut sql = format!("SELECT * FROM {} WHERE id = ?{}", #table_name, #tenant_clause);
                    if driver == "postgres" || driver == "mysql" { sql.push_str(" FOR UPDATE"); }
                    if driver == "postgres" { sql = rullst_orm::replace_placeholders(&sql); }
                    let query = rullst_orm::_sqlx::query_as::<_, #name>(
                        rullst_orm::_sqlx::AssertSqlSafe(sql.as_str())
                    ).bind(self.model.id) #tenant_bind;
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
                    #(#apply)*
                    candidate.save_with_tx(&mut savepoint).await?;
                    Ok::<_, rullst_orm::Error>(candidate)
                }).await;
                let candidate = match result {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        savepoint.rollback().await?;
                        return Err(error);
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
            #builder { model: self, #(#inits),* }
        }
    };
    (struct_def, method_def)
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
        if self.model.#column != expected {
            return Err(rullst_orm::Error::Validation("record is outside the active tenant scope".to_string()));
        }
    };
    (
        guard,
        format!(" AND {} = ?", parsed.tenant_column),
        quote! { .bind(self.model.#column.clone()) },
    )
}
