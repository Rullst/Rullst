#[proc_macro_attribute]
pub fn require_role(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let name = &sig.ident;
    let inputs = &sig.inputs;
    let body = &input_fn.block;

    let role_lit = parse_macro_input!(attr as syn::LitStr);
    let role_str = role_lit.value();

    let expanded = quote::quote! {
        #vis async fn #name(#inputs) -> axum::response::Response {
            if !rullst::auth::HasRole::has_role(&user, #role_str) {
                return axum::response::IntoResponse::into_response((
                    axum::http::StatusCode::FORBIDDEN,
                    "Forbidden: Insufficient privileges",
                ));
            }
            
            let result = async move { #body }.await;
            axum::response::IntoResponse::into_response(result)
        }
    };

    expanded.into()
}
