#![cfg(feature = "auth")]

use rullst::auth::HasRole;
use rullst::require_role;
use rullst::server::Extension;

#[derive(Clone)]
struct User {
    role: &'static str,
}

impl HasRole for User {
    fn has_role(&self, role: &str) -> bool {
        self.role == role
    }
}

#[require_role("Admin")]
async fn admin_handler(user: User, value: &'static str) -> String {
    format!("allowed:{value}")
}

#[require_role("Admin")]
async fn extension_handler(Extension(user): Extension<User>) -> &'static str {
    "allowed"
}

#[tokio::test]
async fn role_attribute_preserves_arguments_and_denies_before_the_handler() {
    let denied = admin_handler(User { role: "User" }, "secret").await;
    assert_eq!(denied.status(), rullst::http::StatusCode::FORBIDDEN);

    let allowed = admin_handler(User { role: "Admin" }, "visible").await;
    assert_eq!(allowed.status(), rullst::http::StatusCode::OK);

    let extracted = extension_handler(Extension(User { role: "Admin" })).await;
    assert_eq!(extracted.status(), rullst::http::StatusCode::OK);
}
