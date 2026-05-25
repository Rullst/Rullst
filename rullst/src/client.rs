#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Client-side utilities for Rullst Wasm Islands.
/// This module is compiled when targeting `wasm32-unknown-unknown`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn rullst_client_init() {
    // Setup client panic hooks for nicer debugging in browser console
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Rullst Wasm Islands client initialized successfully!".into());
}
