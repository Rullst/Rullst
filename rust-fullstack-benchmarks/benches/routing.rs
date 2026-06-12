use axum::routing::get as axum_get;
use criterion::{Criterion, criterion_group, criterion_main};
use http::Request;
use rullst::routes;
use tower::ServiceExt; // for oneshot

// ---- Loco App Simulation ----
// Skipping Loco App instantiation directly since it requires deep Hooks boilerplate.
// We'll simulate standard routing without the Loco overhead, since the primary focus of
// Loco is leveraging Axum underneath with MVC layers.

// ---- Setup functions ----

fn setup_rullst() -> axum::Router {
    let r = routes![
        get("/" => || async { "Hello World Rullst!" }),
        get("/json" => || async { axum::Json(serde_json::json!({"status": "ok", "framework": "rullst"})) })
    ];
    r.into_axum()
}

fn setup_axum() -> axum::Router {
    axum::Router::new()
        .route("/", axum_get(|| async { "Hello World Axum!" }))
        .route(
            "/json",
            axum_get(|| async {
                axum::Json(serde_json::json!({"status": "ok", "framework": "axum"}))
            }),
        )
}

// Benchmark
fn bench_routing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("Memory Routing");

    let axum_app = setup_axum();
    let rullst_app = setup_rullst();

    group.bench_function("Rullst Router (Plain Text)", |b| {
        b.to_async(&runtime).iter(|| async {
            let req = Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())
                .unwrap();
            let _res = rullst_app.clone().oneshot(req).await.unwrap();
        });
    });

    group.bench_function("Axum Router (Plain Text)", |b| {
        b.to_async(&runtime).iter(|| async {
            let req = Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())
                .unwrap();
            let _res = axum_app.clone().oneshot(req).await.unwrap();
        });
    });

    group.bench_function("Rullst Router (JSON)", |b| {
        b.to_async(&runtime).iter(|| async {
            let req = Request::builder()
                .uri("/json")
                .body(axum::body::Body::empty())
                .unwrap();
            let _res = rullst_app.clone().oneshot(req).await.unwrap();
        });
    });

    group.bench_function("Axum Router (JSON)", |b| {
        b.to_async(&runtime).iter(|| async {
            let req = Request::builder()
                .uri("/json")
                .body(axum::body::Body::empty())
                .unwrap();
            let _res = axum_app.clone().oneshot(req).await.unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_routing);
criterion_main!(benches);
