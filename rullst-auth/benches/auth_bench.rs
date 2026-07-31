use criterion::{Criterion, criterion_group, criterion_main};
use rullst_auth::{decrypt_session, encrypt_session, make_login_cookie};
use std::hint::black_box;

/// A 32-byte app key for benchmarks (AES-256 requires exactly 32 bytes).
const TEST_KEY: &[u8; 32] = b"rullst-bench-key-32-bytes-exact!";

/// Benchmarks AES-256-GCM session encryption.
/// `encrypt_session` runs on every successful login and session cookie refresh.
fn bench_encrypt_session(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_crypto");

    group.bench_function("encrypt_session", |b| {
        b.iter(|| encrypt_session(black_box(12345_i32), black_box(TEST_KEY)))
    });

    // Pre-generate a valid token to use for the decrypt bench
    let token = encrypt_session(99, TEST_KEY).expect("bench setup: encrypt_session");

    group.bench_function("decrypt_session", |b| {
        b.iter(|| decrypt_session(black_box(token.as_str()), black_box(TEST_KEY)))
    });

    // Round-trip: both encrypt + decrypt (represents the full per-request overhead)
    group.bench_function("round_trip_encrypt_decrypt", |b| {
        b.iter(|| {
            let tok = encrypt_session(black_box(42_i32), black_box(TEST_KEY)).unwrap();
            let _uid = decrypt_session(black_box(&tok), black_box(TEST_KEY)).unwrap();
        })
    });

    group.finish();
}

/// Benchmarks login cookie assembly.
/// `make_login_cookie` is the full pipeline: encrypt_session + base64-encode + Set-Cookie formatting.
fn bench_login_cookie(c: &mut Criterion) {
    c.bench_function("make_login_cookie", |b| {
        // Temporarily set APP_KEY env var for the bench
        // SAFETY: This is a single-threaded benchmark setup, no race conditions can occur.
        unsafe {
            std::env::set_var("APP_KEY", "cmxzdC1iZW5jaC1rZXktMzItYnl0ZXMt");
        }
        b.iter(|| make_login_cookie(black_box(42_i32)))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_encrypt_session, bench_login_cookie
);
criterion_main!(benches);
