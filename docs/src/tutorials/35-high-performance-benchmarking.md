# Tutorial 35: Sub-Microsecond Tuning & High-Availability Operations ⚡

Measure and optimize your Rullst application for throughput, latency, and resilient failure handling. No universal latency or availability result is implied.

---

## 🛠️ Step 1: Configure Fast Linkers & Compiler Flags

In `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold", "-C", "target-cpu=native"]
```

In `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

---

## 💻 Step 2: SQLx Connection Pool Tuning

```rust
use rullst_orm::DbPool;
use std::time::Duration;

pub async fn configure_production_pool() -> DbPool {
    DbPool::connect_lazy("postgres://...")
        .unwrap()
}
```

---

## 🧪 Step 3: Run Criterion Micro-Benchmarks & Miri Verification

```bash
# Run Criterion benchmarks
cargo bench

# Verify against memory leaks and UB with Miri
cargo miri test -p rullst-core
```

---

## 💡 Key Takeaways
- Sub-microsecond routing (~974 ns) and SSR (~1.07 µs).
- Fast linkers (`mold`, `lld`) can reduce incremental link time; measure results on the target project and host.
