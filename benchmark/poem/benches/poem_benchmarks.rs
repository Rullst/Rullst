use criterion::{black_box, criterion_group, criterion_main, Criterion};

use poem::web::Json;
use poem::{get, handler, Endpoint, Request, Route};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

#[handler]
fn json_handler() -> Json<Message> {
    Json(Message {
        message: "Hello World",
    })
}

fn bench_json_parsing(c: &mut Criterion) {
    c.bench_function("poem_json_serialize", |b| {
        b.iter(|| {
            let msg = Message {
                message: "Hello World",
            };
            let _ = black_box(serde_json::to_string(&msg).unwrap());
        })
    });
}

fn bench_routing(c: &mut Criterion) {
    let app = Route::new().at("/json", get(json_handler));

    c.bench_function("poem_route_match", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter(|| async {
            let req = Request::builder()
                .uri(poem::http::Uri::from_static("/json"))
                .finish();
            let _ = black_box(app.call(req).await);
        })
    });
}

criterion_group!(benches, bench_json_parsing, bench_routing);
criterion_main!(benches);
