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

Choose limits from database capacity and measured concurrency. More connections
can increase contention and resource use instead of increasing throughput.

## 3. Run reproducible benchmarks

```bash
cargo bench --workspace
```

Record at least:

- exact commit, Rust version, features and release profile;
- CPU, memory, operating system and power/virtualization settings;
- database/provider version and topology;
- warm-up, sample count, concurrency and payload distribution;
- median and tail latency, throughput, errors and resource use.

Criterion microbenchmarks detect regressions in selected functions. They do not
model a production network, database, proxy or user journey. The separate
cross-framework repository currently exercises historical Rullst 4.x and is not
v12 evidence until refreshed.

## 4. Test failure behavior separately

Availability requires deployment exercises: health/readiness semantics,
timeouts, overload, cancellation, database failover, backup/restore, migration
rollback and secret rotation. Miri, sanitizers, fuzzing and Kani are
target-specific correctness tools; none measures availability or proves the
absence of every memory error.
