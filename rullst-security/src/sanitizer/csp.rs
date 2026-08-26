use axum::{
    body::Body,
    http::{HeaderValue, Request, Response},
};
use rullst_core::security::{CspNonce, render_csp_policy};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

pub fn generate_nonce() -> String {
    CspNonce::generate().to_string()
}

#[derive(Clone, Debug, Default)]
pub struct CspSecurityLayer;

impl<S> Layer<S> for CspSecurityLayer {
    type Service = CspSecurityService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CspSecurityService { inner }
    }
}

#[derive(Clone)]
pub struct CspSecurityService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for CspSecurityService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let nonce = CspNonce::get_or_insert(req.extensions_mut());
        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let headers = res.headers_mut();

            let csp_value = render_csp_policy(None, Some(&nonce));
            if let Ok(v) = HeaderValue::from_str(&csp_value) {
                headers.insert("content-security-policy", v);
            }
            headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
            headers.insert(
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                "referrer-policy",
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            Ok(res)
        })
    }
}
