use criterion::{Criterion, criterion_group, criterion_main};
use rullst_security::{HtmlSanitizer, RbacGuard, UserContext, VaultSecret};
use std::hint::black_box;

fn bench_html_sanitizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("html_sanitizer");

    let dirty_input = "<script>alert('xss')</script><p>Clean Text <a href=\"javascript:evil()\">Link</a></p>";

    group.bench_function("sanitize_html_xss", |b| {
        b.iter(|| HtmlSanitizer::sanitize(black_box(dirty_input)))
    });

    group.bench_function("sanitize_text_escape", |b| {
        b.iter(|| HtmlSanitizer::sanitize_text(black_box(dirty_input)))
    });

    group.finish();
}

fn bench_rbac_authorization(c: &mut Criterion) {
    let mut group = c.benchmark_group("rbac_guard");

    let user = UserContext::new("usr_123", vec!["user".to_string(), "editor".to_string()]);

    group.bench_function("authorize_role", |b| {
        b.iter(|| RbacGuard::authorize(black_box(&user), black_box("editor")))
    });

    group.bench_function("authorize_owner_or_role", |b| {
        b.iter(|| {
            RbacGuard::authorize_owner_or_role(
                black_box(&user),
                black_box("usr_123"),
                black_box("admin"),
            )
        })
    });

    group.finish();
}

fn bench_vault_zeroization(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault_secret");

    group.bench_function("vault_secret_new_and_drop", |b| {
        b.iter(|| {
            let secret = VaultSecret::new(black_box("super_secret_db_password_123".to_string()));
            black_box(secret);
        })
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_html_sanitizer, bench_rbac_authorization, bench_vault_zeroization
);
criterion_main!(benches);
