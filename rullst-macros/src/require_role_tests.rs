use super::{expand_require_role, pattern_binds_user};

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

#[test]
fn role_attribute_accepts_a_role_of_exactly_128_characters() {
    let handler: syn::ItemFn = syn::parse_str("async fn handler(user: String) {}").expect("fn");
    let literal = syn::LitStr::new(&"é".repeat(128), proc_macro2::Span::call_site());
    assert!(expand_require_role(&literal, &handler).is_ok());
}

#[test]
fn the_user_binding_is_found_inside_every_pattern_form() {
    for (argument, binds) in [
        ("user: User", true),
        ("current @ user: User", true),
        ("current @ other: User", false),
        ("user @ Session { .. }: Session", true),
        ("(user, id): (User, u64)", true),
        ("(id, other): (u64, User)", false),
        ("Extension(user): Extension<User>", true),
        ("Extension(other): Extension<User>", false),
        ("Auth { user, .. }: Auth", true),
        ("Auth { session: user }: Auth", true),
        ("Auth { user: other }: Auth", false),
        ("&user: &User", true),
        ("&other: &User", false),
        ("[user, ..]: [User; 2]", true),
        ("[other, ..]: [User; 2]", false),
        ("(user): User", true),
        ("(other): User", false),
        ("(Ok(user) | Err(user)): Result<User, User>", true),
        ("(Ok(other) | Err(other)): Result<User, User>", false),
        ("_: User", false),
    ] {
        let syn::FnArg::Typed(typed) = syn::parse_str::<syn::FnArg>(argument).expect(argument)
        else {
            panic!("expected a typed argument: {argument}");
        };
        assert_eq!(pattern_binds_user(&typed.pat), binds, "{argument}");
        // A typed pattern nested in another pattern is looked through too.
        assert_eq!(
            pattern_binds_user(&syn::Pat::Type(typed.clone())),
            binds,
            "{argument}"
        );
    }
}
