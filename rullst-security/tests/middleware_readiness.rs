use axum::{
    body::Body,
    http::{Request, Response},
};
use rullst_security::{DlpResponseLayer, RaspSecurityLayer};
use std::{
    future::{Ready, ready},
    task::{Context, Poll},
};
use tower::{Layer, Service, ServiceExt};

// Like a bounded service, readiness belongs to this instance. A clone has no
// reserved permit until its own poll_ready has succeeded.
#[derive(Default)]
struct PermitService(bool);

impl Clone for PermitService {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Service<Request<Body>> for PermitService {
    type Response = Response<Body>;
    type Error = &'static str;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0 = true;
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        if std::mem::take(&mut self.0) {
            ready(Ok(Response::new(Body::from("ok"))))
        } else {
            ready(Err("middleware lost its reserved readiness permit"))
        }
    }
}

#[tokio::test]
async fn rasp_calls_the_ready_instance_for_inspected_and_uninspected_bodies() {
    let requests = [
        Request::new(Body::empty()),
        Request::post("/")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap(),
    ];
    for request in requests {
        assert!(
            RaspSecurityLayer
                .layer(PermitService::default())
                .oneshot(request)
                .await
                .is_ok()
        );
    }
}

#[tokio::test]
async fn dlp_calls_the_ready_instance() {
    assert!(
        DlpResponseLayer
            .layer(PermitService::default())
            .oneshot(Request::new(Body::empty()))
            .await
            .is_ok()
    );
}
