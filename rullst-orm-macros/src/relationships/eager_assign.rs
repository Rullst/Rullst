//! Distributes one eager-loaded batch among the parent models.
//!
//! Several parents can share one related row or group: every child of a
//! `belongs_to` parent, or parents whose non-unique local key (or a duplicated
//! parent row) matches the same related rows. Each parent receives the full
//! relation. The related value moves to the last parent that needs it and is
//! cloned for the others; a related model without `Clone` fails closed with a
//! `Validation` error instead of leaving later parents empty.

use proc_macro2::TokenStream;
use quote::quote;

/// Local autoref dispatch: `Clone` targets are cloned, others report `None`.
/// The generated code names a concrete related model, so method resolution
/// picks the `Clone` impl whenever it applies, without requiring `Clone`.
fn share_helpers() -> TokenStream {
    quote! {
        struct __RullstEagerShare<'a, T>(&'a T);
        trait __RullstEagerClone {
            type Item;
            fn __rullst_eager_share(&self) -> Option<Self::Item>;
        }
        impl<T: Clone> __RullstEagerClone for __RullstEagerShare<'_, T> {
            type Item = T;
            fn __rullst_eager_share(&self) -> Option<T> {
                Some(self.0.clone())
            }
        }
        trait __RullstEagerUnshared {
            type Item;
            fn __rullst_eager_share(&self) -> Option<Self::Item>;
        }
        impl<T> __RullstEagerUnshared for &__RullstEagerShare<'_, T> {
            type Item = T;
            fn __rullst_eager_share(&self) -> Option<T> {
                None
            }
        }
    }
}

#[cfg_attr(test, mutants::skip)]
pub(super) fn generate_eager_load_assignment(
    parent_name: &syn::Ident,
    is_many: bool,
    map_key_ident: &syn::Ident,
    model_key_ident: &syn::Ident,
    method_name: &syn::Ident,
) -> TokenStream {
    let helpers = share_helpers();
    let (collect, take_last, missing) = if is_many {
        (
            quote! { map.entry(rel.#map_key_ident.clone()).or_insert_with(Vec::new).push(rel); },
            quote! { Some(map.remove(key).unwrap_or_default()) },
            quote! { Some(Vec::new()) },
        )
    } else {
        (
            quote! { map.entry(rel.#map_key_ident.clone()).or_insert(rel); },
            quote! { map.remove(key) },
            quote! { None },
        )
    };
    quote! {
        #helpers
        let mut map = std::collections::HashMap::with_capacity(all_related.len());
        for rel in all_related {
            #collect
        }
        let mut parents_per_key = std::collections::HashMap::with_capacity(results.len());
        for model in results.iter() {
            *parents_per_key.entry(model.#model_key_ident.clone()).or_insert(0_usize) += 1;
        }
        for model in &mut results {
            let key = &model.#model_key_ident;
            let is_last_parent = match parents_per_key.get_mut(key) {
                Some(remaining) => {
                    *remaining = remaining.saturating_sub(1);
                    *remaining == 0
                }
                None => true,
            };
            model.#method_name = if is_last_parent {
                #take_last
            } else if let Some(related) = map.get(key) {
                let shared = (&__RullstEagerShare(related))
                    .__rullst_eager_share()
                    .ok_or_else(|| rullst_orm::Error::Validation(format!(
                        "eager loading `{}` on `{}` gives one related value to several parents; the related model must implement Clone",
                        stringify!(#method_name),
                        stringify!(#parent_name),
                    )))?;
                Some(shared)
            } else {
                #missing
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_eager_load_assignment;

    #[test]
    fn shared_keys_clone_for_earlier_parents_instead_of_consuming_the_map() {
        let parent = quote::format_ident!("Post");
        let key = quote::format_ident!("id");
        let fk = quote::format_ident!("user_id");
        let method = quote::format_ident!("author");
        let single = generate_eager_load_assignment(&parent, false, &key, &fk, &method).to_string();
        assert!(single.contains("parents_per_key"));
        assert!(single.contains("map . get (key)"));
        assert!(single.contains("__rullst_eager_share"));
        assert!(single.contains("the related model must implement Clone"));
        // The consuming lookup is reserved for the last parent of each key.
        assert_eq!(single.matches("map . remove (key)").count(), 1);

        let many = generate_eager_load_assignment(&parent, true, &fk, &key, &method).to_string();
        assert!(many.contains("unwrap_or_default"));
        assert!(many.contains("Some (Vec :: new ())"));
    }
}
