#![cfg(feature = "ai")]

use rullst::Server;
use rullst::ai::{AiClient, providers::openai::OpenAiProvider};
use rullst::web::axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ChatPrompt {
    prompt: String,
}

#[derive(Serialize)]
struct ChatResponse {
    answer: String,
}

async fn chat(
    State(client): State<AiClient>,
    Json(body): Json<ChatPrompt>,
) -> Result<Json<ChatResponse>, (StatusCode, &'static str)> {
    if body.prompt.len() > 8_192 {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "prompt is too large"));
    }

    let answer = client
        .prompt(&body.prompt)
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "AI request failed"))?;
    Ok(Json(ChatResponse { answer }))
}

#[test]
fn documented_guarded_ai_handler_builds_on_the_axum_escape_hatch() {
    let client = AiClient::new(OpenAiProvider::new("mock_tutorial"));
    let app = Router::new()
        .route("/api/chat", post(chat))
        .with_state(client);
    let _server = Server::new(app.into());
}
