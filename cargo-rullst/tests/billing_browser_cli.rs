//! Generated SaaS policy plus Core headers; external checkout is intercepted.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::{Router, body::Body, http::Request, middleware, response::Html, routing::get};
use cargo_rullst::{blueprints::SAAS_BLUEPRINT_ID, generators::project::env_config};
use rullst_core::config::{DEFAULT_CSP_TEMPLATE, SecurityConfig};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use tower::ServiceExt;

#[tokio::test]
async fn generated_stripe_policy_allows_only_the_reviewed_browser_handoff() {
    let root = tempfile::tempdir().unwrap();
    env_config::generate_env_and_configs(
        root.path(),
        true,
        "Sqlite",
        &[],
        SAAS_BLUEPRINT_ID,
        "0123456789abcdef0123456789abcdef",
    )
    .unwrap();
    let config: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.path().join("Rullst.toml")).unwrap()).unwrap();
    let policy: SecurityConfig = config["security"].clone().try_into().unwrap();
    policy.validate().unwrap();
    assert_eq!(
        policy.csp.replace(" https://checkout.stripe.com", ""),
        DEFAULT_CSP_TEMPLATE
    );
    assert_eq!(policy.csp.matches("https://checkout.stripe.com").count(), 1);
    assert_eq!(SecurityConfig::default().csp, DEFAULT_CSP_TEMPLATE);
    assert_eq!(policy.csrf_signed_webhook_paths, ["/billing/webhook"]);

    let router = Router::new()
        .route("/", get(|| async { Html("policy fixture") }))
        .layer(middleware::from_fn(
            rullst_core::security::headers_middleware,
        ))
        .layer(axum::Extension(policy));
    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value.to_str().unwrap().to_string(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(!headers["content-security-policy"].contains("{NONCE}"));

    match std::env::var("RULLST_BILLING_BROWSER_TESTS").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("0") => {
            eprintln!(
                "Generated policy/Core headers checked; Chromium requires RULLST_BILLING_BROWSER_TESTS=1 (Linux CI)."
            );
            return;
        }
        Ok("1") => {}
        other => panic!("invalid browser test mode: {other:?}"),
    }
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../.github/billing-csp-browser-smoke.mjs");
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Node 24 and Chromium required");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&headers).unwrap())
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "hosted checkout browser policy regression"
    );
}
