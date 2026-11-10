//! The UI component gallery that the "Generated starters in browsers" job
//! drives (.github/check-ui-components.py). The job copies this project out
//! of the repository and runs `cargo rullst ui add --all` in it, which writes
//! src/ui/ and declares `mod ui;` here, and `cargo rullst add tailwind`.

mod gallery;

use rullst::htmx::{HtmxRequest, HtmxResponse};
use rullst::response::IntoResponse;
use rullst::server::Extension;
use rullst::{Router, Server, routes};

async fn index(
    htmx: HtmxRequest,
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
) -> impl IntoResponse {
    gallery::page(&htmx, csrf_token.as_str())
}

/// Answers an htmx form with a toast and no content (`hx-swap="none"`).
async fn toast() -> HtmxResponse {
    HtmxResponse::new("").trigger(ui::toast::trigger(
        "Saved to the gallery",
        ui::toast::Tone::Success,
    ))
}

/// A slow request, so the job sees the button's loading state.
async fn slow() -> HtmxResponse {
    rullst::runtime::time::sleep(std::time::Duration::from_millis(1500)).await;
    HtmxResponse::new("").trigger(ui::toast::trigger(
        "The slow save finished",
        ui::toast::Tone::Info,
    ))
}

fn router() -> Router {
    routes![
        get("/" => index),
        post("/toast" => toast),
        post("/slow" => slow),
    ]
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .merge_axum(rullst::health::health_router())
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::new(router()).run(3000).await?;
    Ok(())
}
