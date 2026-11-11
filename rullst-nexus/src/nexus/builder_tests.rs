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
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![],
        }]),
        brand: Arc::new("Test".to_string()),
        audit_policy: NexusAuditPolicy::Disabled,
    };
    let sidebar = render_sidebar(&state, None);
    assert!(sidebar.contains("/nexus/table/users"));
    // The assistant link follows the `ai` feature.
    assert_eq!(sidebar.contains("AI Assistant"), cfg!(feature = "ai"));
    assert!(sidebar.contains("Threat Radar (SOC)"));
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
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![],
        }]),
        brand: Arc::new("Test".to_string()),
        audit_policy: NexusAuditPolicy::Disabled,
    };
    let sidebar = render_sidebar(&state, Some("users"));
    assert!(sidebar.contains("nexus-nav-active"));
}

#[test]
fn test_render_shell_contains_brand() {
    let state = NexusState {
        registry: Arc::new(vec![]),
        brand: Arc::new("MySaaS".to_string()),
        audit_policy: NexusAuditPolicy::Disabled,
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
                    base64::engine::general_purpose::STANDARD.encode(format!("admin:{test_pass}"))
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
    let test_password = format!(
        "metadata_audit_test_{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let result = Nexus::new()
        .register::<InvalidMetadataModel>()
        .with_auth("metadata-audit", test_password)
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
    assert_eq!(DummyModel::nexus_pk_columns(), ["id"]);
}

struct Progress;
impl NexusModel for Progress {
    fn nexus_table() -> &'static str {
        "lesson_progress"
    }
    fn nexus_label() -> &'static str {
        "Progress"
    }
    fn nexus_pk_columns() -> Vec<&'static str> {
        vec!["user_id", "lesson_id"]
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("user_id", "User", FieldKind::Number),
            FieldMeta::new("lesson_id", "Lesson", FieldKind::Number),
        ]
    }
}

struct Slug;
impl NexusModel for Slug {
    fn nexus_table() -> &'static str {
        "pages"
    }
    fn nexus_label() -> &'static str {
        "Pages"
    }
    fn nexus_pk() -> &'static str {
        "slug"
    }
    fn nexus_pk_columns() -> Vec<&'static str> {
        Vec::new()
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![FieldMeta::new("slug", "Slug", FieldKind::Text)]
    }
}

#[test]
fn registration_records_every_key_column_with_the_first_as_pk() {
    let nexus = Nexus::new()
        .register::<Progress>()
        .register::<Slug>()
        .register::<DummyModel>();
    let progress = &nexus.registry[0];
    assert_eq!(progress.pk, "user_id");
    assert_eq!(progress.key_columns(), ["user_id", "lesson_id"]);
    assert!(progress.is_key_column("lesson_id"));
    assert!(!progress.is_key_column("id"));
    // An empty list falls back to `nexus_pk()`.
    let slug = &nexus.registry[1];
    assert_eq!(slug.pk, "slug");
    assert_eq!(slug.key_columns(), ["slug"]);
    assert_eq!(nexus.registry[2].key_columns(), ["id"]);
}

#[tokio::test]
#[allow(deprecated)]
async fn legacy_into_router_is_also_fail_closed_without_authentication() {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let router = Nexus::new().into_router();
    let request = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .expect("valid request");
    let response = router.oneshot(request).await.expect("router response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn builder_options_keep_the_registry_and_policy() {
    let audited = Nexus::new()
        .register::<DummyModel>()
        .with_brand("Audited")
        .with_required_audit();
    assert_eq!(audited.registry.len(), 1);
    assert_eq!(audited.brand, "Audited");
    assert!(matches!(audited.audit_policy, NexusAuditPolicy::Required));

    let local = Nexus::new()
        .register::<DummyModel>()
        .with_local_access(LocalNexusAccess::loopback_only());
    assert_eq!(local.registry.len(), 1);
    assert!(matches!(
        local.auth,
        Some(PendingAuthPolicy::Validated(NexusAuthPolicy::LoopbackOnly(
            _
        )))
    ));
}

#[tokio::test]
async fn application_stylesheets_follow_the_nexus_stylesheet_on_every_page() {
    use axum::http::{Request, StatusCode};
    use base64::Engine;
    use tower::ServiceExt;

    let password = "stylesheet_test_password_0123456789";
    let router = Nexus::new()
        .with_auth("admin", password)
        .with_stylesheet("/static/nexus-overrides.css")
        .with_stylesheet("/static/brand.css?v=2&theme=dark")
        .try_build()
        .expect("same-origin stylesheets");
    let mut request = Request::builder()
        .uri("/")
        .header(
            axum::http::header::AUTHORIZATION,
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(format!("admin:{password}"))
            ),
        )
        .body(axum::body::Body::empty())
        .expect("request");
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.31:443"
            .parse::<std::net::SocketAddr>()
            .expect("valid test peer"),
    ));
    request
        .extensions_mut()
        .insert(NexusVerifiedTls::from_trusted_tls_termination());
    let response = router.oneshot(request).await.expect("dashboard");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("bounded page");
    let page = String::from_utf8(body.to_vec()).expect("UTF-8 page");
    let links = "<link rel=\"stylesheet\" href=\"/nexus/assets/nexus.css\" />\n\
                 <link rel=\"stylesheet\" href=\"/static/nexus-overrides.css\" />\n\
                 <link rel=\"stylesheet\" href=\"/static/brand.css?v=2&amp;theme=dark\" />\n\
                 <script src=\"/nexus/assets/htmx-2.0.4.min.js\" defer></script>";
    assert!(page.contains(links), "{page}");

    // Without the option the shell links Nexus's stylesheet only.
    let state = NexusState {
        registry: Arc::new(Vec::new()),
        brand: Arc::new("Nexus".to_string()),
        audit_policy: NexusAuditPolicy::Disabled,
    };
    let shell = render_shell(&state, "", "");
    assert_eq!(shell.matches("rel=\"stylesheet\"").count(), 1, "{shell}");
}

#[test]
fn stylesheets_from_another_origin_do_not_build() {
    for path in [
        "https://cdn.example.com/admin.css",
        "//cdn.example.com/admin.css",
        "static/admin.css",
        "/admin.css\"><script>",
    ] {
        let result = Nexus::new()
            .with_auth("admin", "stylesheet_test_password_0123456789")
            .with_stylesheet(path)
            .try_build();
        let Err(error) = result else {
            panic!("{path} must not build");
        };
        assert_eq!(error, NexusBuildError::InvalidStylesheet, "{path}");
        assert!(
            error.to_string().starts_with(
                "a Nexus stylesheet must be a same-origin path starting with a single '/'"
            ),
            "{error}"
        );
    }
    let nine = (0..9).fold(
        Nexus::new().with_auth("admin", "stylesheet_test_password_0123456789"),
        |nexus, index| nexus.with_stylesheet(format!("/static/{index}.css")),
    );
    assert!(matches!(
        nine.try_build(),
        Err(NexusBuildError::InvalidStylesheet)
    ));
}
