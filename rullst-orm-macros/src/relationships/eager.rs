//! Eager relationship loads generated inside the builder's `get()`: one
//! query per relation for the whole parent batch.

use super::RelationNames;
use super::eager_assign::generate_eager_load_assignment;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub(super) fn generate(name: &syn::Ident, names: &RelationNames<'_>) -> TokenStream {
    let RelationNames {
        rel,
        method_name,
        rel_model_ident,
        fk_ident,
        lk_ident,
        pk_ident,
        morph_id_ident,
        morph_type_ident,
        ..
    } = names;
    let rel_type = &rel.rel_type;
    let foreign_key = &rel.foreign_key;
    let related_key = &rel.related_key;
    let pivot_table = &rel.pivot_table;
    let load_flag = quote::format_ident!("load_{}", rel.field_name);
    let filter_flag = quote::format_ident!("filter_{}", rel.field_name);

    // A parent or related row whose key is `None` (a nullable key) takes part
    // in no match, like SQL `NULL`.
    let m_lk = super::relation_key(quote!(m.#lk_ident));
    let m_fk = super::relation_key(quote!(m.#fk_ident));
    let model_lk = super::relation_key(quote!(model.#lk_ident));
    let related_pk = super::relation_key(quote!(related.#pk_ident));

    // One query serves every parent: exceeding the row cap fails instead of dropping rows.
    let guarded_fetch = quote! {
        let eager_limit = rullst_orm::__eager_limit::guard(&mut query.limit);
        let all_related = Box::pin(query.get()).await?;
        rullst_orm::__eager_limit::ensure_complete(all_related.len(), eager_limit, stringify!(#name), stringify!(#method_name))?;
    };

    let eager_load_assignment = match rel_type.as_str() {
        "has_many" => generate_eager_load_assignment(name, true, fk_ident, lk_ident, method_name),
        "has_one" => generate_eager_load_assignment(name, false, fk_ident, lk_ident, method_name),
        "belongs_to" => {
            generate_eager_load_assignment(name, false, pk_ident, fk_ident, method_name)
        }
        "morph_many" => {
            generate_eager_load_assignment(name, true, morph_id_ident, lk_ident, method_name)
        }
        "morph_one" => {
            generate_eager_load_assignment(name, false, morph_id_ident, lk_ident, method_name)
        }
        _ => proc_macro2::TokenStream::new(),
    };

    if rel_type == "has_many" || rel_type == "has_one" {
        quote! {
            if self.#load_flag {
                let parent_ids: Vec<_> = results.iter().filter_map(|m| #m_lk).collect();
                let all_related: Vec<#rel_model_ident> = if parent_ids.is_empty() {
                    Vec::new()
                } else {
                    let mut query = #rel_model_ident::query().where_in(stringify!(#fk_ident), parent_ids).__rullst_freeze_scope();
                    if let Some(ref filter) = self.#filter_flag {
                        query = filter(query);
                    }
                    #guarded_fetch
                    all_related
                };
                #eager_load_assignment
            }
        }
    } else if rel_type == "belongs_to" {
        quote! {
            if self.#load_flag {
                let parent_ids: Vec<_> = results.iter().filter_map(|m| #m_fk).collect();
                let all_related: Vec<#rel_model_ident> = if parent_ids.is_empty() {
                    Vec::new()
                } else {
                    let mut query = #rel_model_ident::query().where_in(stringify!(#pk_ident), parent_ids).__rullst_freeze_scope();
                    if let Some(ref filter) = self.#filter_flag {
                        query = filter(query);
                    }
                    #guarded_fetch
                    all_related
                };
                #eager_load_assignment
            }
        }
    } else if rel_type == "morph_to" {
        quote! {
            if self.#load_flag {
                let target_ids: Vec<_> = results
                    .iter()
                    .filter(|model| model.#morph_type_ident == stringify!(#rel_model_ident))
                    .map(|model| model.#morph_id_ident.clone())
                    .collect();
                if !target_ids.is_empty() {
                    let mut query = #rel_model_ident::query()
                        .where_in(stringify!(#pk_ident), target_ids)
                        .__rullst_freeze_scope();
                    if let Some(ref filter) = self.#filter_flag {
                        query = filter(query);
                    }
                    #guarded_fetch
                    let related_by_id: std::collections::HashMap<_, _> = all_related
                        .into_iter()
                        .filter_map(|related| {
                            let key = #related_pk?;
                            Some((key, related))
                        })
                        .collect();
                    for model in &mut results {
                        model.#method_name = if model.#morph_type_ident == stringify!(#rel_model_ident) {
                            related_by_id.get(&model.#morph_id_ident).cloned()
                        } else {
                            None
                        };
                    }
                }
            }
        }
    } else if rel_type == "morph_many" || rel_type == "morph_one" {
        // Batch load: one query with WHERE morph_id IN (...) AND morph_type = 'Name'
        // instead of issuing one relationship query per parent model.
        quote! {
            if self.#load_flag {
                let parent_ids: Vec<_> = results.iter().filter_map(|m| #m_lk).collect();
                let all_related: Vec<#rel_model_ident> = if parent_ids.is_empty() {
                    Vec::new()
                } else {
                    let mut query = #rel_model_ident::query()
                        .where_in(stringify!(#morph_id_ident), parent_ids)
                        .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                        .__rullst_freeze_scope();
                    if let Some(ref filter) = self.#filter_flag {
                        query = filter(query);
                    }
                    #guarded_fetch
                    all_related
                };
                #eager_load_assignment
            }
        }
    } else {
        // Batch load belongs_to_many: 2 queries for any collection size.
        // Q1: SELECT parent_fk, related_fk FROM pivot WHERE parent_fk IN (...)
        // Q2: SELECT * FROM related_table WHERE id IN (unique_related_ids)
        // Then distribute in memory. No N+1.
        quote! {
            if self.#load_flag {
                let parent_ids: Vec<i32> = results.iter().filter_map(|m| #m_lk).collect();
                // parent_id -> related models, in the related query order
                let mut parent_to_related: std::collections::HashMap<i32, Vec<#rel_model_ident>> =
                    std::collections::HashMap::with_capacity(results.len());
                if !parent_ids.is_empty() {
                    let driver = rullst_orm::Orm::driver()?;
                    // Q1: pivot table pairs
                    rullst_orm::schema::validate_identifier(#foreign_key)?;
                    rullst_orm::schema::validate_identifier(#related_key)?;
                    rullst_orm::schema::validate_table_name(#pivot_table)?;
                    let placeholders_str = if driver == "postgres" {
                        let mut ph = String::with_capacity(parent_ids.len() * 4);
                        for i in 1..=parent_ids.len() {
                            if i > 1 { ph.push_str(", "); }
                            ph.push('$');
                            ph.push_str(&i.to_string());
                        }
                        ph
                    } else {
                        let mut ph = String::with_capacity(parent_ids.len() * 3);
                        for i in 0..parent_ids.len() {
                            if i > 0 { ph.push_str(", "); }
                            ph.push('?');
                        }
                        ph
                    };
                    let pivot_sql = format!(
                        "SELECT {fk}, {rk} FROM {pt} WHERE {fk} IN ({ph})",
                        fk = #foreign_key,
                        rk = #related_key,
                        pt = #pivot_table,
                        ph = placeholders_str,
                    );
                    let mut pivot_query = rullst_orm::_sqlx::query_as::<_, (i32, i32)>(
                        rullst_orm::_sqlx::AssertSqlSafe(pivot_sql.as_str())
                    );
                    for id in &parent_ids {
                        pivot_query = pivot_query.bind(*id);
                    }
                    let pivot_pairs: Vec<(i32, i32)> = rullst_orm::dispatch_executor!(read_pool, |executor| {
                        let fetch = pivot_query.fetch_all(executor);
                        match rullst_orm::schema::get_query_timeout() {
                            Some(t) => tokio::time::timeout(t, fetch).await.map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))?.map_err(rullst_orm::Error::from),
                            None => fetch.await.map_err(rullst_orm::Error::from),
                        }
                    })?;

                    if !pivot_pairs.is_empty() {
                        // Deduplicate related IDs for Q2
                        let mut related_ids: Vec<i32> = pivot_pairs.iter().map(|(_, rid)| *rid).collect();
                        related_ids.sort_unstable();
                        related_ids.dedup();

                        let mut query = #rel_model_ident::query().where_in("id", related_ids).__rullst_freeze_scope();
                        if let Some(ref filter) = self.#filter_flag {
                            query = filter(query);
                        }
                        #guarded_fetch

                        // related_id -> the parent of each of its pivot rows
                        let mut parents_of: std::collections::HashMap<i32, Vec<i32>> =
                            std::collections::HashMap::with_capacity(pivot_pairs.len());
                        for (parent_id, related_id) in &pivot_pairs {
                            parents_of.entry(*related_id).or_insert_with(Vec::new).push(*parent_id);
                        }

                        // Distribute in the related query's order, so every parent's
                        // list follows its ORDER BY (for example a constrained
                        // `order_by`), like the lazy loader, instead of the order of
                        // the unordered pivot rows.
                        for related in all_related {
                            let Some((last, earlier)) = parents_of
                                .get(&related.id)
                                .and_then(|parents| parents.split_last())
                            else {
                                continue;
                            };
                            for parent_id in earlier {
                                parent_to_related
                                    .entry(*parent_id)
                                    .or_insert_with(Vec::new)
                                    .push(related.clone());
                            }
                            parent_to_related
                                .entry(*last)
                                .or_insert_with(Vec::new)
                                .push(related);
                        }
                    }
                }

                // Every parent is loaded: one without related rows (or
                // without a key) gets an empty list, and parents sharing a
                // local key each receive the group.
                for model in &mut results {
                    let key = #model_lk;
                    model.#method_name = Some(
                        key.and_then(|key| parent_to_related.get(&key).cloned()).unwrap_or_default()
                    );
                }
            }
        }
    }
}
