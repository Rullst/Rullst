use super::same_payload;

#[test]
fn replayed_payloads_compare_as_json_values() {
    assert!(same_payload(r#"{"b":2,"a":1}"#, r#"{"a":1,"b":2}"#));
    assert!(same_payload(
        r#"{"outer":{"y":[1,2],"x":null}}"#,
        r#"{"outer":{"x":null,"y":[1,2]}}"#
    ));
    assert!(!same_payload(r#"{"a":1,"b":2}"#, r#"{"a":1,"b":3}"#));
    assert!(!same_payload(r#"{"list":[1,2]}"#, r#"{"list":[2,1]}"#));
    assert!(!same_payload("not json", r#"{"a":1}"#));
}
