use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    routing::get,
    Router,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    message: String,
}

fn bench_json_parse(c: &mut Criterion) {
    let json_str = r#"{"message": "Hello World"}"#;
    c.bench_function("axum_json_parse", |b| {
        b.iter(|| {
            let msg: Message = serde_json::from_str(black_box(json_str)).unwrap();
            black_box(msg);
        })
    });
}

fn bench_routing(c: &mut Criterion) {
    let app = Router::new().route("/test", get(|| async { "Hello" }));

    c.bench_function("axum_routing", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let req = Request::builder()
                    .method(Method::GET)
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap();

                // Explicit type annotation needed for oneshot
                let res = app.clone().oneshot(req).await.unwrap();
                assert_eq!(res.status(), StatusCode::OK);
            })
    });
}

criterion_group!(benches, bench_json_parse, bench_routing);
criterion_main!(benches);
