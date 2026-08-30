//! Identifier normalization for generated mailables.

use std::io::{Error as IoError, ErrorKind};
use std::path::PathBuf;

pub(super) fn project_root_module() -> Result<PathBuf, IoError> {
    ["src/lib.rs", "src/main.rs"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::NotFound,
                "Rullst project has neither src/lib.rs nor src/main.rs",
            )
        })
}

pub(super) fn to_pascal_case(value: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for character in value.chars() {
        if character == '_' || character == '-' || character.is_whitespace() {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(character.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(character);
        }
    }
    result
}

pub(super) fn to_snake_case(value: &str) -> String {
    let mut result = String::new();
    let mut previous_is_lower = false;

    for character in value.chars() {
        if character == '_' || character == '-' || character.is_whitespace() {
            result.push('_');
            previous_is_lower = false;
        } else if character.is_uppercase() {
            if previous_is_lower {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
            previous_is_lower = false;
        } else {
            result.push(character);
            previous_is_lower = true;
        }
    }

    let mut normalized = String::new();
    let mut previous_is_underscore = false;
    for character in result.chars() {
        if character == '_' {
            if !previous_is_underscore {
                normalized.push(character);
            }
            previous_is_underscore = true;
        } else {
            normalized.push(character);
            previous_is_underscore = false;
        }
    }
    normalized.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::is_valid_rust_identifier;

    #[test]
    fn converts_common_mailable_names() {
        assert_eq!(to_pascal_case("welcome_email"), "WelcomeEmail");
        assert_eq!(to_pascal_case("reset-password"), "ResetPassword");
        assert_eq!(to_snake_case("WelcomeEmail"), "welcome_email");
        assert_eq!(to_snake_case("ResetPassword"), "reset_password");
    }

    #[test]
    fn unsafe_mailable_names_never_produce_two_valid_identifiers() {
        for invalid in ["../../escape", "type", "", "mail/name", "💥"] {
            let pascal = to_pascal_case(invalid);
            let snake = to_snake_case(&pascal);
            assert!(
                !is_valid_rust_identifier(&pascal) || !is_valid_rust_identifier(&snake),
                "accepted {invalid}"
            );
        }
    }
}
