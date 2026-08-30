use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn generate(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let tenant_prepare = tenant_prepare(parsed);
    let (policy_create, policy_update) = policy_checks(parsed);

    if !parsed.auditable {
        return quote! {
            #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
            pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
                let is_new = self.id == 0;
                #tenant_prepare
                if is_new {
                    #policy_create
                } else {
                    #policy_update
                }
                rullst_orm::dispatch_executor!(pool, |pool| self.save_with_tx_internal(pool).await)
            }

            pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
                let is_new = self.id == 0;
                #tenant_prepare
                if is_new {
                    #policy_create
                } else {
                    #policy_update
                }
                self.save_with_tx_internal(&mut **tx).await
            }
        };
    }

    let lookup = audit_lookup(parsed);
    let before_tx = audit_before_tx(&lookup);
    let after_tx = audit_after_tx(table_name);

    quote! {
        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
            let scoped_transaction = rullst_orm::CURRENT_TX
                .try_with(|transaction| transaction.clone())
                .ok();
            if let Some(transaction) = scoped_transaction {
                let mut transaction = transaction.lock().await;
                if let Some(tx) = transaction.as_mut() {
                    return self.save_with_tx(tx).await;
                }
            }

            let original_id = self.id;
            let mut transaction = rullst_orm::Orm::begin_transaction().await?;
            if let Err(save_error) = self.save_with_tx(&mut transaction).await {
                self.id = original_id;
                return match transaction.rollback().await {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "auditable save failed: {}; rollback also failed: {}",
                        save_error,
                        rollback_error,
                    ))),
                };
            }
            transaction.commit().await?;
            Ok(())
        }

        pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
            use rullst_orm::_sqlx::Acquire;
            let original_id = self.id;
            let mut savepoint = (&mut **tx).begin().await?;
            let save_result = async {
                let tx = &mut savepoint;
                let is_new = self.id == 0;
                #tenant_prepare
                if is_new {
                    #policy_create
                } else {
                    #policy_update
                }
                #before_tx
                self.save_with_tx_internal(&mut **tx).await?;
                #after_tx
                Ok::<(), rullst_orm::Error>(())
            }.await;
            if let Err(save_error) = save_result {
                self.id = original_id;
                return match savepoint.rollback().await {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "auditable save failed: {}; savepoint rollback also failed: {}",
                        save_error,
                        rollback_error,
                    ))),
                };
            }
            savepoint.commit().await?;
            Ok(())
        }
    }
}

fn tenant_prepare(parsed: &ParsedModel) -> TokenStream {
    if parsed.tenant_column.is_empty() {
        return quote! {};
    }

    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let column = syn::Ident::new(&parsed.tenant_column, name.span());
    quote! {
        let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(|| {
            rullst_orm::Error::Validation(format!(
                "tenant context is required to save `{}`",
                #table_name
            ))
        })?;
        self.#column = tenant.try_into().map_err(|_| {
            rullst_orm::Error::Validation(format!(
                "tenant context type does not match `{}.{}`",
                #table_name,
                stringify!(#column)
            ))
        })?;
    }
}

fn policy_checks(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    if parsed.policy.is_empty() {
        return (quote! {}, quote! {});
    }

    let policy = syn::Ident::new(&parsed.policy, parsed.name.span());
    (
        quote! {
            if !<#policy as rullst_orm::Policy<Self>>::can_create(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents creation of this record".to_string()));
            }
        },
        quote! {
            if !<#policy as rullst_orm::Policy<Self>>::can_update(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents updating this record".to_string()));
            }
        },
    )
}

fn audit_lookup(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    if parsed.tenant_column.is_empty() {
        return quote! {
            let query = if driver == "postgres" {
                format!("SELECT * FROM {} WHERE id = $1", #table_name)
            } else {
                format!("SELECT * FROM {} WHERE id = ?", #table_name)
            };
            let q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                .bind(self.id);
        };
    }

    let column = syn::Ident::new(&parsed.tenant_column, name.span());
    let column_name = &parsed.tenant_column;
    quote! {
        let query = if driver == "postgres" {
            format!("SELECT * FROM {} WHERE id = $1 AND {} = $2", #table_name, #column_name)
        } else {
            format!("SELECT * FROM {} WHERE id = ? AND {} = ?", #table_name, #column_name)
        };
        let q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
            .bind(self.id)
            .bind(self.#column.clone());
    }
}

fn audit_before_tx(lookup: &TokenStream) -> TokenStream {
    quote! {
        let mut old_model_for_audit = if !is_new {
            let driver = rullst_orm::Orm::driver()?;
            #lookup
            q.fetch_optional(&mut **tx).await?
        } else {
            None
        };
        if let Some(old_model) = old_model_for_audit.as_mut() {
            old_model.__rullst_decrypt_encrypted_fields()?;
        }
    }
}

fn audit_after_tx(table_name: &str) -> TokenStream {
    quote! {
        if is_new {
            rullst_orm::audit::log_audit_with_tx(
                tx,
                #table_name,
                self.id,
                "created",
                None,
                Some(self.to_json())
            ).await?;
        } else if let Some(old_model) = old_model_for_audit {
            rullst_orm::audit::log_audit_diff_with_tx(
                tx,
                #table_name,
                self.id,
                "updated",
                &old_model.to_json(),
                &self.to_json()
            ).await?;
        }
    }
}
