use super::expand_billable;

#[test]
fn billable_rejects_inputs_without_named_email() {
    for source in [
        "enum Account { Free }",
        "struct Account(String);",
        "struct Account { name: String }",
    ] {
        let input = syn::parse_str(source).expect("derive input must parse");
        assert!(expand_billable(&input).is_err(), "accepted `{source}`");
    }
}

#[test]
fn billable_preserves_generics_and_optional_fields() {
    let input = syn::parse_str(
        "struct Account<T: Clone> where T: Send { email: String, subscription_id: Option<String>, tier: Option<String>, value: T }",
    )
    .expect("derive input must parse");
    let output = expand_billable(&input)
        .expect("valid Billable input")
        .to_string();

    assert!(output.contains("impl < T : Clone >"));
    assert!(output.contains("Account < T >"));
    assert!(output.contains("where T : Send"));
    assert!(output.contains("fn subscription_id"));
    assert!(output.contains("fn tier"));
}
