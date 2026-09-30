//! Unit tests for the validating extractors and their error rendering.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::http::Request;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Clone)]
struct TestPayload {
    #[validate(length(min = 3, message = "Username too short"))]
    username: String,
    #[validate(email(message = "Must be a valid email"))]
    email: String,
}

#[tokio::test]
async fn test_validation_success() {
    let _payload = TestPayload {
        username: "venelouis".to_string(),
        email: "vene@rullst.dev".to_string(),
    };

    // Form success
    let req = Request::builder()
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            "username=venelouis&email=vene%40rullst.dev",
        ))
        .unwrap();

    let validated = ValidatedForm::<TestPayload>::from_request(req, &())
        .await
        .unwrap();
    assert_eq!(validated.0.username, "venelouis");
    assert_eq!(validated.0.email, "vene@rullst.dev");

    // Json success
    let req_json = Request::builder()
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            r#"{"username": "venelouis", "email": "vene@rullst.dev"}"#,
        ))
        .unwrap();

    let validated_json = ValidatedJson::<TestPayload>::from_request(req_json, &())
        .await
        .unwrap();
    assert_eq!(validated_json.0.username, "venelouis");
}

#[tokio::test]
async fn test_validation_failure_json() {
    let req = Request::builder()
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            r#"{"username": "ab", "email": "invalid-email"}"#,
        ))
        .unwrap();

    let err = ValidatedJson::<TestPayload>::from_request(req, &())
        .await
        .unwrap_err();

    match err {
        ValidationError::ValidationError { errors, is_htmx } => {
            assert!(!is_htmx);
            let formatted = format_errors(&errors);
            assert!(formatted.contains_key("username"));
            assert!(formatted.contains_key("email"));
            assert_eq!(formatted.get("username").unwrap()[0], "Username too short");
            assert_eq!(formatted.get("email").unwrap()[0], "Must be a valid email");
        }
        _ => panic!("Expected ValidationError"),
    }
}

#[tokio::test]
async fn test_validation_failure_htmx() {
    let req = Request::builder()
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("HX-Request", "true")
        .body(axum::body::Body::from("username=ab&email=invalid-email"))
        .unwrap();

    let err = ValidatedForm::<TestPayload>::from_request(req, &())
        .await
        .unwrap_err();

    match err {
        ValidationError::ValidationError { errors, is_htmx } => {
            assert!(is_htmx);
            let response = ValidationError::ValidationError { errors, is_htmx }.into_response();
            // htmx swaps only 2xx/3xx responses, so the fragment is a 200.
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[VALIDATION_STATUS_HEADER], "422");

            let body_bytes = axum::body::to_bytes(response.into_body(), 10000)
                .await
                .unwrap();
            let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
            assert!(body_str.contains("Validation Failed"));
            assert!(body_str.contains("username"));
            assert!(body_str.contains("email"));
        }
        _ => panic!("Expected ValidationError"),
    }
}

#[tokio::test]
async fn test_extraction_error_responses() {
    let err_htmx = ValidationError::ExtractionError {
        message: "Malformed form data".to_string(),
        is_htmx: true,
    };
    assert!(format!("{}", err_htmx).contains("Malformed form data"));

    let resp_htmx = err_htmx.into_response();
    assert_eq!(resp_htmx.status(), StatusCode::OK);
    assert_eq!(resp_htmx.headers()[VALIDATION_STATUS_HEADER], "400");

    let err_json = ValidationError::ExtractionError {
        message: "Invalid JSON syntax".to_string(),
        is_htmx: false,
    };
    let resp_json = err_json.into_response();
    assert_eq!(resp_json.status(), StatusCode::BAD_REQUEST);
    assert!(!resp_json.headers().contains_key(VALIDATION_STATUS_HEADER));
}

#[derive(Debug, Deserialize, Validate)]
struct RolePayload {
    #[allow(dead_code)]
    role: Role,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    Admin,
}

async fn body_string(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn htmx_extraction_error_does_not_echo_request_input() {
    let payload = "%3Cimg%20src%3Dx%20onerror%3Dalert(1)%3E";
    let req = Request::builder()
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("HX-Request", "true")
        .body(axum::body::Body::from(format!("role={payload}")))
        .unwrap();
    let err = ValidatedForm::<RolePayload>::from_request(req, &())
        .await
        .unwrap_err();
    let body = body_string(err.into_response()).await;
    assert!(!body.contains("<img"), "{body}");
    assert!(!body.contains("onerror"), "{body}");
    assert!(body.contains("could not be read"), "{body}");

    let req = Request::builder()
        .header("content-type", "application/json")
        .header("HX-Request", "true")
        .body(axum::body::Body::from(r#"{"role":"<div hx-get=\"/x\">"}"#))
        .unwrap();
    let err = ValidatedJson::<RolePayload>::from_request(req, &())
        .await
        .unwrap_err();
    let body = body_string(err.into_response()).await;
    assert!(!body.contains("hx-get"), "{body}");

    let req = Request::builder()
        .header("content-type", "text/plain")
        .body(axum::body::Body::from("{}"))
        .unwrap();
    let err = ValidatedJson::<RolePayload>::from_request(req, &())
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Extraction error: Unsupported request content type."
    );
}

#[tokio::test]
async fn htmx_fragments_escape_messages_and_fields() {
    let err = ValidationError::ExtractionError {
        message: "<script>alert(1)</script>".to_string(),
        is_htmx: true,
    };
    let body = body_string(err.into_response()).await;
    assert!(!body.contains("<script>"), "{body}");
    assert!(body.contains("&lt;script&gt;"), "{body}");

    let mut errors = validator::ValidationErrors::new();
    let mut field_error = validator::ValidationError::new("custom");
    field_error.message = Some("<b onmouseover=x>bad</b>".into());
    errors.add("name", field_error);
    let body = body_string(
        ValidationError::ValidationError {
            errors,
            is_htmx: true,
        }
        .into_response(),
    )
    .await;
    assert!(!body.contains("<b onmouseover"), "{body}");
    assert!(body.contains("&lt;b onmouseover=x&gt;"), "{body}");
}
