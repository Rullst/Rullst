use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, FnArg, GenericArgument, ItemFn, Lit, LitStr, Meta, Pat, PathArguments,
    ReturnType, Token, Type,
};

const MAX_ARGUMENTS: usize = 16;
const MAX_PATH_BYTES: usize = 128;

#[derive(Default)]
struct ServerFunctionArgs {
    path: Option<LitStr>,
}

impl Parse for ServerFunctionArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut parsed = Self::default();
        for meta in Punctuated::<Meta, Token![,]>::parse_terminated(input)? {
            match meta {
                Meta::NameValue(value) if value.path.is_ident("path") => {
                    if parsed.path.is_some() {
                        return Err(syn::Error::new_spanned(value, "duplicate `path` option"));
                    }
                    let Expr::Lit(expression) = &value.value else {
                        return Err(syn::Error::new_spanned(
                            value,
                            "`path` must be a string literal",
                        ));
                    };
                    let Lit::Str(path) = &expression.lit else {
                        return Err(syn::Error::new_spanned(
                            value,
                            "`path` must be a string literal",
                        ));
                    };
                    parsed.path = Some(path.clone());
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "unsupported #[server_function] option; expected `path = \"/api/rpc/...\"`",
                    ));
                }
            }
        }
        Ok(parsed)
    }
}

pub(crate) fn expand(attributes: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = syn::parse2::<ServerFunctionArgs>(attributes)?;
    let input = syn::parse2::<ItemFn>(item)?;
    validate_signature(&input)?;

    let function_name = &input.sig.ident;
    let default_path = format!(
        "/api/rpc/{}",
        function_name.to_string().trim_start_matches("r#")
    );
    let path = args
        .path
        .unwrap_or_else(|| LitStr::new(&default_path, function_name.span()));
    validate_path(&path)?;

    let output = rpc_output(&input.sig.output)?;
    let arguments = input
        .sig
        .inputs
        .iter()
        .map(argument)
        .collect::<syn::Result<Vec<_>>>()?;
    let argument_names = arguments.iter().map(|(name, _)| name).collect::<Vec<_>>();
    let argument_types = arguments.iter().map(|(_, ty)| ty).collect::<Vec<_>>();
    let payload_type = tuple(&argument_types);
    let payload_value = tuple(&argument_names);
    let payload_pattern = tuple(&argument_names);

    let visibility = &input.vis;
    let signature = &input.sig;
    let body = &input.block;
    let item_attributes = &input.attrs;
    let conditional_attributes = conditional_attributes(item_attributes);
    let route_name = format_ident!("{}_rpc_router", function_name);
    let module_name = format_ident!("__rullst_rpc_{}", function_name);

    Ok(quote! {
        #[cfg(not(target_arch = "wasm32"))]
        #(#item_attributes)*
        #visibility #signature #body

        #[cfg(target_arch = "wasm32")]
        #(#item_attributes)*
        #visibility #signature {
            let payload: #payload_type = #payload_value;
            rullst::client::rpc_call::<#payload_type, #output>(#path, &payload).await
        }

        #[cfg(not(target_arch = "wasm32"))]
        #(#conditional_attributes)*
        #visibility fn #route_name() -> rullst::Router {
            rullst::rpc::route(#path, #module_name::handle)
        }

        #[cfg(not(target_arch = "wasm32"))]
        #(#conditional_attributes)*
        #[doc(hidden)]
        mod #module_name {
            use super::*;

            pub(super) async fn handle(
                request: rullst::web::axum::extract::Request,
            ) -> rullst::web::axum::response::Response {
                rullst::rpc::handle_request::<#payload_type, #output, _, _>(
                    request,
                    |payload| async move {
                        let #payload_pattern = payload;
                        super::#function_name(#(#argument_names),*).await
                    },
                )
                .await
            }
        }
    })
}

fn validate_signature(input: &ItemFn) -> syn::Result<()> {
    let signature = &input.sig;
    if signature.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            signature.fn_token,
            "#[server_function] requires an async function",
        ));
    }
    if let Some(receiver) = signature.inputs.iter().find_map(|argument| match argument {
        FnArg::Receiver(receiver) => Some(receiver),
        FnArg::Typed(_) => None,
    }) {
        return Err(syn::Error::new_spanned(
            receiver,
            "#[server_function] supports free functions, not methods with a self receiver",
        ));
    }
    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.generics,
            "#[server_function] requires concrete parameter and output types; generics are not supported",
        ));
    }
    if matches!(signature.safety, syn::Safety::Unsafe(_))
        || signature.abi.is_some()
        || signature.variadic.is_some()
    {
        return Err(syn::Error::new_spanned(
            signature,
            "#[server_function] does not support unsafe, extern, or variadic functions",
        ));
    }
    if signature.inputs.len() > MAX_ARGUMENTS {
        return Err(syn::Error::new_spanned(
            &signature.inputs,
            format!("#[server_function] supports at most {MAX_ARGUMENTS} parameters"),
        ));
    }
    rpc_output(&signature.output)?;
    for input in &signature.inputs {
        argument(input)?;
    }
    Ok(())
}

fn argument(argument: &FnArg) -> syn::Result<(&syn::Ident, &Type)> {
    let FnArg::Typed(argument) = argument else {
        return Err(syn::Error::new_spanned(
            argument,
            "#[server_function] supports free functions only",
        ));
    };
    let Pat::Ident(pattern) = argument.pat.as_ref() else {
        return Err(syn::Error::new_spanned(
            &argument.pat,
            "#[server_function] parameters must use simple identifier patterns",
        ));
    };
    if pattern.by_ref.is_some() || pattern.subpat.is_some() {
        return Err(syn::Error::new_spanned(
            pattern,
            "#[server_function] parameters must use owned identifier bindings",
        ));
    }
    if matches!(argument.ty.as_ref(), Type::Reference(_)) {
        return Err(syn::Error::new_spanned(
            &argument.ty,
            "#[server_function] parameter types must be owned because requests are deserialized",
        ));
    }
    Ok((&pattern.ident, &argument.ty))
}

fn rpc_output(output: &ReturnType) -> syn::Result<&Type> {
    let ReturnType::Type(_, output) = output else {
        return Err(syn::Error::new_spanned(
            output,
            "#[server_function] must return rullst::rpc::RpcResult<T>",
        ));
    };
    let Type::Path(path) = output.as_ref() else {
        return Err(syn::Error::new_spanned(
            output,
            "#[server_function] must return rullst::rpc::RpcResult<T>",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            output,
            "#[server_function] must return rullst::rpc::RpcResult<T>",
        ));
    };
    if segment.ident != "RpcResult" {
        return Err(syn::Error::new_spanned(
            output,
            "#[server_function] must return rullst::rpc::RpcResult<T>",
        ));
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            segment,
            "RpcResult requires exactly one output type",
        ));
    };
    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    let Some(output) = types.next() else {
        return Err(syn::Error::new_spanned(
            arguments,
            "RpcResult requires exactly one output type",
        ));
    };
    if types.next().is_some() || arguments.args.len() != 1 {
        return Err(syn::Error::new_spanned(
            arguments,
            "RpcResult requires exactly one output type",
        ));
    }
    Ok(output)
}

fn validate_path(path: &LitStr) -> syn::Result<()> {
    let value = path.value();
    let valid = value.starts_with("/api/rpc/")
        && value.len() <= MAX_PATH_BYTES
        && !value.contains(['?', '#', '\\'])
        && value.split('/').skip(1).all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    if valid {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            path,
            "server-function path must be a bounded same-origin `/api/rpc/...` path using ASCII letters, digits, `-`, `_`, and `/`",
        ))
    }
}

fn tuple<T: quote::ToTokens>(items: &[T]) -> TokenStream {
    match items {
        [] => quote!(()),
        [item] => quote!((#item,)),
        _ => quote!((#(#items),*)),
    }
}

fn conditional_attributes(attributes: &[Attribute]) -> Vec<&Attribute> {
    attributes
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::expand;
    use quote::quote;

    #[test]
    fn expansion_contains_matching_client_path_route_and_typed_adapter() {
        let expanded = expand(
            quote!(path = "/api/rpc/math/add"),
            quote! {
                pub async fn add(left: u32, right: u32) -> rullst::rpc::RpcResult<u32> {
                    Ok(left.saturating_add(right))
                }
            },
        )
        .expect("valid server function");
        let rendered = expanded.to_string();
        assert!(rendered.contains("/api/rpc/math/add"));
        assert!(rendered.contains("add_rpc_router"));
        assert!(rendered.contains("handle_request"));
        assert!(rendered.contains("rpc_call"));
        assert!(!rendered.contains("Default :: default"));
    }

    #[test]
    fn default_path_is_derived_from_the_function_name() {
        let expanded = expand(
            quote!(),
            quote! {
                async fn health() -> RpcResult<()> { Ok(()) }
            },
        )
        .expect("valid server function");
        assert!(expanded.to_string().contains("/api/rpc/health"));
    }
}
