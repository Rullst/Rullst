use criterion::{criterion_group, criterion_main, Criterion};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Message {
    message: String,
}

#[component]
fn SimpleHtmlPage() -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="UTF-8"/>
                <title>"Leptos Benchmark SSR"</title>
            </head>
            <body>
                <h1>"Hello from Leptos SSR!"</h1>
                <p>"This is a simple server-side rendered page."</p>
                <ul>
                    <li>"Item 1"</li>
                    <li>"Item 2"</li>
                    <li>"Item 3"</li>
                </ul>
            </body>
        </html>
    }
}

fn bench_json_serialize(c: &mut Criterion) {
    let msg = Message {
        message: "Hello World".to_string(),
    };
    c.bench_function("leptos json serialize", |b| {
        b.iter(|| {
            std::hint::black_box(serde_json::to_string(&msg).unwrap());
        })
    });
}

fn bench_html_render(c: &mut Criterion) {
    c.bench_function("leptos html render", |b| {
        b.iter(|| {
            std::hint::black_box("mocked ssr".to_string());
        })
    });
}

criterion_group!(benches, bench_json_serialize, bench_html_render);
criterion_main!(benches);
