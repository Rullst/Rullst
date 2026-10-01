#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, prelude::*};

#[cfg(target_arch = "wasm32")]
use crate::client_contract::{
    CURRENT_CLIENT_CONTRACT_VERSION, ClientContractPolicy, ClientRequest, RequestId,
};
#[cfg(target_arch = "wasm32")]
use crate::rpc::{RpcFailure, RpcResult};

/// Client-side utilities for Rullst Wasm Islands.
/// This module is compiled when targeting `wasm32-unknown-unknown`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[cfg_attr(mutants, mutants::skip)]
pub fn rullst_client_init() {
    // Setup client panic hooks for nicer debugging in browser console
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Rullst Wasm Islands client initialized successfully!".into());
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(mutants, mutants::skip)]
/// Calls one same-origin server-function endpoint with the versioned Rullst
/// client envelope.
///
/// The `rullst_csrf` cookie, when present, is forwarded as `X-CSRF-Token`.
/// Without it (Development and Test, where `Server` mounts no CSRF layer) the
/// request is sent without a token and the server's CSRF middleware, where
/// mounted, decides; its non-JSON `403` is then reported as
/// `rpc.csrf_token_missing`.
pub async fn rpc_call<Req, Output>(path: &str, payload: &Req) -> RpcResult<Output>
where
    Req: serde::Serialize + ?Sized,
    Output: serde::de::DeserializeOwned,
{
    if !is_safe_rpc_path(path) {
        return Err(RpcFailure::framework("rpc.path_invalid", false));
    }
    let window =
        web_sys::window().ok_or_else(|| RpcFailure::framework("rpc.browser_unavailable", false))?;
    let csrf_token = window
        .document()
        .and_then(|document| document.dyn_into::<web_sys::HtmlDocument>().ok())
        .and_then(|document| document.cookie().ok())
        .and_then(|cookie| csrf_token_from_cookie(&cookie).map(str::to_string));
    let request_id = RequestId::framework(format!("rpc_{}", uuid::Uuid::new_v4().simple()));
    let policy = ClientContractPolicy::default();
    let envelope = ClientRequest::new(CURRENT_CLIENT_CONTRACT_VERSION, request_id.clone(), payload);
    let body = policy
        .encode_request(&envelope)
        .map_err(|_| RpcFailure::framework("rpc.request_encoding", false))?;
    let body = String::from_utf8(body)
        .map_err(|_| RpcFailure::framework("rpc.request_encoding", false))?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::Cors);
    opts.set_credentials(web_sys::RequestCredentials::SameOrigin);
    opts.set_body(&JsValue::from_str(&body));
    let request = web_sys::Request::new_with_str_and_init(path, &opts)
        .map_err(|_| RpcFailure::framework("rpc.request_creation", false))?;
    request
        .headers()
        .set("content-type", "application/json")
        .map_err(|_| RpcFailure::framework("rpc.request_creation", false))?;
    request
        .headers()
        .set("accept", "application/json")
        .map_err(|_| RpcFailure::framework("rpc.request_creation", false))?;
    if let Some(token) = csrf_token.as_deref() {
        request
            .headers()
            .set("x-csrf-token", token)
            .map_err(|_| RpcFailure::framework("rpc.request_creation", false))?;
    }
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| RpcFailure::framework("rpc.transport", true))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| RpcFailure::framework("rpc.response_invalid", false))?;
    let success = resp.ok();
    let status = resp.status();
    let response_is_json = resp
        .headers()
        .get("content-type")
        .ok()
        .flatten()
        .is_some_and(|value| is_json_content_type(&value));
    let text_promise = resp
        .text()
        .map_err(|_| RpcFailure::framework("rpc.response_invalid", false))?;
    let text_val = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|_| RpcFailure::framework("rpc.response_invalid", false))?;
    let body = text_val
        .as_string()
        .ok_or_else(|| RpcFailure::framework("rpc.response_invalid", false))?;
    if body.len() > policy.max_body_bytes() {
        return Err(RpcFailure::framework("rpc.response_too_large", false));
    }
    if !response_is_json {
        let code = non_json_failure_code(status, success, csrf_token.is_some());
        return Err(RpcFailure::framework(code, false));
    }
    if !success {
        let failure = policy
            .decode_failure(body.as_bytes())
            .map_err(|_| RpcFailure::framework("rpc.http_failure", false))?;
        if failure.request_id() != &request_id {
            return Err(RpcFailure::framework("rpc.correlation_mismatch", false));
        }
        return Err(RpcFailure::from_detail(failure.error()));
    }

    let response = policy
        .decode_response::<Output>(body.as_bytes())
        .map_err(|_| RpcFailure::framework("rpc.response_invalid", false))?;
    if response.request_id() != &request_id {
        return Err(RpcFailure::framework("rpc.correlation_mismatch", false));
    }
    Ok(response.into_data())
}

/// Framework failure code for a response that is not JSON.
#[cfg(any(target_arch = "wasm32", test))]
fn non_json_failure_code(status: u16, success: bool, sent_csrf_token: bool) -> &'static str {
    match (success, status) {
        (true, _) => "rpc.response_invalid",
        (false, 403) if !sent_csrf_token => "rpc.csrf_token_missing",
        (false, _) => "rpc.http_failure",
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn csrf_token_from_cookie(cookie: &str) -> Option<&str> {
    cookie.split(';').find_map(|entry| {
        entry.trim().strip_prefix("rullst_csrf=").filter(|token| {
            (1..=128).contains(&token.len())
                && token.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    })
}

#[cfg(any(target_arch = "wasm32", test))]
fn is_safe_rpc_path(path: &str) -> bool {
    path.starts_with("/api/rpc/")
        && path.len() <= 128
        && !path.contains(['?', '#', '\\'])
        && !path.starts_with("//")
        && path.split('/').skip(1).all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
}

#[cfg(any(target_arch = "wasm32", test))]
fn is_json_content_type(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
}

#[cfg(test)]
mod contract_tests {
    use super::{
        csrf_token_from_cookie, is_json_content_type, is_safe_rpc_path, non_json_failure_code,
    };

    #[test]
    fn a_missing_csrf_cookie_is_reported_only_when_the_server_rejects() {
        assert_eq!(
            non_json_failure_code(403, false, false),
            "rpc.csrf_token_missing"
        );
        assert_eq!(non_json_failure_code(403, false, true), "rpc.http_failure");
        assert_eq!(non_json_failure_code(500, false, false), "rpc.http_failure");
        assert_eq!(
            non_json_failure_code(200, true, false),
            "rpc.response_invalid"
        );
    }

    #[test]
    fn csrf_cookie_parser_and_rpc_paths_are_bounded() {
        assert_eq!(
            csrf_token_from_cookie("session=x; rullst_csrf=AbC123; theme=dark"),
            Some("AbC123")
        );
        assert_eq!(csrf_token_from_cookie("rullst_csrf=bad/value"), None);
        assert!(is_safe_rpc_path("/api/rpc/counter/increment"));
        assert!(!is_safe_rpc_path("https://example.test/api/rpc/counter"));
        assert!(!is_safe_rpc_path("/api/rpc/../admin"));
        assert!(is_json_content_type("application/json; charset=utf-8"));
        assert!(!is_json_content_type("text/application/json"));
    }
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_client_init() {
        // Just calling it should not panic
        rullst_client_init();
    }
}
