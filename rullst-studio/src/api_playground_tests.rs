use super::*;
use axum::{body::Body, http::Request};
use serde_json::json;
use tower::ServiceExt;

fn document(value: serde_json::Value) -> OpenApi {
    serde_json::from_value(value).expect("valid OpenAPI document")
}

fn sample() -> OpenApi {
    document(json!({
        "openapi": "3.1.0",
        "info": { "title": "Orders <API>", "version": "1.2.0", "description": "Order service" },
        "paths": {
            "/orders/{id}": {
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } },
                    { "name": "trace", "in": "header", "required": false, "schema": { "type": "string" } }
                ],
                "get": {
                    "operationId": "show_order",
                    "tags": ["orders"],
                    "summary": "<script>alert(1)</script>",
                    "parameters": [
                        { "name": "trace", "in": "header", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": { "200": { "description": "The order" }, "404": { "description": "Missing" } }
                },
                "delete": {
                    "deprecated": true,
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "type": "object" } } }
                    },
                    "responses": { "204": { "description": "Deleted" } }
                }
            }
        }
    }))
}

async fn get(app: &Router, uri: &str) -> Response {
    app.clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn text(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn the_page_the_document_and_the_former_swagger_location_are_mounted() {
    let app = router(sample());

    let page = get(&app, "/studio/api").await;
    assert_eq!(page.status(), StatusCode::OK);
    let html = text(page).await;
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("/studio/assets/studio.css"));
    assert!(html.contains("show_order"));

    let former = get(&app, "/studio/api/").await;
    assert_eq!(former.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        former.headers().get(header::LOCATION).unwrap(),
        "/studio/api"
    );

    let json = get(&app, "/studio/api/openapi.json").await;
    assert_eq!(json.status(), StatusCode::OK);
    assert_eq!(
        json.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let served: serde_json::Value = serde_json::from_str(&text(json).await).unwrap();
    assert_eq!(served, serde_json::to_value(sample()).unwrap());

    assert_eq!(get(&app, "/api").await.status(), StatusCode::NOT_FOUND);
}

#[test]
fn the_page_lists_operations_without_scripts_or_remote_assets() {
    let html = studio_layout(render_reference(&sample()), None, &[]);
    assert!(
        !html.contains("<script"),
        "no script may be loaded or inlined"
    );
    assert!(!html.contains("https://") && !html.contains("http://"));
    assert!(html.contains("Orders &lt;API&gt;"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains(">GET</span>") && html.contains(">DELETE</span>"));
    assert!(html.contains("/orders/{id}"));
    assert!(html.contains("2 operations"));
    assert!(html.contains("deprecated"));
    assert!(html.contains("Request body (required) application/json"));
    assert!(html.contains("<code class=\"text-emerald-300\">404</code>"));
}

#[test]
fn operation_parameters_override_shared_path_parameters() {
    let body = render_reference(&sample());
    let get = &body[..body.find(">DELETE<").unwrap()];
    // `id` is inherited from the path; `trace` is overridden as required.
    assert!(get.contains(">id</td><td class=\"px-4 py-3 text-slate-400\">path<"));
    assert_eq!(get.matches(">trace</td>").count(), 1);
    assert!(get.contains(">trace</td><td class=\"px-4 py-3 text-slate-400\">header</td><td class=\"px-4 py-3 text-slate-400\">required<"));
    let delete = &body[body.find(">DELETE<").unwrap()..];
    assert!(delete.contains(">trace</td><td class=\"px-4 py-3 text-slate-400\">header</td><td class=\"px-4 py-3 text-slate-400\">optional<"));
}

#[test]
fn empty_and_oversized_documents_are_reported() {
    let empty = render_reference(&document(json!({
        "openapi": "3.1.0",
        "info": { "title": "Empty", "version": "0" },
        "paths": {}
    })));
    assert!(empty.contains("declares no operations"));

    let mut paths = serde_json::Map::new();
    for index in 0..=MAX_RENDERED_OPERATIONS {
        paths.insert(
            format!("/item/{index}"),
            json!({ "get": { "responses": { "200": { "description": "ok" } } } }),
        );
    }
    let large = render_reference(&document(json!({
        "openapi": "3.1.0",
        "info": { "title": "Large", "version": "0" },
        "paths": paths
    })));
    assert_eq!(large.matches("<article").count(), MAX_RENDERED_OPERATIONS);
    assert!(large.contains("Showing the first 500 of 501 operations"));
}

#[test]
fn long_text_is_cut_on_a_character_boundary() {
    let long = "é".repeat(MAX_TEXT_CHARS + 10);
    let cut = bounded(&long);
    assert_eq!(cut.chars().count(), MAX_TEXT_CHARS + 1);
    assert!(cut.ends_with('…'));
    assert_eq!(bounded("short"), "short");
}
