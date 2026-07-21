use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ImplItem, Item};

pub fn parse_live_component(item: TokenStream) -> TokenStream {
    let parsed = syn::parse2::<Item>(item.clone());
    match parsed {
        Ok(Item::Impl(mut item_impl)) => {
            let struct_name = &item_impl.self_ty;
            let mut handlers = Vec::new();
            let mut has_mount = false;
            let mut is_mount_async = false;
            let mut has_render = false;

            for item in &mut item_impl.items {
                if let ImplItem::Fn(method) = item {
                    let method_name = &method.sig.ident;
                    let method_name_str = method_name.to_string();

                    if method_name == "mount" {
                        has_mount = true;
                        is_mount_async = method.sig.asyncness.is_some();
                    }
                    if method_name == "render" {
                        has_render = true;
                    }

                    let has_live_event = method
                        .attrs
                        .iter()
                        .any(|a| a.path().is_ident("live_event"));

                    if has_live_event {
                        // Assume self is first argument, check if it takes payload
                        let takes_payload = method.sig.inputs.len() > 1;
                        let is_async = method.sig.asyncness.is_some();

                        let call = if takes_payload {
                            if is_async {
                                quote! { self.#method_name(payload.clone()).await }
                            } else {
                                quote! { self.#method_name(payload.clone()) }
                            }
                        } else {
                            if is_async {
                                quote! { self.#method_name().await }
                            } else {
                                quote! { self.#method_name() }
                            }
                        };

                        handlers.push(quote! {
                            #method_name_str => { #call; },
                        });

                        method.attrs.retain(|a| !a.path().is_ident("live_event"));
                    }
                }
            }

            if !has_render {
                return Error::new_spanned(
                    item_impl,
                    "#[live_component] impl block must contain a `pub fn render(&self) -> String` method",
                )
                .to_compile_error();
            }

            let mount_tokens = if has_mount {
                if is_mount_async {
                    quote! { async fn mount(&mut self) { self.mount().await; } }
                } else {
                    quote! { async fn mount(&mut self) { self.mount(); } }
                }
            } else {
                quote! {}
            };

            let expanded = quote! {
                #item_impl

                #[rullst::async_trait]
                impl rullst::live::LiveComponent for #struct_name {
                    #mount_tokens

                    async fn handle_event(&mut self, payload: serde_json::Value) {
                        if let Some(event_name) = payload.get("rullst_event").and_then(|v| v.as_str()) {
                            match event_name {
                                #(#handlers)*
                                _ => {}
                            }
                        }
                    }

                    fn render(&self) -> String {
                        self.render()
                    }
                }
            };
            expanded
        }
        Ok(Item::Struct(item_struct)) => {
            // Just return the struct untouched
            quote! { #item_struct }
        }
        _ => Error::new_spanned(
            item,
            "#[live_component] can only be applied to structs or impl blocks",
        )
        .to_compile_error(),
    }
}
