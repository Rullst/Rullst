mod access;
pub mod ai_chat;
pub mod crud;
mod metadata;
pub mod security;
pub mod telemetry;
pub mod types;
pub mod ui;

pub use access::*;
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
    auth: Option<PendingAuthPolicy>,
}

enum PendingAuthPolicy {
    Validated(NexusAuthPolicy),
    Basic { username: String, password: String },
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
        self.auth = Some(PendingAuthPolicy::Basic {
            username: username.into(),
            password: password.into(),
        });
        self
    }

    /// Selects a validated access policy for the Nexus admin panel.
    pub fn with_auth_policy(mut self, policy: NexusAuthPolicy) -> Self {
        self.auth = Some(PendingAuthPolicy::Validated(policy));
        self
    }

    /// Explicitly enables loopback-only access for debug builds.
    pub fn with_local_access(self, access: LocalNexusAccess) -> Self {
        self.with_auth_policy(NexusAuthPolicy::loopback_only(access))
    }

    /// Legacy infallible conversion retained for one compatibility cycle.
    ///
    /// Invalid or absent access configuration produces a deny-all router. New code should use
    /// [`Nexus::try_build`] so startup can report the typed configuration error.
    #[deprecated(
        since = "12.0.0",
        note = "use `try_build()` and handle `NexusBuildError`; this compatibility method denies all requests on configuration errors"
    )]
    pub fn into_router(self) -> AxumRouter {
        self.build_fail_closed()
    }

    /// Legacy infallible builder retained for one compatibility cycle.
    ///
    /// Invalid or absent access configuration produces a deny-all router. New code should use
    /// [`Nexus::try_build`] so startup can report the typed configuration error.
    #[deprecated(
        since = "12.0.0",
        note = "use `try_build()` and handle `NexusBuildError`; this compatibility method denies all requests on configuration errors"
    )]
    pub fn build(self) -> AxumRouter {
        self.build_fail_closed()
    }

    /// Builds the Nexus router only after validating an explicit access policy.
    pub fn try_build(self) -> Result<AxumRouter, NexusBuildError> {
        let policy = match self.auth {
            Some(PendingAuthPolicy::Validated(policy)) => validate_policy(policy)?,
            Some(PendingAuthPolicy::Basic { username, password }) => {
                NexusAuthPolicy::basic(username, password)?
            }
            None => return Err(NexusBuildError::MissingAuthenticationPolicy),
        };
        metadata::validate_registry(&self.registry)?;

        let state = Arc::new(NexusState {
            registry: Arc::new(self.registry),
            brand: Arc::new(self.brand),
        });

        let router = AxumRouter::new()
            .route("/", get(nexus_dashboard))
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}", get(nexus_table_view))
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}/search", get(nexus_table_search))
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}/new", get(nexus_new_form))
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}", post(nexus_create_record))
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}/{id}/edit", get(nexus_edit_form))
            .route(
                // rullst-access: admin — protected by policy.protect_router below.
                "/table/{table}/{id}",
                put(nexus_update_record).post(nexus_update_record),
            )
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}/{id}", delete(nexus_delete_record))
            // rullst-access: admin — protected by policy.protect_router below.
            .route("/table/{table}/batch", post(nexus_batch_action))
            .route("/chat", get(nexus_chat_page))
            .route("/chat/query", post(nexus_chat_query))
            .route("/security", get(nexus_security_page))
            .route("/telemetry", get(nexus_telemetry_page))
            .layer(axum::middleware::from_fn(
                rullst_core::security::csrf_middleware,
            ));

        let router = policy.protect_router(router)?;

        Ok(router.with_state(state))
    }

    fn build_fail_closed(self) -> AxumRouter {
        match self.try_build() {
            Ok(router) => router,
            Err(error) => {
                eprintln!("Nexus was not mounted because its access policy is invalid: {error}");
                deny_all_router()
            }
        }
    }
}

fn deny_all_router() -> AxumRouter {
    AxumRouter::new().fallback(|| async {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Nexus is unavailable because secure access has not been configured.",
        )
    })
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

    struct InvalidMetadataModel;
    impl NexusModel for InvalidMetadataModel {
        fn nexus_table() -> &'static str {
            "invalid_metadata"
        }
        fn nexus_label() -> &'static str {
            "Invalid Metadata"
        }
        fn nexus_fields() -> Vec<FieldMeta> {
            vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new(
                    "status",
                    "Status",
                    FieldKind::Enum {
                        options: Vec::new(),
                    },
                ),
            ]
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

        let test_pass = format!(
            "nexus_auth_test_{:016x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let nexus = Nexus::new()
            .with_brand("Auth Test")
            .with_auth("admin", &test_pass);

        let router = nexus.try_build().expect("valid Nexus configuration");
        let secure_request = |mut request: Request<axum::body::Body>| {
            request.extensions_mut().insert(axum::extract::ConnectInfo(
                "192.0.2.30:443"
                    .parse::<std::net::SocketAddr>()
                    .expect("valid test peer"),
            ));
            request
                .extensions_mut()
                .insert(NexusVerifiedTls::from_trusted_tls_termination());
            request
        };

        // 1. Request without authorization header -> 401 Unauthorized
        let req = secure_request(
            Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())
                .unwrap(),
        );

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
        let req = secure_request(
            Request::builder()
                .uri("/")
                .header(
                    axum::http::header::AUTHORIZATION,
                    format!(
                        "Basic {}",
                        base64::engine::general_purpose::STANDARD.encode("admin:wrong")
                    ),
                )
                .body(axum::body::Body::empty())
                .unwrap(),
        );

        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // 3. Request with correct credentials -> 200 OK
        let req = secure_request(
            Request::builder()
                .uri("/")
                .header(
                    axum::http::header::AUTHORIZATION,
                    format!(
                        "Basic {}",
                        base64::engine::general_purpose::STANDARD
                            .encode(format!("admin:{test_pass}"))
                    ),
                )
                .body(axum::body::Body::empty())
                .unwrap(),
        );

        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn nexus_requires_an_explicit_authentication_policy() {
        let result = Nexus::new().register::<DummyModel>().try_build();

        match result {
            Err(error) => assert_eq!(error, NexusBuildError::MissingAuthenticationPolicy),
            Ok(_) => panic!("Nexus must not build without an authentication policy"),
        }
    }

    #[test]
    fn nexus_rejects_invalid_registered_metadata_during_build() {
        let result = Nexus::new()
            .register::<InvalidMetadataModel>()
            .with_local_access(LocalNexusAccess::loopback_only())
            .try_build();

        assert!(matches!(
            result,
            Err(NexusBuildError::InvalidModelMetadata { .. })
        ));
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn legacy_build_is_fail_closed_without_authentication() {
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let router = Nexus::new().build();
        let request = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .expect("valid request");
        let response = router.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
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
