use criterion::{Criterion, criterion_group, criterion_main};
use rullst_connect::providers::github::GithubProvider;
use std::hint::black_box;

fn provider_benchmark(c: &mut Criterion) {
    c.bench_function("github_provider_creation", |b| {
        b.iter(|| {
            GithubProvider::try_new(
                black_box("client_id".to_string()),
                black_box("client_secret".to_string().into()),
                black_box("https://redirect_url".to_string()),
            )
        })
    });
}

criterion_group!(benches, provider_benchmark);
criterion_main!(benches);
