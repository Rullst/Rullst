#![cfg_attr(mutants, mutants::skip)]
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[cfg(test)]
mod billable_tests;
mod html_parser;
mod live_parser;
#[cfg(test)]
mod require_role_tests;

/// A macro for writing HTML inline in Rust.
/// It compiles down to highly optimized string concatenations at compile time,
/// and automatically escapes dynamic variables to prevent XSS.
///
/// # Example
///
/// This block is ignored only in the standalone proc-macro crate because the
/// expansion deliberately calls the `rullst::html` runtime and adding that
/// facade here would create a circular development dependency. The same dynamic
/// expansion is compiled and asserted in `rullst/tests/html_snapshot_tests.rs`.
///
/// ```rust,ignore
/// use rullst_macros::html;
///
/// let name = "Mundo";
/// let page = html! {
///     <div class="container">
///         <h1>"Olá, " {name} "!"</h1>
///     </div>
/// };
/// assert!(page.contains("Olá, Mundo!"));
/// ```
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    let node = parse_macro_input!(input as html_parser::HtmlNode);
    let expanded = node.to_tokens();
    expanded.into()
}

/// Proc macro attribute to define a Wasm Island client component.
///
/// It compiles dual versions depending on compilation targets:
/// - On native server compiles, it wraps the component's HTML output in a `<div data-island="..." data-props="...">`
/// - On wasm32-unknown-unknown compiles, it generates structural props parsing and registers a hydration function
#[proc_macro_attribute]
#[allow(clippy::collapsible_if)]
pub fn island(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let name = &sig.ident;
    let body = &input_fn.block;

    // Extract argument names and types
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();

    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                arg_names.push(&pat_ident.ident);
                arg_types.push(&pat_type.ty);
            }
        }
    }

    let props_struct_name =
        syn::Ident::new(&format!("{}_Props", name), proc_macro2::Span::call_site());

    let hydrate_fn_name =
        syn::Ident::new(&format!("hydrate_{}", name), proc_macro2::Span::call_site());

    let expanded = quote::quote! {
        #[cfg(not(target_arch = "wasm32"))]
        #vis fn #name(#(#arg_names: #arg_types),*) -> String {
            let inner_html = {
                #body
            };

            let props_json = serde_json::json!({
                #(stringify!(#arg_names): #arg_names),*
            }).to_string();

            let escaped_props = rullst::html::escape_str(&props_json);

            format!(
                "<div data-island=\"{}\" data-props=\"{}\">{}</div>",
                stringify!(#name),
                escaped_props,
                inner_html
            )
        }

        #[cfg(target_arch = "wasm32")]
        #vis fn #name(#(#arg_names: #arg_types),*) -> String {
            let Some(element) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.create_element("div").ok())
            else {
                return String::new();
            };
            let _ = {
                #body
            };
            String::new()
        }

        #[cfg(target_arch = "wasm32")]
        #[derive(serde::Deserialize)]
        #[allow(non_camel_case_types)]
        struct #props_struct_name {
            #(#arg_names: #arg_types),*
        }

        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        #[allow(non_snake_case)]
        pub fn #hydrate_fn_name(element: web_sys::Element, props_json: &str) {
            let props: #props_struct_name = match serde_json::from_str(props_json) {
                Ok(p) => p,
                Err(_) => return,
            };

            #(let #arg_names = props.#arg_names;)*
            let element = element;

            let _ = {
                #body
            };
        }
    };

    expanded.into()
}

/// Proc macro attribute to define a Live Component.
/// Automatically implements the `LiveComponent` trait and wires `#[live_event]` methods.
#[proc_macro_attribute]
pub fn live_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = item.into();
    live_parser::parse_live_component(input).into()
}

/// Marker attribute for events handled by a Live Component.
#[proc_macro_attribute]
pub fn live_event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A macro for intelligent caching of component renders or database queries.
/// It wraps a function, caching the returned output based on the function's arguments.
/// If the function is called again with the same arguments, it returns the cached HTML immediately.
#[proc_macro_attribute]
pub fn memoize(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
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

    let expanded = quote::quote! {
        #vis fn #name(#(#arg_names: #arg_types),*) -> #output_type {
            // Generate a cache key based on the function name and serialized arguments
            let cache_key = format!("{}:{}", stringify!(#name), serde_json::json!([#(#arg_names),*]).to_string());

            // Check if it exists in the global Rullst memory cache
            if let Some(cached) = rullst::cache::memory::get(&cache_key) {
                // If it's a String (HTML output), we can downcast or deserialize it.
                // For simplicity, we assume String return types.
                if let Ok(cached_str) = serde_json::from_str::<#output_type>(&cached) {
                    return cached_str;
                }
            }

            // Otherwise, execute the function
            let result: #output_type = { #body };

            // Store it in the cache
            if let Ok(serialized) = serde_json::to_string(&result) {
                rullst::cache::memory::set(&cache_key, &serialized);
            }

            result
        }
    };

    expanded.into()
}

/// Legacy compatibility marker that preserves the annotated function unchanged.
///
/// Route registration is defined by the `routes!` macro in `rullst-core`. This attribute never
/// implemented path registration or content negotiation and is retained for one compatibility
/// cycle so existing code does not have its function signature rewritten unexpectedly.
#[deprecated(
    since = "12.0.0",
    note = "use rullst::routes! for route registration; this legacy marker will be removed"
)]
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.is_empty() {
        item
    } else {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[route] does not register paths; use rullst::routes! instead",
        )
        .into_compile_error()
        .into()
    }
}

/// Defines a dual-target server function.
///
/// The native expansion preserves the annotated function's attributes,
/// visibility, complete signature, parameters, generics, `where` clause, and
/// body. The Wasm expansion preserves the same public signature and delegates
/// the request to Rullst's client bridge.
///
/// The current client bridge identifies an RPC by function name. Argument
/// transport and server-side route registration are separate runtime concerns;
/// this macro does not claim to implement either one.
#[proc_macro_attribute]
pub fn server_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[server_function] does not accept attribute arguments",
        )
        .into_compile_error()
        .into();
    }

    let input_fn = parse_macro_input!(item as syn::ItemFn);
    match expand_server_function(input_fn) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_server_function(input_fn: syn::ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    if input_fn.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            input_fn.sig.fn_token,
            "#[server_function] requires an async function",
        ));
    }

    if let Some(receiver) = input_fn
        .sig
        .inputs
        .iter()
        .find_map(|argument| match argument {
            syn::FnArg::Receiver(receiver) => Some(receiver),
            syn::FnArg::Typed(_) => None,
        })
    {
        return Err(syn::Error::new_spanned(
            receiver,
            "#[server_function] supports free functions, not methods with a self receiver",
        ));
    }

    let attributes = &input_fn.attrs;
    let visibility = &input_fn.vis;
    let signature = &input_fn.sig;
    let body = &input_fn.block;
    let name = signature.ident.to_string();

    let expanded = quote::quote! {
        #[cfg(not(target_arch = "wasm32"))]
        #(#attributes)*
        #visibility #signature #body

        #[cfg(target_arch = "wasm32")]
        #(#attributes)*
        #[allow(unused_variables)]
        #visibility #signature {
            let response_body = match rullst::client::rpc_call(#name).await {
                Ok(response_body) => response_body,
                Err(_) => return ::core::default::Default::default(),
            };

            match serde_json::from_str(&response_body) {
                Ok(value) => value,
                Err(_) => ::core::default::Default::default(),
            }
        }
    };

    Ok(expanded)
}

#[proc_macro_attribute]
pub fn require_role(attr: TokenStream, item: TokenStream) -> TokenStream {
    let role = parse_macro_input!(attr as syn::LitStr);
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    match expand_require_role(&role, &input_fn) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_require_role(
    role: &syn::LitStr,
    input_fn: &syn::ItemFn,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    if input_fn.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            input_fn.sig.fn_token,
            "#[require_role] supports async handlers only",
        ));
    }
    let role_value = role.value();
    if role_value.trim().is_empty()
        || role_value.chars().count() > 128
        || role_value.chars().any(char::is_control)
    {
        return Err(syn::Error::new_spanned(
            role,
            "#[require_role] needs a non-empty role of at most 128 characters without controls",
        ));
    }
    let has_user = input_fn.sig.inputs.iter().any(|argument| match argument {
        syn::FnArg::Receiver(_) => false,
        syn::FnArg::Typed(argument) => pattern_binds_user(&argument.pat),
    });
    if !has_user {
        return Err(syn::Error::new_spanned(
            &input_fn.sig.inputs,
            "#[require_role] requires a handler parameter that binds an authenticated `user`",
        ));
    }

    let attributes = &input_fn.attrs;
    let vis = &input_fn.vis;
    let mut signature = input_fn.sig.clone();
    signature.output = syn::parse_quote!(-> rullst::response::Response);
    let body = &input_fn.block;

    let expanded = quote::quote! {
        #(#attributes)*
        #vis #signature {
            if !rullst::auth::HasRole::has_role(&user, #role) {
                return rullst::response::IntoResponse::into_response((
                    rullst::http::StatusCode::FORBIDDEN,
                    "Forbidden: Insufficient privileges",
                ));
            }

            let result = async move { #body }.await;
            rullst::response::IntoResponse::into_response(result)
        }
    };
    Ok(expanded)
}

fn pattern_binds_user(pattern: &syn::Pat) -> bool {
    match pattern {
        syn::Pat::Ident(binding) => {
            binding.ident == "user"
                || binding
                    .subpat
                    .as_ref()
                    .is_some_and(|(_, pattern)| pattern_binds_user(pattern))
        }
        syn::Pat::Or(pattern) => pattern.cases.iter().any(pattern_binds_user),
        syn::Pat::Paren(pattern) => pattern_binds_user(&pattern.pat),
        syn::Pat::Reference(pattern) => pattern_binds_user(&pattern.pat),
        syn::Pat::Slice(pattern) => pattern.elems.iter().any(pattern_binds_user),
        syn::Pat::Struct(pattern) => pattern
            .fields
            .iter()
            .any(|field| pattern_binds_user(&field.pat)),
        syn::Pat::Tuple(pattern) => pattern.elems.iter().any(pattern_binds_user),
        syn::Pat::TupleStruct(pattern) => pattern.elems.iter().any(pattern_binds_user),
        syn::Pat::Type(pattern) => pattern_binds_user(&pattern.pat),
        _ => false,
    }
}

#[proc_macro_derive(Billable)]
pub fn derive_billable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match expand_billable(&input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_billable(input: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let syn::Data::Struct(data_struct) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "Billable can only be derived for a struct with named fields",
        ));
    };
    let syn::Fields::Named(fields) = &data_struct.fields else {
        return Err(syn::Error::new_spanned(
            &data_struct.fields,
            "Billable requires a struct with a named `email: String` field",
        ));
    };
    let has_field = |expected: &str| {
        fields
            .named
            .iter()
            .any(|field| field.ident.as_ref().is_some_and(|ident| ident == expected))
    };
    if !has_field("email") {
        return Err(syn::Error::new_spanned(
            fields,
            "Billable requires a named `email: String` field",
        ));
    }

    let sub_id_fn = if has_field("subscription_id") {
        quote::quote! {
            fn subscription_id(&self) -> Option<String> {
                self.subscription_id.clone()
            }
        }
    } else {
        quote::quote! {}
    };

    let tier_fn = if has_field("tier") {
        quote::quote! {
            fn tier(&self) -> Option<String> {
                self.tier.clone()
            }
        }
    } else {
        quote::quote! {}
    };

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote::quote! {
        impl #impl_generics rullst::capital::Billable for #name #type_generics #where_clause {
            fn email(&self) -> String {
                self.email.clone()
            }

            #sub_id_fn
            #tier_fn
        }
    })
}
