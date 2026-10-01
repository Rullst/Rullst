//! Lazy relationship loaders generated on the model: `<relation>()` and
//! `<relation>_constrained(modifier)`.

use super::RelationNames;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub(super) fn generate(name: &syn::Ident, names: &RelationNames<'_>) -> TokenStream {
    let RelationNames {
        rel,
        method_name,
        rel_model_ident,
        rel_model_builder_ident,
        fk_ident,
        lk_ident,
        pk_ident,
        morph_id_ident,
        morph_type_ident,
    } = names;
    let rel_type = &rel.rel_type;
    let foreign_key = &rel.foreign_key;
    let related_key = &rel.related_key;
    let pivot_table = &rel.pivot_table;
    let method_name_constrained = quote::format_ident!("{}_constrained", rel.field_name);

    let lazy_load_check = quote! {
        if rullst_orm::is_lazy_loading_prevented() {
            return Err(rullst_orm::Error::Validation(format!(
                "StrictLazyLoading: attempted to lazily load relation '{}' on '{}' without eager loading",
                stringify!(#method_name),
                stringify!(#name),
            )));
        }
    };

    if rel_type == "has_many" {
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone()).get().await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let mut q = #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone())
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.get().await
                })
            }
        }
    } else if rel_type == "has_one" {
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone()).first().await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let mut q = #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone())
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.first().await
                })
            }
        }
    } else if rel_type == "belongs_to" {
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    #rel_model_ident::query().where_eq(stringify!(#pk_ident), self.#fk_ident.clone()).first().await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let mut q = #rel_model_ident::query().where_eq(stringify!(#pk_ident), self.#fk_ident.clone())
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.first().await
                })
            }
        }
    } else if rel_type == "morph_many" {
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    #rel_model_ident::query()
                        .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                        .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                        .get().await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let mut q = #rel_model_ident::query()
                        .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                        .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.get().await
                })
            }
        }
    } else if rel_type == "morph_one" {
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    #rel_model_ident::query()
                        .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                        .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                        .first().await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let mut q = #rel_model_ident::query()
                        .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                        .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.first().await
                })
            }
        }
    } else if rel_type == "morph_to" {
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    if self.#morph_type_ident != stringify!(#rel_model_ident) {
                        return Ok(None);
                    }
                    #rel_model_ident::query()
                        .where_eq(stringify!(#pk_ident), self.#morph_id_ident.clone())
                        .first()
                        .await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    if self.#morph_type_ident != stringify!(#rel_model_ident) {
                        return Ok(None);
                    }
                    let mut q = #rel_model_ident::query()
                        .where_eq(stringify!(#pk_ident), self.#morph_id_ident.clone())
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.first().await
                })
            }
        }
    } else if rel_type == "belongs_to_many" {
        let pivot_fk = format!("{}.{}", pivot_table, foreign_key);
        let pivot_rk = format!("{}.{}", pivot_table, related_key);
        quote! {
            pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let related_pk = format!("{}.{}", <#rel_model_ident as rullst_orm::RullstModel>::table_name(), "id");
                    let select_raw = format!("{}.*", <#rel_model_ident as rullst_orm::RullstModel>::table_name());
                    #rel_model_ident::query()
                        .select_raw(&select_raw)
                        .join(#pivot_table, &related_pk, "=", #pivot_rk)
                        .where_eq(&#pivot_fk, self.#lk_ident.clone())
                        .get().await
                })
            }
            pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rullst_orm::Error>> + Send + '_>> {
                Box::pin(async move {
                    #lazy_load_check
                    let related_pk = format!("{}.{}", <#rel_model_ident as rullst_orm::RullstModel>::table_name(), "id");
                    let select_raw = format!("{}.*", <#rel_model_ident as rullst_orm::RullstModel>::table_name());
                    let mut q = #rel_model_ident::query()
                        .select_raw(&select_raw)
                        .join(#pivot_table, &related_pk, "=", #pivot_rk)
                        .where_eq(&#pivot_fk, self.#lk_ident.clone())
                        .__rullst_freeze_scope();
                    q = modifier(q);
                    q.get().await
                })
            }
        }
    } else {
        TokenStream::new()
    }
}
