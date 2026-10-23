//! Exact bounds of Nexus Basic Auth credential validation.

use super::*;

#[test]
fn usernames_hold_at_most_255_bytes_without_padding_or_controls() {
    assert_eq!(validate_username("ops"), Ok(()));
    assert_eq!(validate_username(&"u".repeat(255)), Ok(()));
    assert_eq!(
        validate_username(&"u".repeat(256)),
        Err(NexusBuildError::InvalidUsername)
    );
    for padded_or_control in [" ops", "ops ", "o\tps", "ops\u{7f}"] {
        assert_eq!(
            validate_username(padded_or_control),
            Err(NexusBuildError::InvalidUsername),
            "{padded_or_control:?}"
        );
    }
    assert_eq!(validate_username("  "), Err(NexusBuildError::EmptyUsername));
    assert_eq!(
        validate_username("a:b"),
        Err(NexusBuildError::UsernameContainsSeparator)
    );
    assert_eq!(
        validate_username("ChangeMe"),
        Err(NexusBuildError::PlaceholderUsername)
    );
}

#[test]
fn passwords_need_sixteen_characters_not_bytes() {
    assert_eq!(validate_password("p".repeat(16).as_str()), Ok(()));
    assert_eq!(validate_password("é".repeat(16).as_str()), Ok(()));
    assert_eq!(
        validate_password("é".repeat(15).as_str()),
        Err(NexusBuildError::WeakPassword { minimum: 16 })
    );
    assert_eq!(
        validate_password("your_password_0123456789"),
        Err(NexusBuildError::PlaceholderPassword)
    );
}

#[test]
fn placeholder_passwords_match_exact_names_and_documented_prefixes() {
    for placeholder in [
        "password",
        " Password123 ",
        "replace_with_a_strong_password",
        "replace_me_0123456789",
        "change_me_0123456789",
        "your_password_0123456789",
    ] {
        assert!(is_placeholder_password(placeholder), "{placeholder}");
    }
    for real in ["correct-horse-battery-staple", "my_password_0123456789"] {
        assert!(!is_placeholder_password(real), "{real}");
    }
    assert!(is_placeholder_username(" Your_Username "));
    assert!(!is_placeholder_username("operations"));
}

#[test]
fn a_missing_credential_is_reported_by_name() {
    assert_eq!(
        required_environment_variable("RULLST_NEXUS_TEST_CREDENTIAL_NEVER_SET"),
        Err(NexusBuildError::MissingCredential {
            variable: "RULLST_NEXUS_TEST_CREDENTIAL_NEVER_SET"
        })
    );
}
