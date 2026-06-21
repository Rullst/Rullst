use actix_bench::configure_app;
use actix_web::{test, App};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    message: String,
}

fn bench_json_parse(c: &mut Criterion) {
    let json_str = r#"{"message": "Hello World"}"#;
    c.bench_function("actix_json_parse", |b| {
        b.iter(|| {
            let msg: Message = serde_json::from_str(black_box(json_str)).unwrap();
            black_box(msg);
        })
    });
}

fn bench_routing(c: &mut Criterion) {
    c.bench_function("actix_routing", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let app = test::init_service(App::new().configure(configure_app)).await;
                let req = test::TestRequest::get().uri("/text").to_request();
                let resp = test::call_service(&app, req).await;
                assert!(resp.status().is_success());
            })
    });
}

criterion_group!(benches, bench_json_parse, bench_routing);
criterion_main!(benches);
