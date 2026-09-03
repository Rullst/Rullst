//! Metadata-only cache inspector for the verified local Studio boundary.

use axum::extract::{DefaultBodyLimit, Form, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Extension, Router};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use rand::{TryRng as _, rngs::SysRng};
use rullst_core::Cache;
use serde::Deserialize;
use sha2::Sha256;
use std::fmt::Write;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

const TOKEN_DOMAIN: &[u8] = b"rullst.studio.cache-entry.v1";
const TOKEN_KEY_BYTES: usize = 32;
const INSPECTION_LIMIT: usize = 100;
const MAX_TOKEN_TEXT_BYTES: usize = 64;

#[derive(Clone)]
struct CacheInspectorState {
    cache: Option<Cache>,
    token_key: Arc<[u8; TOKEN_KEY_BYTES]>,
}

/// Builds the local cache inspector. Values and bulk flush are not exposed.
pub(crate) fn router(cache: Option<Cache>) -> Result<Router, crate::StudioBuildError> {
    let mut token_key = [0_u8; TOKEN_KEY_BYTES];
    SysRng
        .try_fill_bytes(&mut token_key)
        .map_err(|_| crate::StudioBuildError::RandomnessUnavailable)?;
    let state = CacheInspectorState {
        cache,
        token_key: Arc::new(token_key),
    };
    Ok(Router::new()
        .route("/", get(render_cache_page))
        .route(
            "/forget",
            post(forget_cache_entry).layer(DefaultBodyLimit::max(2 * 1024)),
        )
        .with_state(state))
}

async fn render_cache_page(
    State(state): State<CacheInspectorState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let content = match state.cache.as_ref() {
        Some(cache) => match cache.inspect(INSPECTION_LIMIT).await {
            Ok(snapshot) => render_snapshot(&state, &snapshot),
            Err(error) => render_unavailable(&error.to_string()),
        },
        None => render_unavailable("No Cache was explicitly supplied to Studio::with_cache"),
    };
    if headers.contains_key("hx-request") {
        Html(content).into_response()
    } else {
        Html(crate::data_browser::studio_layout(content, None, &[])).into_response()
    }
}

fn render_snapshot(
    state: &CacheInspectorState,
    snapshot: &rullst_core::cache::CacheInspection,
) -> String {
    let mut rows = String::new();
    for entry in snapshot.entries() {
        let Ok(token) = entry_token(&state.token_key[..], entry.logical_key()) else {
            return render_unavailable("Cache entry token generation failed");
        };
        let fingerprint = &token[..token.len().min(12)];
        let ttl = entry
            .remaining_ttl_ms()
            .map(|milliseconds| format!("{milliseconds} ms"))
            .unwrap_or_else(|| "no expiry".to_string());
        let _ = write!(
            rows,
            r#"<tr class="border-b border-slate-800"><td class="px-4 py-3 font-mono text-xs text-sky-400">cache-{fingerprint}</td><td class="px-4 py-3 text-xs text-slate-300">{} bytes</td><td class="px-4 py-3 text-xs text-slate-400">{ttl}</td><td class="px-4 py-3 text-right"><form method="post" action="/studio/cache/forget"><input type="hidden" name="token" value="{token}"><button type="submit" class="px-3 py-1 rounded border border-rose-800 text-rose-400 text-xs">Invalidate one</button></form></td></tr>"#,
            entry.value_bytes(),
        );
    }
    if rows.is_empty() {
        rows.push_str(
            "<tr><td colspan=\"4\" class=\"px-6 py-10 text-center text-xs text-slate-500\">No live cache entries</td></tr>",
        );
    }
    let truncation = if snapshot.truncated() {
        "The snapshot reached its 100-entry display bound."
    } else {
        "The complete bounded snapshot is shown."
    };
    format!(
        r#"<div class="p-8 font-mono space-y-6 max-w-6xl mx-auto"><header><h1 class="text-3xl font-extrabold text-white">Cache Inspector</h1><p class="text-sm text-slate-400 mt-1">Metadata-only local diagnostics; values and logical keys stay out of HTML.</p></header><div class="p-4 rounded-xl border border-amber-800/50 bg-amber-950/20 text-xs text-amber-300">Individual invalidation uses an opaque process-bound token. Bulk flush and value viewing are deliberately unavailable. {truncation}</div><div class="overflow-x-auto rounded-xl border border-slate-800"><table class="w-full text-left"><thead class="bg-slate-950 text-xs uppercase text-slate-400"><tr><th class="px-4 py-3">Opaque key</th><th class="px-4 py-3">Value size</th><th class="px-4 py-3">TTL</th><th class="px-4 py-3">Action</th></tr></thead><tbody>{rows}</tbody></table></div></div>"#,
    )
}

fn render_unavailable(reason: &str) -> String {
    format!(
        r#"<div class="p-8 font-mono space-y-4 max-w-3xl mx-auto"><h1 class="text-3xl font-extrabold text-white">Cache Inspector</h1><div class="p-5 rounded-xl border border-amber-800/50 bg-amber-950/20"><p class="text-sm font-bold text-amber-400">Unavailable</p><p class="text-xs text-slate-300 mt-2">{}</p></div></div>"#,
        rullst_core::html::escape_str(reason)
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForgetRequest {
    token: String,
}

async fn forget_cache_entry(
    State(state): State<CacheInspectorState>,
    verified: Option<Extension<crate::access::VerifiedLocalStudioAccess>>,
    Form(request): Form<ForgetRequest>,
) -> Response {
    if verified.is_none() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(cache) = state.cache.as_ref() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if request.token.is_empty() || request.token.len() > MAX_TOKEN_TEXT_BYTES {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let signature = match URL_SAFE_NO_PAD.decode(&request.token) {
        Ok(signature) if signature.len() == 32 => signature,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let snapshot = match cache.inspect(INSPECTION_LIMIT).await {
        Ok(snapshot) => snapshot,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let mut selected = None;
    for entry in snapshot.entries() {
        let Ok(mac) = entry_mac(&state.token_key[..], entry.logical_key()) else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        if mac.verify_slice(&signature).is_ok() {
            selected = Some(entry.logical_key().to_string());
        }
    }
    let Some(key) = selected else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if cache.forget(&key).await.is_err() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    Redirect::to("/studio/cache").into_response()
}

fn entry_token(key: &[u8], logical_key: &str) -> Result<String, hmac::digest::InvalidLength> {
    let mac = entry_mac(key, logical_key)?;
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn entry_mac(key: &[u8], logical_key: &str) -> Result<HmacSha256, hmac::digest::InvalidLength> {
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(TOKEN_DOMAIN);
    mac.update(&(logical_key.len() as u64).to_be_bytes());
    mac.update(logical_key.as_bytes());
    Ok(mac)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn with_verified_access(mut request: Request<Body>) -> Request<Body> {
        request
            .extensions_mut()
            .insert(crate::access::VerifiedLocalStudioAccess);
        request
    }

    #[tokio::test]
    async fn page_exposes_only_opaque_metadata_and_one_entry_can_be_invalidated() {
        let cache = Cache::memory();
        cache
            .put("private:user:42", "secret-value", Some(60))
            .await
            .expect("cache fixture");
        let app = router(Some(cache.clone())).expect("cache router");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("cache page");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("cache body");
        let body = String::from_utf8(body.to_vec()).expect("UTF-8 cache body");
        assert!(body.contains("cache-"));
        assert!(body.contains("12 bytes"));
        assert!(!body.contains("private:user:42"));
        assert!(!body.contains("secret-value"));
        let token_start = body.find("name=\"token\" value=\"").expect("token field")
            + "name=\"token\" value=\"".len();
        let token_end = body[token_start..].find('"').expect("token end") + token_start;
        let token = &body[token_start..token_end];

        let request = Request::builder()
            .method("POST")
            .uri("/forget")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("token={token}")))
            .expect("forget request");
        let response = app
            .oneshot(with_verified_access(request))
            .await
            .expect("forget response");
        assert!(response.status().is_redirection());
        assert!(
            cache
                .get("private:user:42")
                .await
                .expect("cache read")
                .is_none()
        );
    }

    #[tokio::test]
    async fn mutation_requires_verified_local_marker_and_rejects_forged_tokens() {
        let cache = Cache::memory();
        cache
            .put("key", "value", None)
            .await
            .expect("cache fixture");
        let app = router(Some(cache.clone())).expect("cache router");
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/forget")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "token=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                ))
                .expect("forget request")
        };

        let unverified = app
            .clone()
            .oneshot(request())
            .await
            .expect("unverified response");
        assert_eq!(unverified.status(), StatusCode::FORBIDDEN);
        let forged = app
            .oneshot(with_verified_access(request()))
            .await
            .expect("forged response");
        assert_eq!(forged.status(), StatusCode::NOT_FOUND);
        assert!(cache.get("key").await.expect("cache read").is_some());
    }
}
