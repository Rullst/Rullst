use rullst::server::{
    Query,
    Redirect, IntoResponse,
    HeaderMap, StatusCode,
    Bytes,
};
use serde::Deserialize;
use std::collections::HashMap;
use rullst::capital::{BillingProvider, StripeProvider, LemonSqueezyProvider};
use rullst::db::sqlx::{self, Row};
use crate::pages::billing;

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub plan: String,
}

pub async fn pricing_view() -> impl IntoResponse {
    billing::pricing_page()
}

pub async fn checkout_redirect(Query(query): Query<CheckoutQuery>) -> impl IntoResponse {
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());
    let redirect_url = std::env::var("BILLING_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/dashboard".to_string());

    let url_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
    };

    match url_result {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create session: {}", e)).into_response(),
    }
}

pub async fn webhook_handler(headers: HeaderMap, body: rullst::server::Bytes) -> impl IntoResponse {
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());

    let mut headers_map = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(val_str) = v.to_str() {
            headers_map.insert(k.as_str().to_string(), val_str.to_string());
        }
    }

    let event_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
    };

    let event = match event_result {
        Ok(evt) => evt,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid signature").into_response(),
    };

    let pool = rullst_orm::Orm::pool();
    let existing = rullst::db::sqlx::query("SELECT id FROM subscriptions WHERE subscription_id = ?1")
        .bind(&event.subscription_id)
        .fetch_optional(pool)
        .await;

    match existing {
        Ok(Some(row)) => {
            let id: i32 = row.get("id");
            let _ = rullst::db::sqlx::query("UPDATE subscriptions SET status = ?1, plan_id = ?2, ends_at = ?3, updated_at = datetime('now') WHERE id = ?4")
                .bind(event.status.as_str())
                .bind(&event.plan_id)
                .bind(event.ends_at)
                .bind(id)
                .execute(pool)
                .await;
        }
        Ok(None) => {
            let _ = rullst::db::sqlx::query("INSERT INTO subscriptions (user_id, customer_id, subscription_id, plan_id, status, ends_at, created_at, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))")
                .bind(&event.customer_id)
                .bind(&event.subscription_id)
                .bind(&event.plan_id)
                .bind(event.status.as_str())
                .bind(event.ends_at)
                .execute(pool)
                .await;
        }
        Err(_) => {}
    }

    (StatusCode::OK, "OK").into_response()
}
