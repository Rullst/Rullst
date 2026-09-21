//! Retain admission through ordinary HTTP body delivery, error or cancellation.
use super::ApplicationRequestGuard;
use axum::{
    body::{Body, HttpBody},
    response::Response,
};
use http_body::{Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub(super) fn track(response: Response, guard: ApplicationRequestGuard) -> Response {
    if response.body().is_end_stream() {
        // Empty/HEAD/upgrade responses need no body-lifetime extension. An
        // upgraded connection must have its own application shutdown policy.
        return response;
    }
    response.map(|body| {
        Body::new(TrackedBody {
            body,
            guard: Some(guard),
        })
    })
}

struct TrackedBody {
    body: Body,
    guard: Option<ApplicationRequestGuard>,
}

impl HttpBody for TrackedBody {
    type Data = <Body as HttpBody>::Data;
    type Error = <Body as HttpBody>::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let result = Pin::new(&mut self.body).poll_frame(context);
        if matches!(result, Poll::Ready(None | Some(Err(_)))) || self.body.is_end_stream() {
            self.guard.take();
        }
        result
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

#[cfg(test)]
mod tests;
