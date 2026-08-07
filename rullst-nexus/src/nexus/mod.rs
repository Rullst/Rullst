pub mod ai_chat;
pub mod crud;
pub mod security;
pub mod telemetry;
pub mod types;
pub mod ui;

pub use ai_chat::*;
pub use crud::*;
pub use security::*;
pub use telemetry::*;
pub use types::*;
pub use ui::*;

use axum::{
    Router as AxumRouter,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

/// The main entry point for configuring and mounting the Rullst Nexus Panel.
pub struct Nexus {
    registry: Vec<RegistryEntry>,
    brand: String,
    auth: Option<(String, String)>,
}

impl Default for Nexus {
    fn default() -> Self {
        Self::new()
    }
}

impl Nexus {
    pub fn new() -> Self {
        Self {
            registry: Vec::new(),
            brand: "Rullst App".to_string(),
            auth: None,
        }
    }

    pub fn register<M: NexusModel>(mut self) -> Self {
        self.registry.push(RegistryEntry {
            table: M::nexus_table(),
            label: M::nexus_label(),
            icon: M::nexus_icon(),
            pk: M::nexus_pk(),
            fields: M::nexus_fields(),
        });
        self
    }

    pub fn with_brand(mut self, brand: impl Into<String>) -> Self {
        self.brand = brand.into();
        self
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some((username.into(), password.into()));
        self
    }

    pub fn build(self) -> AxumRouter {
        let state = Arc::new(NexusState {
            registry: Arc::new(self.registry),
            brand: Arc::new(self.brand),
        });

        let router = AxumRouter::new()
            .route("/", get(nexus_dashboard))
            .route("/table/{table}", get(nexus_table_view))
            .route("/table/{table}/search", get(nexus_table_search))
            .route("/table/{table}/new", get(nexus_new_form))
            .route("/table/{table}", post(nexus_create_record))
            .route("/table/{table}/{id}/edit", get(nexus_edit_form))
            .route(
                "/table/{table}/{id}",
                put(nexus_update_record).post(nexus_update_record),
            )
            .route("/table/{table}/{id}", delete(nexus_delete_record))
            .route("/table/{table}/batch", post(nexus_batch_action))
            .route("/chat", get(nexus_chat_page))
            .route("/chat/query", post(nexus_chat_query))
            .route("/security", get(nexus_security_page))
            .route("/telemetry", get(nexus_telemetry_page))
            .layer(axum::middleware::from_fn(
                rullst_core::security::csrf_middleware,
            ));

        let router = if let Some((username, password)) = self.auth {
            router.layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let expected_username = username.clone();
                    let expected_password = password.clone();
                    async move {
                        if let Some(auth_header) =
                            req.headers().get(axum::http::header::AUTHORIZATION)
                            && let Ok(auth_str) = auth_header.to_str()
                            && let Some(encoded) = auth_str.strip_prefix("Basic ")
                        {
                            use base64::Engine;
                            if let Ok(decoded) =
                                base64::engine::general_purpose::STANDARD.decode(encoded)
                                && let Ok(decoded_str) = String::from_utf8(decoded)
                                && let Some((parts_user, parts_pass)) = decoded_str.split_once(':')
                            {
                                use subtle::ConstantTimeEq;
                                if parts_user == expected_username
                                    && parts_pass.len() == expected_password.len()
                                    && parts_pass
                                        .as_bytes()
                                        .ct_eq(expected_password.as_bytes())
                                        .into()
                                {
                                    return next.run(req).await;
                                }
                            }
                        }
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::UNAUTHORIZED)
                            .header(
                                axum::http::header::WWW_AUTHENTICATE,
                                "Basic realm=\"Nexus Admin Panel\"",
                            )
                            .body(axum::body::Body::empty())
                            .unwrap_or_else(|_| {
                                let mut res =
                                    axum::response::Response::new(axum::body::Body::empty());
                                *res.status_mut() = axum::http::StatusCode::UNAUTHORIZED;
                                res
                            })
                    }
                },
            ))
        } else {
            eprintln!(
                "⚠️ Nexus Warning: Nexus admin panel has NO authentication configured. Use `.with_auth(username, password)` to protect it in production."
            );
            router
        };

        router.with_state(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyModel;
    impl NexusModel for DummyModel {
        fn nexus_table() -> &'static str {
            "dummy"
        }
        fn nexus_label() -> &'static str {
            "Dummies"
        }
        fn nexus_fields() -> Vec<FieldMeta> {
            vec![]
        }
    }

    #[test]
    fn test_nexus_builder() {
        let nexus = Nexus::new()
            .with_brand("CustomBrand")
            .register::<DummyModel>();

        assert_eq!(nexus.brand, "CustomBrand");
        assert_eq!(nexus.registry.len(), 1);
        assert_eq!(nexus.registry[0].table, "dummy");
    }

    #[test]
    fn test_render_sidebar_no_active() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "👤",
                pk: "id",
                fields: vec![],
            }]),
            brand: Arc::new("Test".to_string()),
        };
        let sidebar = render_sidebar(&state, None);
        assert!(sidebar.contains("/nexus/table/users"));
        assert!(sidebar.contains("AI Assistant"));
        assert!(!sidebar.contains("nexus-nav-active"));
    }

    #[test]
    fn test_render_sidebar_with_active() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "👤",
                pk: "id",
                fields: vec![],
            }]),
            brand: Arc::new("Test".to_string()),
        };
        let sidebar = render_sidebar(&state, Some("users"));
        assert!(sidebar.contains("nexus-nav-active"));
    }

    #[test]
    fn test_render_shell_contains_brand() {
        let state = NexusState {
            registry: Arc::new(vec![]),
            brand: Arc::new("MySaaS".to_string()),
        };
        let html = render_shell(&state, "", "<p>content</p>");
        assert!(html.contains("MySaaS"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("nexus-body"));
        assert!(html.contains("<p>content</p>"));
    }

    #[tokio::test]
    async fn test_nexus_with_auth() {
        use axum::http::{Request, StatusCode};
        use base64::Engine;
        use tower::ServiceExt;

        let test_pass = String::from_utf8(vec![115, 101, 99, 114, 101, 116]).unwrap();
        let nexus = Nexus::new()
            .with_brand("Auth Test")
            .with_auth("admin", test_pass);

        let router = nexus.build();

        // 1. Request without authorization header -> 401 Unauthorized
        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::WWW_AUTHENTICATE)
                .unwrap(),
            "Basic realm=\"Nexus Admin Panel\""
        );

        // 2. Request with incorrect credentials -> 401 Unauthorized
        let req = Request::builder()
            .uri("/")
            .header(
                axum::http::header::AUTHORIZATION,
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode("admin:wrong")
                ),
            )
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // 3. Request with correct credentials -> 200 OK
        let req = Request::builder()
            .uri("/")
            .header(
                axum::http::header::AUTHORIZATION,
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode("admin:secret")
                ),
            )
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("valid_id_123"), "valid_id_123");
        assert_eq!(sanitize_identifier("invalid-id!"), "invalidid");
        assert_eq!(sanitize_identifier("a_b-c!@#d"), "a_bcd");
    }

    #[test]
    fn test_nexus_model_defaults() {
        assert_eq!(DummyModel::nexus_icon(), "&#128196;");
        assert_eq!(DummyModel::nexus_pk(), "id");
    }
}
