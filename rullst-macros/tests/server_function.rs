use rullst_macros::server_function;
use std::future::Future;
use std::task::{Context, Poll, Waker};

#[server_function]
async fn add(left: u32, right: u32) -> u32 {
    left.saturating_add(right)
}

#[server_function]
async fn sum_pair((left, right): (u32, u32)) -> u32 {
    left.saturating_add(right)
}

#[server_function]
async fn preserve_generic<T>(value: T) -> T
where
    T: Send + 'static,
{
    value
}

fn ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test server function unexpectedly returned Pending"),
    }
}

#[test]
fn native_expansion_preserves_parameters_patterns_generics_and_body() {
    assert_eq!(ready(add(20, 22)), 42);
    assert_eq!(ready(sum_pair((19, 23))), 42);
    assert_eq!(ready(preserve_generic(String::from("kept"))), "kept");
}
