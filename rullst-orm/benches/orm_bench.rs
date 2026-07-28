// rullst-orm criterion benchmarks
//
// Run with:
//   cargo bench --bench orm_bench
//
// These benchmarks measure:
//   1. Pure CPU work that the ORM adds on top of raw SQLx.
//   2. Full round-trip operations (save / query / delete) against an
//      in-process SQLite file, exercising the real driver stack.
//
// All async benchmarks use Criterion's `async_tokio` feature so they run
// inside a real Tokio runtime and the results are directly comparable to
// production async code.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

// ── model used throughout ────────────────────────────────────────────────
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "bench_users")]
struct BenchUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

// ── one-time DB initialisation (idempotent across bench groups) ──────────
static DB_INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn db_path() -> &'static str {
    "bench_suite.db"
}

fn setup_db(rt: &tokio::runtime::Runtime) {
    DB_INIT.get_or_init(|| {
        rt.block_on(async {
            let _ = std::fs::remove_file(db_path());
            Orm::init(&format!("sqlite:{}?mode=rwc", db_path()))
                .await
                .expect("Orm::init");
            Schema::create("bench_users", |t: &mut Blueprint| {
                t.id();
                t.string("name").not_null();
                t.string("email").not_null();
            })
            .await
            .expect("create bench_users");
        });
    });
}

// ══════════════════════════════════════════════════════════════════════════
// GROUP 1 — pure CPU / in-process work (no I/O)
// ══════════════════════════════════════════════════════════════════════════

fn bench_validate_identifier(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu");

    // Short valid identifier
    group.bench_function("validate_identifier/short", |b| {
        b.iter(|| rullst_orm::schema::validate_identifier(std::hint::black_box("users")))
    });

    // Qualified identifier (table.column)
    group.bench_function("validate_identifier/qualified", |b| {
        b.iter(|| rullst_orm::schema::validate_identifier(std::hint::black_box("public.users")))
    });

    // Invalid identifier — early exit on first bad char
    group.bench_function("validate_identifier/invalid", |b| {
        b.iter(|| rullst_orm::schema::validate_identifier(std::hint::black_box("DROP TABLE users")))
    });

    group.finish();
}

fn bench_to_json(c: &mut Criterion) {
    let user = BenchUser {
        id: 42,
        name: "Alice Benchmark".into(),
        email: "alice@bench.example".into(),
    };

    let mut group = c.benchmark_group("cpu");
    group.bench_function("to_json/user", |b| {
        b.iter(|| std::hint::black_box(user.to_json()))
    });

    // from_json round-trip
    let json_str = user.to_json();
    group.bench_function("from_json/user", |b| {
        b.iter(|| BenchUser::from_json(std::hint::black_box(&json_str)).unwrap())
    });

    group.finish();
}

fn bench_query_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu");

    // Building a query object (no DB — just struct construction + method calls)
    group.bench_function("query_builder/build", |b| {
        b.iter(|| {
            std::hint::black_box(
                BenchUser::query()
                    .where_eq("email", "alice@bench.example")
                    .order_by("name")
                    .limit(10)
                    .offset(20),
            )
        })
    });

    group.finish();
}

// ══════════════════════════════════════════════════════════════════════════
// GROUP 2 — full DB round-trips (async, SQLite)
// ══════════════════════════════════════════════════════════════════════════

fn bench_save(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    setup_db(&rt);

    let mut group = c.benchmark_group("db_roundtrip");

    group.bench_function("save/insert", |b| {
        b.to_async(&rt).iter(|| async {
            let mut u = BenchUser {
                id: 0,
                name: "Bench".into(),
                email: "bench@example.com".into(),
            };
            u.save().await.unwrap();
            // clean up immediately so the table stays small
            u.delete().await.unwrap();
        });
    });

    group.finish();
}

fn bench_query(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    setup_db(&rt);

    // Pre-populate 100 rows that stay across iterations
    rt.block_on(async {
        for i in 0..100i32 {
            let mut u = BenchUser {
                id: 0,
                name: format!("User{}", i),
                email: format!("user{}@example.com", i),
            };
            u.save().await.unwrap();
        }
    });

    let mut group = c.benchmark_group("db_roundtrip");

    group.bench_function("query/find_by_id", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = std::hint::black_box(BenchUser::find(1).await.unwrap());
        });
    });

    group.bench_function("query/where_eq_first", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = std::hint::black_box(
                BenchUser::query()
                    .where_eq("email", "user50@example.com")
                    .first()
                    .await
                    .unwrap(),
            );
        });
    });

    group.bench_function("query/count", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = std::hint::black_box(BenchUser::query().count().await.unwrap());
        });
    });

    group.bench_function("query/all_limit_10", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = std::hint::black_box(BenchUser::query().limit(10).get().await.unwrap());
        });
    });

    // parametric: vary LIMIT to show scaling
    for size in [1usize, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("query/limit_n", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async move {
                    let _ =
                        std::hint::black_box(BenchUser::query().limit(size).get().await.unwrap());
                });
            },
        );
    }

    group.finish();
}

// ══════════════════════════════════════════════════════════════════════════
// Criterion wiring
// ══════════════════════════════════════════════════════════════════════════

criterion_group!(
    benches,
    bench_validate_identifier,
    bench_to_json,
    bench_query_builder,
    bench_save,
    bench_query,
);
criterion_main!(benches);
