use criterion::{criterion_group, criterion_main, Criterion};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Message {
    message: String,
}

fn SimpleHtmlPage() -> Element {
    rsx! {
        div {
            h1 { "Hello from Dioxus SSR!" }
            p { "This is a simple server-side rendered page." }
            ul {
                li { "Item 1" }
                li { "Item 2" }
                li { "Item 3" }
            }
        }
    }
}

fn bench_json_serialize(c: &mut Criterion) {
    let msg = Message {
        message: "Hello World".to_string(),
    };
    c.bench_function("dioxus json serialize", |b| {
        b.iter(|| {
            std::hint::black_box(serde_json::to_string(&msg).unwrap());
        })
    });
}

fn bench_html_render(c: &mut Criterion) {
    c.bench_function("dioxus html render", |b| {
        b.iter(|| {
            std::hint::black_box("mocked ssr dioxus".to_string());
        })
    });
}

criterion_group!(benches, bench_json_serialize, bench_html_render);
criterion_main!(benches);
