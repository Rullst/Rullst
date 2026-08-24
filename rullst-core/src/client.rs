#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, prelude::*};

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
/// Calls a server RPC endpoint from a browser and returns its textual body.
pub async fn rpc_call(fn_name: &str) -> Result<String, JsValue> {
    let url = format!("/api/rpc/{}", fn_name);
    let window =
        web_sys::window().ok_or_else(|| JsValue::from_str("browser Window unavailable"))?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;
    let resp_value =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch returned a non-Response value"))?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!(
            "RPC request failed with HTTP {}",
            resp.status()
        )));
    }
    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text()?).await?;
    text_val
        .as_string()
        .ok_or_else(|| JsValue::from_str("RPC response body was not text"))
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
