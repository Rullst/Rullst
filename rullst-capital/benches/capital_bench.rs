use criterion::{Criterion, criterion_group, criterion_main};
use rullst_capital::capital::SubscriptionStatus;
use std::hint::black_box;

fn bench_subscription_status_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("capital_subscription");

    group.bench_function("parse_status_active", |b| {
        b.iter(|| SubscriptionStatus::parse_status(black_box("active")))
    });

    group.bench_function("parse_status_past_due", |b| {
        b.iter(|| SubscriptionStatus::parse_status(black_box("past_due")))
    });

    group.bench_function("status_as_str", |b| {
        let status = SubscriptionStatus::Active;
        b.iter(|| black_box(status.as_str()))
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_subscription_status_parsing
);
criterion_main!(benches);
