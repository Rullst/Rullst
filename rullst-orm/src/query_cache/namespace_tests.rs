use super::validate_namespace;

#[test]
fn namespaces_hold_one_to_64_portable_bytes() {
    for valid in ["a", "app.cache-v2_x", &"n".repeat(64)] {
        assert!(validate_namespace(valid).is_ok(), "{valid}");
    }
    for invalid in [
        String::new(),
        "n".repeat(65),
        "app cache".to_string(),
        "app:cache".to_string(),
    ] {
        assert!(validate_namespace(&invalid).is_err(), "{invalid}");
    }
}
