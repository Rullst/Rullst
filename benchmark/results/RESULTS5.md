# Benchmark Results: Poem vs Warp

*Note: Automated container benchmarks (Tier 1, 3, and 4) were configured but skipped execution due to Docker Hub unauthenticated pull rate limits on the CI environment.*

## Tier 2: Micro-benchmarks
(Criterion - Local Execution on Rust 1.85)

| Framework | JSON Serialization (ns) | Router Match (ns) |
|-----------|-------------------------|-------------------|
| Poem | ~55-60 ns | ~950-1000 ns |
| Warp | ~50-55 ns | ~600-650 ns |

## Tier 1: Fast Load
(125 connections, 10 seconds) - *Requires Docker*

| Framework | Endpoint | RPS | Latency (avg) |
|-----------|----------|-----|---------------|
| Poem | /text | - | - |
| Warp | /text | - | - |
| Poem | /json | - | - |
| Warp | /json | - | - |
| Poem | /db-single | - | - |
| Warp | /db-single | - | - |
| Poem | /html | - | - |
| Warp | /html | - | - |

## Tier 3: Resource Efficiency
(CPU / RAM - Idle vs Load) - *Requires Docker*

| State | Framework | CPU % | Memory |
|-------|-----------|-------|--------|
| idle | poem | - | - |
| load | poem | - | - |
| idle | warp | - | - |
| load | warp | - | - |

## Tier 4: Resilience / Stress Test
(1000 connections, 10 minutes) - *Requires Docker*

| Framework | RPS | Max Latency | Errors |
|-----------|-----|-------------|--------|
| Poem | - | - | - |
| Warp | - | - | - |
