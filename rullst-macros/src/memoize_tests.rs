use super::expand_memoize;

#[test]
fn memoize_keys_carry_a_per_function_identity() {
    let function: syn::ItemFn =
        syn::parse_str("pub fn sidebar(user_id: i64) -> String { user_id.to_string() }")
            .expect("fn");
    let expanded = expand_memoize(&function).to_string();

    for identity in [
        "module_path ! ()",
        "stringify ! (sidebar)",
        "file ! ()",
        "line ! ()",
        "column ! ()",
    ] {
        assert!(expanded.contains(identity), "missing {identity}");
    }
    assert!(expanded.contains("pub fn sidebar (user_id : i64) -> String"));
    // The bare-name key that let unrelated functions collide is gone.
    assert!(!expanded.contains("format ! (\"{}:{}\" , stringify ! (sidebar) ,"));
}

#[test]
fn memoize_serializes_arguments_without_panicking_macros() {
    let function: syn::ItemFn =
        syn::parse_str("fn wide(id: u128, label: String) -> String { label }").expect("fn");
    let expanded = expand_memoize(&function).to_string();

    // `serde_json::json!` expands to `to_value(..).unwrap()`.
    assert!(!expanded.contains("json !"), "{expanded}");
    assert!(!expanded.contains("unwrap ()"), "{expanded}");
    assert!(expanded.contains("serde_json :: to_value (& id) ?"));
    assert!(expanded.contains("serde_json :: to_value (& label) ?"));
}
