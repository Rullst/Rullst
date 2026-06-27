use criterion::{Criterion, criterion_group, criterion_main};
use rullst::html;
use rullst::routes;
use rullst::server::Request;

// 1. Router Benchmarks
fn bench_router(c: &mut Criterion) {
    let router = routes![
        get("/" => || async { "home" }),
        get("/users" => || async { "users" }),
        post("/users" => || async { "create_user" }),
        get("/users/{id}" => || async { "user_details" }),
        get("/posts/{post_id}/comments/{comment_id}" => || async { "comments" }),
    ];

    let axum_router = router.into_axum();

    c.bench_function("router_match_simple", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let req = Request::builder()
                    .uri("/users")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let mut app = axum_router.clone();
                async move {
                    use tower::Service;
                    let _ = app.call(req).await;
                }
            });
    });

    c.bench_function("router_match_nested_params", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let req = Request::builder()
                    .uri("/posts/123/comments/456")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let mut app = axum_router.clone();
                async move {
                    use tower::Service;
                    let _ = app.call(req).await;
                }
            });
    });
}

// 2. HTML Macro Rendering Benchmarks
fn bench_html_macro(c: &mut Criterion) {
    c.bench_function("html_macro_static", |b| {
        b.iter(|| {
            let _val = html! {
                <div class="container">
                    <h1>"Hello World"</h1>
                    <p>"This is a static HTML render benchmark."</p>
                </div>
            };
        });
    });

    c.bench_function("html_macro_dynamic", |b| {
        let name = "Alice";
        let items = ["Rust", "Go", "Python"];
        b.iter(|| {
            let _val = html! {
                <div class="user-profile">
                    <h2>"Hello, "{name}"!"</h2>
                    <ul>
                        { items.iter().map(|item| html! { <li>{item}</li> }).collect::<String>() }
                    </ul>
                </div>
            };
        });
    });
}

// 3. Middleware Benchmarks
fn bench_middlewares(c: &mut Criterion) {
    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "safe_value" }))
        .layer(axum::middleware::from_fn(rullst::security::waf_middleware));

    c.bench_function("waf_middleware_overhead", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let req = Request::builder()
                    .uri("/?query=safe_value")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let mut app = app.clone();
                async move {
                    use tower::Service;
                    let _ = app.call(req).await;
                }
            });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_router, bench_html_macro, bench_middlewares
);
criterion_main!(benches);
