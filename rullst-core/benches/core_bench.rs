use criterion::{Criterion, criterion_group, criterion_main};
use rullst_core::html::escape_str;
use rullst_core::security::{generate_csrf_token, mask_pii};
use std::hint::black_box;

/// Benchmarks the HTML escaping hot path.
/// `escape_str` is called on EVERY dynamic value inside an `html!` macro block.
/// Sub-100ns is required to keep SSR competitive with static files.
fn bench_html_escape(c: &mut Criterion) {
    let mut group = c.benchmark_group("html_escape");

    // Common case: clean input that requires no escaping (fast path via Cow::Borrowed)
    group.bench_function("clean_input_no_escape", |b| {
        let input = black_box("Hello, World! This is a regular sentence.");
        b.iter(|| escape_str(input))
    });

    // Worst case: input full of HTML special characters (triggers full allocation)
    group.bench_function("malicious_input_full_escape", |b| {
        let input = black_box("<script>alert('xss')</script> & \"quotes\" & 'apostrophes'");
        b.iter(|| escape_str(input))
    });

    // Realistic case: one or two escapes in a longer string
    group.bench_function("realistic_partial_escape", |b| {
        let input = black_box("User: Alice <admin@example.com> & Co.");
        b.iter(|| escape_str(input))
    });

    group.finish();
}

/// Benchmarks PII masking — called on every ORM write in audit mode.
/// Must be sub-microsecond to avoid slowing down database operations.
fn bench_mask_pii(c: &mut Criterion) {
    let mut group = c.benchmark_group("mask_pii");

    group.bench_function("email_field", |b| {
        b.iter(|| mask_pii(black_box("alice@example.com")))
    });

    group.bench_function("credit_card_field", |b| {
        b.iter(|| mask_pii(black_box("4242424242424242")))
    });

    group.bench_function("phone_field", |b| {
        b.iter(|| mask_pii(black_box("+1-800-555-0199")))
    });

    group.bench_function("safe_field_no_pii", |b| {
        b.iter(|| mask_pii(black_box("My favorite color is blue")))
    });

    group.finish();
}

/// Benchmarks CSRF token generation — called on every form render.
/// Uses `rand::distr::Alphanumeric` — must be fast enough for high-traffic endpoints.
fn bench_csrf_token(c: &mut Criterion) {
    c.bench_function("generate_csrf_token_32_chars", |b| {
        b.iter(|| generate_csrf_token())
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(200);
    targets = bench_html_escape, bench_mask_pii, bench_csrf_token
);
criterion_main!(benches);
