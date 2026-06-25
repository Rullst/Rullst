#![no_main]
use libfuzzer_sys::fuzz_target;
use rullst::htmx::{HtmxRequest, HtmxResponse, render_page};
use axum::extract::FromRequestParts;
use axum::http::Request;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            if let Ok(req) = Request::builder()
                .header("HX-Trigger", s)
                .header("HX-Target", s)
                .body(())
            {
                let (mut parts, _) = req.into_parts();
                let _ = HtmxRequest::from_request_parts(&mut parts, &()).await;
            }
            
            let _res = HtmxResponse::new(s).trigger(s).redirect(s).refresh();
            
            let req_htmx = HtmxRequest {
                is_htmx: true,
                trigger: Some(s.to_string()),
                target: None,
                prompt: None,
                current_url: None,
            };
            let _ = render_page(&req_htmx, s, s.to_string());
        });
    }
});
