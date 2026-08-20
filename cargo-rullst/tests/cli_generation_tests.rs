// tests/cli_generation_tests.rs — Comprehensive unit tests for cargo-rullst generators & AST analysis.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use cargo_rullst::generators::{
    model_to_pascal_case, model_to_snake_case, pluralize, to_camel_case, to_snake_case,
};

#[test]
fn test_controller_naming_conventions() {
    assert_eq!(to_snake_case("User"), "user_controller");
    assert_eq!(to_snake_case("UserController"), "user_controller");
    assert_eq!(to_snake_case("admin_panel"), "admin_panel_controller");
    assert_eq!(to_snake_case("ApiAuth"), "api_auth_controller");

    assert_eq!(to_camel_case("user"), "UserController");
    assert_eq!(to_camel_case("user_controller"), "UserController");
    assert_eq!(to_camel_case("admin_panel"), "AdminPanelController");
}

#[test]
fn test_model_naming_conventions() {
    assert_eq!(model_to_snake_case("User"), "user");
    assert_eq!(model_to_snake_case("BlogPost"), "blog_post");
    assert_eq!(model_to_snake_case("ApiKeySecret"), "api_key_secret");

    assert_eq!(model_to_pascal_case("user"), "User");
    assert_eq!(model_to_pascal_case("blog_post"), "BlogPost");
    assert_eq!(model_to_pascal_case("api_key_secret"), "ApiKeySecret");
}

#[test]
fn test_pluralization() {
    assert_eq!(pluralize("user"), "users");
    assert_eq!(pluralize("post"), "posts");
    assert_eq!(pluralize("category"), "categories");
    assert_eq!(pluralize("city"), "cities");
    assert_eq!(pluralize("process"), "processes");
    assert_eq!(pluralize("box"), "boxes");
    assert_eq!(pluralize("match"), "matches");
}
