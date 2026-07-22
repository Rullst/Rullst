#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
pub async fn rpc_call(fn_name: &str) -> String {
    let url = format!("/api/rpc/{}", fn_name);
    let window = web_sys::window().expect("no global `window` exists");
    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap();
    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap())
        .await
        .unwrap();
    text_val.as_string().unwrap()
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
