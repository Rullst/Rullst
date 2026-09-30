//! Token fragments shared by generated instance mutations: `delete()`,
//! `force_delete()` and `restore()`.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Rejects a model handle whose tenant column differs from the active tenant.
pub(super) fn tenant_guard(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let Some(tenant_field_type) = parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
        .find(|(field, _)| *field == parsed.tenant_column.as_str())
        .map(|(_, ty)| ty)
    else {
        return quote! {};
    };
    let col_ident = syn::Ident::new(&parsed.tenant_column, parsed.name.span());
    quote! {
        let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(|| {
            rullst_orm::Error::Validation(format!(
                "tenant context is required to mutate `{}`",
                #table_name
            ))
        })?;
        let expected_tenant: #tenant_field_type = tenant.try_into().map_err(|_| {
            rullst_orm::Error::Validation(format!(
                "tenant context type does not match `{}.{}`",
                #table_name,
                stringify!(#col_ident)
            ))
        })?;
        if self.#col_ident != expected_tenant {
            return Err(rullst_orm::Error::Validation(
                "record is outside the active tenant scope".to_string()
            ));
        }
    }
}

/// The tenant predicate of a by-ID mutation: its SQL suffix, its binding and
/// the check that exactly one row in the tenant was affected.
pub(super) struct TenantPredicate {
    pub(super) clause: String,
    pub(super) binding: TokenStream,
    pub(super) rows_check: TokenStream,
}

pub(super) fn tenant_predicate(parsed: &ParsedModel) -> TenantPredicate {
    if parsed.tenant_column.is_empty() {
        return TenantPredicate {
            clause: String::new(),
            binding: quote! {},
            rows_check: quote! {},
        };
    }
    let col_ident = syn::Ident::new(&parsed.tenant_column, parsed.name.span());
    TenantPredicate {
        clause: format!(" AND {} = ?", parsed.tenant_column),
        binding: quote! { .bind(self.#col_ident.clone()) },
        rows_check: quote! {
            if mutation_result.rows_affected() != 1 {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
        },
    }
}

/// Awaits a configured model hook while the mutation borrows its executor.
pub(super) fn instance_hook(parsed: &ParsedModel, method: &str) -> TokenStream {
    if method.is_empty() {
        return quote! {};
    }
    let method = syn::Ident::new(method, parsed.name.span());
    quote! { rullst_orm::__transaction_access::run(self.#method()).await?; }
}

/// Registers the post-commit effects of a removed row: query-cache
/// invalidation and the Redis `deleted` event, `committed(Deleted)` observer
/// callbacks and Scout removal. Expects an `observers` binding in scope.
pub(super) fn deleted_effects(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let redis_cfg = crate::feature_gates::redis();
    let scout_delete = if parsed.searchable {
        quote! {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                rullst_orm::ModelOperation::Deleted,
                self.to_json(),
            );
            rullst_orm::after_commit(move || async move {
                if let Some(engine) = rullst_orm::scout::get_search_engine() {
                    engine.delete(event.table, event.id).await?;
                }
                Ok(())
            }).await?;
        }
    } else {
        quote! {}
    };
    quote! {
        #redis_cfg
        {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                rullst_orm::ModelOperation::Deleted,
                self.to_json(),
            );
            rullst_orm::after_commit(move || async move {
                use rullst_orm::_redis::AsyncCommands;
                // A failed invalidation must not suppress the event.
                let invalidated = rullst_orm::query_cache::invalidate_table(event.table).await;
                if let Ok(mut connection) = rullst_orm::Orm::redis_manager() {
                    let topic = format!("orm:events:{}:deleted", event.table);
                    let _: usize = connection.publish(&topic, &event.payload).await?;
                }
                invalidated?;
                Ok(())
            }).await?;
        }
        let event = rullst_orm::ModelCommittedEvent::new(
            #table_name,
            self.id,
            rullst_orm::ModelOperation::Deleted,
            self.to_json(),
        );
        rullst_orm::after_commit(move || async move {
            let futures = observers.iter().map(|observer| observer.committed(&event));
            rullst_orm::_futures::future::try_join_all(futures).await?;
            Ok(())
        }).await?;
        #scout_delete
    }
}

/// Registers the post-commit query-cache invalidation and the Redis
/// `<operation>`/`saved` events of a saved row. Expects `operation` in scope.
pub(super) fn saved_redis_effects(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let redis_cfg = crate::feature_gates::redis();
    quote! {
        #redis_cfg
        {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                operation,
                self.to_json(),
            );
            rullst_orm::after_commit(move || async move {
                use rullst_orm::_redis::AsyncCommands;
                // A failed invalidation must not suppress the events.
                let invalidated = rullst_orm::query_cache::invalidate_table(event.table).await;
                if let Ok(mut connection) = rullst_orm::Orm::redis_manager() {
                    let topic = format!(
                        "orm:events:{}:{}",
                        event.table,
                        event.operation.as_str(),
                    );
                    let _: usize = connection.publish(&topic, &event.payload).await?;
                    let topic = format!("orm:events:{}:saved", event.table);
                    let _: usize = connection.publish(&topic, &event.payload).await?;
                }
                invalidated?;
                Ok(())
            }).await?;
        }
    }
}

/// Executes the `exec` query on the current savepoint (`tx`) under the
/// configured query timeout and binds its result to `mutation_result`.
pub(super) fn execute_mutation() -> TokenStream {
    quote! {
        let timeout = rullst_orm::schema::get_query_timeout();
        let mutation_result = if let Some(t) = timeout {
            tokio::time::timeout(t, exec.execute(&mut **tx))
                .await
                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
        } else {
            exec.execute(&mut **tx).await?
        };
    }
}
