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
