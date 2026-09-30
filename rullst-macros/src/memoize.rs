//! Expansion of `#[memoize]`; see the attribute's documentation in `lib.rs`.

pub(crate) fn expand_memoize(input_fn: &syn::ItemFn) -> proc_macro2::TokenStream {
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let name = &sig.ident;
    let body = &input_fn.block;
    let output_type = match &sig.output {
        syn::ReturnType::Default => quote::quote!(()),
        syn::ReturnType::Type(_, ty) => quote::quote!(#ty),
    };

    // Extract argument names
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();
    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg
            && let syn::Pat::Ident(pat_ident) = &*pat_type.pat
        {
            arg_names.push(&pat_ident.ident);
            arg_types.push(&pat_type.ty);
        }
    }

    quote::quote! {
        #vis fn #name(#(#arg_names: #arg_types),*) -> #output_type {
            // The key combines a per-function identity with the serialized
            // arguments. `module_path!()` separates modules and crates; the
            // attribute's location separates same-named associated functions.
            // Arguments that cannot be represented as JSON (for example a u128
            // above u64::MAX) run the body uncached instead of panicking.
            // Arguments whose JSON would equal another value's (NaN, +/-inf,
            // `Some(())`, ...) also run uncached rather than share an entry.
            let cache_key = (|| -> ::core::result::Result<serde_json::Value, serde_json::Error> {
                ::core::result::Result::Ok(serde_json::Value::Array(vec![
                    #(serde_json::to_value(&#arg_names)?),*
                ]))
            })()
            .ok()
            .filter(|_| true #(&& rullst::cache::memory::is_exact_json(&#arg_names))*)
            .map(|arguments| {
                format!(
                    "{}:{}",
                    concat!(
                        module_path!(),
                        "::",
                        stringify!(#name),
                        "@",
                        file!(),
                        ":",
                        line!(),
                        ":",
                        column!()
                    ),
                    arguments
                )
            });

            // Check if it exists in the global Rullst memory cache
            if let Some(cache_key) = &cache_key {
                if let Some(cached) = rullst::cache::memory::get(cache_key) {
                    // If it's a String (HTML output), we can downcast or deserialize it.
                    // For simplicity, we assume String return types.
                    if let Ok(cached_str) = serde_json::from_str::<#output_type>(&cached) {
                        return cached_str;
                    }
                }
            }

            // Otherwise, execute the function
            let result: #output_type = { #body };

            // Store it in the cache, unless reading it back would change it.
            if let Some(cache_key) = &cache_key {
                if rullst::cache::memory::is_exact_json(&result) {
                    if let Ok(serialized) = serde_json::to_string(&result) {
                        rullst::cache::memory::set(cache_key, &serialized);
                    }
                }
            }

            result
        }
    }
}
