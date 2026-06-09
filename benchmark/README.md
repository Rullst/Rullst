# Rullst Web Framework Benchmark Suite

This directory contains a comprehensive benchmark suite designed to compare the performance of Rullst against other popular web frameworks in two different tiers.

## Benchmark Methodology

The suite consists of two tiers:

1. **Tier 1 (Cross-Language / Black-box)**: Containerized HTTP load testing measuring requests/sec, latency, and memory footprint of Rullst compared to Axum, Actix-web, Loco, Rocket, Leptos, Gin, Fiber, Django, and Laravel. We use **`bombardier` version `2.0.2`** running inside a Docker container.
2. **Tier 2 (Rust-Specific / Micro-benchmarking)**: Internal CPU-bound measurements of routing speed, template HTML rendering, and middleware overhead using **`criterion` version `0.8.2`** comparing Rullst.

---

## Prerequisites

Before running the benchmarks, ensure you have the following installed:
- **Docker** and **Docker Compose**
- **Rust (v1.85+)** (to run internal benchmarks)
- **Python (v3.x)** (to parse results and build tables)

---

## Tier 1: Running Cross-Language Benchmarks

To ensure accurate measurements and prevent idle container resources from interfering with the benchmarks, the orchestration scripts build, start, benchmark, and tear down each target framework container **sequentially**.

### On Windows (PowerShell)

```powershell
cd benchmark
.\run_benchmarks.ps1
```

### On Linux / macOS (Bash)

```bash
cd benchmark
chmod +x run_benchmarks.sh
./run_benchmarks.sh
```

### Output

The scripts output raw results under `benchmark/results/` and compile a comparative markdown table into `benchmark/results/RESULTS.md`.

---

## Tier 2: Running Rust Micro-benchmarks

Internal Rust micro-benchmarks are managed by `criterion` (v0.8.2). These measure execution times of Rullst's internal components:
- Router matching (simple path & nested URL parameters)
- HTML macro rendering (static template vs dynamic rendering loop)
- Security middleware (WAF middleware overhead)

To run the internal micro-benchmarks:

```bash
cargo bench --workspace
```

Criterion generates visual HTML reports, which can be viewed at `target/criterion/report/index.html`.
