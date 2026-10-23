use super::{document_with_id, validate_index, validate_query};
use serde_json::json;

#[test]
fn index_names_are_lowercase_identifiers_up_to_128_bytes() {
    for valid in [
        "a",
        "posts",
        "posts-2024_v1",
        &format!("a{}", "b".repeat(127)),
    ] {
        assert!(validate_index(valid).is_ok(), "{valid}");
    }
    for invalid in [
        String::new(),
        format!("a{}", "b".repeat(128)),
        "Posts".to_string(),
        "1posts".to_string(),
        "-posts".to_string(),
        "posts.v1".to_string(),
        "posts index".to_string(),
    ] {
        assert!(validate_index(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn queries_and_documents_reach_exactly_their_byte_limits() {
    assert!(validate_query(&"q".repeat(1_024)).is_ok());
    assert!(validate_query(&"q".repeat(1_025)).is_err());
    assert!(validate_query("").is_ok());

    // `{"id":7,"body":"…"}` is 18 bytes plus the body.
    let exact = json!({ "body": "b".repeat(1_048_576 - 18) });
    let document = document_with_id(7, exact).expect("a document of exactly 1 MiB");
    assert_eq!(serde_json::to_vec(&document).unwrap().len(), 1_048_576);
    let over = json!({ "body": "b".repeat(1_048_576 - 17) });
    assert!(document_with_id(7, over).is_err());
}
