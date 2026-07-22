extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod html_parser;
mod live_parser;

/// A macro for writing HTML inline in Rust.
/// It compiles down to highly optimized string concatenations at compile time,
/// and automatically escapes dynamic variables to prevent XSS.
///
/// # Example
/// ```rust,ignore
/// let name = "Mundo";
/// let page = html! {
///     <div class="container">
///         <h1>"Olá, " {name} "!"</h1>
///     </div>
/// };
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
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                arg_names.push(&pat_ident.ident);
                arg_types.push(&pat_type.ty);
            }
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

/// Defines a Dual-Target route. 
/// Generates an HTML responder for browsers and a JSON responder for mobile applications natively.
#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let name = &sig.ident;
    let body = &input_fn.block;

    // A real implementation would parse the path and parameters,
    // generate an Axum handler, and conditionally serialize to JSON or HTML
    // based on the Accept header. Here we provide the foundational macro.
    let expanded = quote::quote! {
        #vis fn #name() -> String {
            let result = { #body };
            // Simulate the dual-target response wrapper
            result
        }
    };
    expanded.into()
}

/// Defines a Server Function for transparent RPC.
/// Functions annotated with `#[server_function]` can be called directly from Wasm frontend code,
/// and the framework automatically generates the HTTP fetch bridge.
#[proc_macro_attribute]
pub fn server_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let name = &sig.ident;
    let body = &input_fn.block;
    let output_type = match &sig.output {
        syn::ReturnType::Default => quote::quote!(()),
        syn::ReturnType::Type(_, ty) => quote::quote!(#ty),
    };

    let name_str = name.to_string();

    let expanded = quote::quote! {
        #[cfg(not(target_arch = "wasm32"))]
        #vis async fn #name() -> #output_type {
            #body
        }

        #[cfg(target_arch = "wasm32")]
        #vis async fn #name() -> #output_type {
            let res = rullst::client::rpc_call(#name_str).await;
            // Deserialize response if it's not unit type
            // Note: For simplicity in this macro, we assume the output implements serde::Deserialize
            // and we parse it from JSON.
            serde_json::from_str(&res).unwrap_or_else(|_| panic!("Failed to parse RPC response from server"))
        }
    };
    expanded.into()
}
