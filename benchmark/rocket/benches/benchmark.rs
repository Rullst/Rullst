use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rocket_bench::Message;

fn json_bench(c: &mut Criterion) {
    c.bench_function("rocket json serialize", |b| {
        let msg = Message {
            message: "Hello World".to_string(),
        };
        b.iter(|| {
            serde_json::to_string(black_box(&msg)).unwrap()
        })
    });
}

criterion_group!(benches, json_bench);
criterion_main!(benches);
