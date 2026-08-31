use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_nexus::{FieldKind, FieldMeta, LocalNexusAccess, Nexus, NexusModel};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::ServiceExt;

struct SemanticModel;

impl NexusModel for SemanticModel {
    fn nexus_table() -> &'static str {
        "semantic_records"
    }

    fn nexus_label() -> &'static str {
        "Semantic Records"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("description", "Description", FieldKind::Textarea),
            FieldMeta::new("is_published", "Published", FieldKind::Boolean),
            FieldMeta::new(
                "status",
                "Status",
                FieldKind::Enum {
                    options: vec!["active", "pending", "archived"],
                },
            ),
        ]
    }
}

fn local_test_router() -> axum::Router {
    let loopback = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000);
    Nexus::new()
        .register::<SemanticModel>()
        .with_local_access(LocalNexusAccess::loopback_only())
        .try_build()
        .expect("valid semantic-widget Nexus metadata")
        .layer(axum::Extension(axum::extract::ConnectInfo(loopback)))
}

#[tokio::test]
async fn semantic_widgets_render_and_reject_unregistered_values() {
    let app = local_test_router();
    let form = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/table/semantic_records/new")
                .body(Body::empty())
                .expect("semantic widget form request"),
        )
        .await
        .expect("semantic widget form response");
    assert_eq!(form.status(), StatusCode::OK);
    let body = axum::body::to_bytes(form.into_body(), 128 * 1024)
        .await
        .expect("bounded semantic widget form body");
    let body = String::from_utf8(body.to_vec()).expect("UTF-8 semantic widget form");
    assert!(body.contains("<textarea name=\"description\""));
    assert!(body.contains("<input type=\"checkbox\" name=\"is_published\" value=\"1\""));
    assert!(body.contains("<select name=\"status\""));
    assert!(body.contains("<option value=\"active\">active</option>"));
    assert!(body.contains("<option value=\"archived\">archived</option>"));

    let csrf = "semantic_widget_csrf";
    for invalid_body in [
        "status=administrator",
        "is_published=maybe",
        "unknown_field=value",
        "status=active&status=archived",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/table/semantic_records")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("Cookie", format!("rullst_csrf={csrf}"))
                    .header("X-CSRF-Token", csrf)
                    .body(Body::from(invalid_body))
                    .expect("invalid semantic widget request"),
            )
            .await
            .expect("invalid semantic widget response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
