//! Actix Web middleware adapter for the canonical webhook verifier.

use super::{
    DEFAULT_REPLAY_STORE, InMemoryWebhookReplayStore, MAX_WEBHOOK_PAYLOAD_BYTES,
    WebhookMiddlewareState, capital_error_status_code, verify_payload,
};
#[cfg(test)]
use crate::capital::WebhookEvent;
use crate::capital::{BillingProvider, provider};
use actix_web::HttpMessage;
use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::Next;
use actix_web::{Error, HttpResponse, web};
use futures_util::StreamExt;
use std::collections::HashMap;

/// Production-safe Actix middleware for signed billing webhooks.
///
/// Mount it with `actix_web::middleware::from_fn(verify_webhook_actix)`. It uses the globally
/// configured Capital provider and rejects `mock_*` verification modes.
pub async fn verify_webhook_actix<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    verify_webhook_actix_inner(req, next, &DEFAULT_REPLAY_STORE, false, provider()).await
}

/// Explicit local-only Actix middleware that permits signed `mock_*` fixtures.
///
/// Do not mount this variant on a publicly reachable endpoint.
pub async fn verify_webhook_actix_mock_local<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    verify_webhook_actix_inner(req, next, &DEFAULT_REPLAY_STORE, true, provider()).await
}

/// Actix middleware entry point using [`WebhookMiddlewareState`] from application data.
///
/// Mount with `App::app_data(web::Data::new(state))` and
/// `actix_web::middleware::from_fn(verify_webhook_actix_with_state)`.
pub async fn verify_webhook_actix_with_state<B>(
    state: web::Data<WebhookMiddlewareState>,
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    verify_webhook_actix_inner(
        req,
        next,
        &state.replay_store,
        state.allow_mock,
        state.resolved_provider(),
    )
    .await
}

async fn verify_webhook_actix_inner<B>(
    mut req: ServiceRequest,
    next: Next<B>,
    replay_store: &InMemoryWebhookReplayStore,
    allow_mock: bool,
    active_provider: Option<&dyn BillingProvider>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    let mut payload = req.take_payload();
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return Ok(reject(req, StatusCode::BAD_REQUEST)),
        };
        let Some(next_length) = body.len().checked_add(chunk.len()) else {
            return Ok(reject(req, StatusCode::PAYLOAD_TOO_LARGE));
        };
        if next_length > MAX_WEBHOOK_PAYLOAD_BYTES {
            return Ok(reject(req, StatusCode::PAYLOAD_TOO_LARGE));
        }
        body.extend_from_slice(&chunk);
    }
    let body = body.freeze();

    let Some(active_provider) = active_provider else {
        return Ok(reject(req, StatusCode::INTERNAL_SERVER_ERROR));
    };
    let headers = collect_headers(&req);
    let event = match verify_payload(active_provider, &body, &headers, replay_store, allow_mock) {
        Ok(event) => event,
        Err(error) => return Ok(reject(req, status_from_error(&error))),
    };

    req.extensions_mut().insert(event);
    req.set_payload(Payload::from(body));
    next.call(req)
        .await
        .map(ServiceResponse::map_into_left_body)
}

fn collect_headers(req: &ServiceRequest) -> HashMap<String, String> {
    req.headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_lowercase(), value.to_string()))
        })
        .collect()
}

fn status_from_error(error: &crate::CapitalError) -> StatusCode {
    StatusCode::from_u16(capital_error_status_code(error))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn reject<B>(req: ServiceRequest, status: StatusCode) -> ServiceResponse<EitherBody<B>>
where
    B: MessageBody + 'static,
{
    req.into_response(HttpResponse::build(status).finish())
        .map_into_right_body()
}

#[cfg(test)]
mod tests;
