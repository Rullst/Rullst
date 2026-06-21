use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::Serialize;
use warp::http::Request;
use warp::Filter;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

fn bench_json_parsing(c: &mut Criterion) {
    c.bench_function("warp_json_serialize", |b| {
        b.iter(|| {
            let msg = Message {
                message: "Hello World",
            };
            let _ = black_box(serde_json::to_string(&msg).unwrap());
        })
    });
}

fn bench_routing(c: &mut Criterion) {
    let json_route = warp::path("json").and(warp::get()).map(|| {
        warp::reply::json(&Message {
            message: "Hello World",
        })
    });

    c.bench_function("warp_route_match", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter(|| async {
            let _req = Request::builder()
                .method("GET")
                .uri("/json")
                .body(Bytes::new())
                .unwrap();

            // Warp testing utility doesn't have a direct async reply for Request so we just benchmark json serialize
            let _ = black_box(warp::test::request().path("/json").reply(&json_route).await);
        })
    });
}

criterion_group!(benches, bench_json_parsing, bench_routing);
criterion_main!(benches);
