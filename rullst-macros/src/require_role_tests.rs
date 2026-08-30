use super::expand_require_role;

#[test]
fn role_attribute_rejects_missing_user_and_sync_handlers() {
    let role: syn::LitStr = syn::parse_str("\"Admin\"").expect("role literal");
    let missing: syn::ItemFn = syn::parse_str("async fn handler(value: String) {}").expect("fn");
    let error = expand_require_role(&role, &missing).expect_err("missing user must fail");
    assert!(error.to_string().contains("authenticated `user`"));

    let synchronous: syn::ItemFn = syn::parse_str("fn handler(user: String) {}").expect("sync fn");
    let error = expand_require_role(&role, &synchronous).expect_err("sync handler must fail");
    assert!(error.to_string().contains("async handlers only"));
}

#[test]
fn role_attribute_preserves_generics_where_clause_and_other_arguments() {
    let role: syn::LitStr = syn::parse_str("\"Admin\"").expect("role literal");
    let handler: syn::ItemFn = syn::parse_str(
        "pub async fn handler<T>(user: T, value: usize) -> String where T: Clone { value.to_string() }",
    )
    .expect("generic fn");
    let expanded = expand_require_role(&role, &handler)
        .expect("valid handler")
        .to_string();
    assert!(expanded.contains("fn handler < T >"));
    assert!(expanded.contains("where T : Clone"));
    assert!(expanded.contains("value : usize"));
    assert!(expanded.contains("StatusCode :: FORBIDDEN"));
}

#[test]
fn role_attribute_rejects_empty_control_and_oversized_roles() {
    let handler: syn::ItemFn = syn::parse_str("async fn handler(user: String) {}").expect("fn");
    for role in ["", "Admin\nRoot", &"a".repeat(129)] {
        let literal = syn::LitStr::new(role, proc_macro2::Span::call_site());
        assert!(expand_require_role(&literal, &handler).is_err());
    }
}
