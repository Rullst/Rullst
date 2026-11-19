//! The security page reads the AI provider configuration from the process
//! environment and the events from the process-wide security store. Its own
//! test binary, with a single test, because it changes both.
#![cfg(all(debug_assertions, feature = "local"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use rullst_core::security::{LiveSecurityEvent, SecurityStore};
use rullst_studio::{LocalStudioAccess, Studio};
use std::net::SocketAddr;
use tower::ServiceExt;

const PROVIDER_VARIABLES: [&str; 5] = [
    "GEMINI_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "DEEPSEEK_API_KEY",
    "OLLAMA_HOST",
];

fn set_all_blank() {
    for name in PROVIDER_VARIABLES {
        // SAFETY: this binary runs a single test, so nothing reads the
        // environment concurrently.
        unsafe { std::env::set_var(name, "  ") };
    }
}

async fn security_page(studio: &Router) -> String {
    let mut request = Request::builder()
        .uri("/studio/security")
        .header(header::HOST, "127.0.0.1:5555")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:42000".parse::<SocketAddr>().unwrap(),
    ));
    let response = studio.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn the_page_names_the_configured_provider_and_colors_event_types() {
    let studio = Studio::new()
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router");

    // Blank values are not a configuration.
    set_all_blank();
    let page = security_page(&studio).await;
    assert!(page.contains("No supported provider configuration was detected."));
    assert!(page.contains("<span>AI provider configuration</span>"));

    for (name, provider) in [
        ("GEMINI_API_KEY", "Google Gemini API"),
        ("OPENAI_API_KEY", "OpenAI (ChatGPT / GPT-4o)"),
        ("ANTHROPIC_API_KEY", "Anthropic Claude"),
        ("DEEPSEEK_API_KEY", "DeepSeek"),
        ("OLLAMA_HOST", "Ollama"),
    ] {
        set_all_blank();
        // SAFETY: as above.
        unsafe { std::env::set_var(name, "mock_studio_provider") };
        let page = security_page(&studio).await;
        assert!(
            page.contains(&format!(
                "Configuration detected for {provider}; reachability"
            )),
            "{name}"
        );
        assert!(
            !page.contains("<span>AI provider configuration</span>"),
            "{name}"
        );
    }

    let store = SecurityStore::global();
    for (event_type, badge) in [
        (
            "HONEYPOT_TRAP_TRIGGERED",
            "text-rose-400 border-rose-500/30 bg-rose-500/10",
        ),
        (
            "LOGIN_JAIL_TRIGGERED",
            "text-rose-400 border-rose-500/30 bg-rose-500/10",
        ),
        (
            "XSS_SANITIZED",
            "text-cyan-400 border-cyan-500/30 bg-cyan-500/10",
        ),
        (
            "DLP_SECRET_LEAK_PREVENTED",
            "text-cyan-400 border-cyan-500/30 bg-cyan-500/10",
        ),
        (
            "STUDIO_TEST_EVENT",
            "text-amber-400 border-amber-500/30 bg-amber-500/10",
        ),
    ] {
        store.push_local_event(LiveSecurityEvent::local(
            event_type,
            "studio page test",
            "127.0.0.1",
        ));
        let page = security_page(&studio).await;
        assert!(
            page.contains(&format!(
                "<span class=\"px-2 py-0.5 rounded text-[10px] font-bold border {badge}\">{event_type}</span>"
            )),
            "{event_type}"
        );
    }
}
