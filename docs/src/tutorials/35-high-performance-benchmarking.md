# Tutorial 35: Performance measurement and resilient operation

Performance is a property of a concrete application, build, host and workload.
Rullst includes Criterion microbenchmarks and production helpers, but does not
promise a universal latency, throughput or availability number.

## 1. Build an application-specific release binary

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "debuginfo"
```

Generated projects already ship a release profile with `lto = "thin"`,
`codegen-units = 1` and `strip = "symbols"`; `strip = "debuginfo"` keeps symbol
names for production backtraces. `cargo rullst build` compiles the release
binary and pre-compresses static assets.

Alternative linkers such as `mold` or `lld` can improve developer link time on
supported hosts. `target-cpu=native` can improve a host-specific binary but
makes it less portable; do not put it in a generally distributed artifact
without an explicit CPU baseline.

## 2. Configure and measure the database pool

```rust
use rullst_orm::Orm;

# async fn configure() -> Result<(), rullst_orm::Error> {
Orm::init_with_options(
    "postgres://app:password@127.0.0.1/app",
    20, // maximum connections
    10, // acquire timeout in seconds
)
.await?;
# Ok(())
# }
```

Call it before `Server::run`: the server keeps an already-initialized pool
instead of opening its own from `DATABASE_URL`. Choose limits from database
capacity and measured concurrency. More connections can increase contention and
resource use instead of increasing throughput.

## 3. Run reproducible benchmarks

In a checkout of the Rullst repository, run one group at a time with the
command the
[benchmark workflow](https://github.com/Rullst/Rullst/blob/main/.github/workflows/bench.yml)
uses for it, for example:

```bash
cargo bench -p rullst-core --bench core_bench
cargo bench -p rullst-orm --features drivers-all --bench orm_bench
```

Record at least:

- exact commit, Rust version, features and release profile;
- CPU, memory, operating system and power/virtualization settings;
- database/provider version and topology;
- warm-up, sample count, concurrency and payload distribution;
- median and tail latency, throughput, errors and resource use.

Criterion microbenchmarks detect regressions in selected functions. They do not
model a production network, database, proxy or user journey. For
cross-framework comparisons, follow the method in the
[v13 framework comparison plan](../v13-framework-comparison.md).

The [public benchmark hub](https://rullst.github.io/Rullst/benches/) links the
eight published groups: facade/HTTP, Core primitives, ORM, Auth, Connect,
Security, AI and Capital. Nine Criterion binaries back those groups because ORM
has both local and cross-ORM inputs. A crate without a microbenchmark is not
automatically less mature: protocol vectors, generated-project compilation,
restart/failure contracts, no_std target builds or whole-request tests can fit
its dominant risk better. Add a new benchmark only with a stable operation,
input, unit and interpretation.

The repository also contains a cross-ORM SQLite harness:

```bash
cargo bench -p rullst-orm --features strict-sqlite \
  --bench orm_comparison
```

It pins Diesel and SeaORM in the lockfile and gives all three ORMs one typed
SQLite connection, separate database files, the same schema/unique index, 100
equivalent rows and identical WAL/synchronous/busy-timeout policy. Criterion
measures primary-key lookup, indexed filtered lookup, count, ordered list-ten
and insert/delete. This makes the input inspectable; it does not make the
architectures identical. Diesel's synchronous call path and the async
Rullst/SeaORM executor paths remain part of what is measured.

The benchmark workflow runs it and publishes the results with the other
groups. Read them as regression/comparison evidence for these five SQLite
shapes only, and never generalize them to networked PostgreSQL/MySQL,
concurrency, memory use or complete applications.

## 4. Test failure behavior separately

Availability requires deployment exercises: health/readiness semantics,
timeouts, overload, cancellation, database failover, backup/restore, migration
rollback and secret rotation. Miri, sanitizers, fuzzing and Kani are
target-specific correctness tools; none measures availability or proves the
absence of every memory error.
