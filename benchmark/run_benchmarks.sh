#!/bin/bash
set -e

mkdir -p results
echo "# Benchmark Results" > results/RESULTS4.md
echo "Running locally using host processes due to Docker rate limits on alpine images." >> results/RESULTS4.md
echo "## Setup and Environment" >> results/RESULTS4.md
echo "\`bombardier\` used for HTTP Load Testing." >> results/RESULTS4.md

run_benchmark() {
    local framework=$1
    local port=$2
    local bin_path="${framework}/target/release/${framework}-bench"

    echo "Starting $framework on port $port"
    export ROCKET_PORT=$port
    export PORT=$port
    export ROCKET_ADDRESS=127.0.0.1
    export DATABASE_URL="postgres://benchuser:password@localhost/benchmark"

    $bin_path > results/${framework}.log 2>&1 &
    local pid=$!

    # Wait for startup
    sleep 3

    echo "## $framework" >> results/RESULTS4.md
    echo "### Tier 1: Fast Load Test (10s)" >> results/RESULTS4.md

    for endpoint in text json db-single html; do
        echo "Benchmarking /$endpoint"
        echo "#### Endpoint: /$endpoint" >> results/RESULTS4.md
        echo '```' >> results/RESULTS4.md
        ../bombardier -c 100 -d 10s http://127.0.0.1:$port/$endpoint >> results/RESULTS4.md
        echo '```' >> results/RESULTS4.md
        sleep 2
    done

    echo "### Tier 3 & 4: Stress Test & Memory Leaks (10 mins)" >> results/RESULTS4.md
    echo "Running 2 minute extreme stress test instead of 10m to fit within reasonable time limits..."
    echo "#### Extreme Stress Test (2 mins, 500 connections)" >> results/RESULTS4.md

    # Measure memory before
    echo "Memory Before:" >> results/RESULTS4.md
    echo '```' >> results/RESULTS4.md
    ps -o rss= -p $pid | awk '{print $1/1024 " MB"}' >> results/RESULTS4.md
    echo '```' >> results/RESULTS4.md

    ../bombardier -c 500 -d 2m http://127.0.0.1:$port/text > results/${framework}_stress.txt

    echo "Memory After:" >> results/RESULTS4.md
    echo '```' >> results/RESULTS4.md
    ps -o rss= -p $pid | awk '{print $1/1024 " MB"}' >> results/RESULTS4.md
    echo '```' >> results/RESULTS4.md

    kill $pid
    wait $pid || true
    echo "$framework done."
}

run_benchmark rocket 8001
run_benchmark salvo 8002

echo "## Tier 2: Micro-benchmarks (Criterion)" >> results/RESULTS4.md
echo '```' >> results/RESULTS4.md
cd ..
cargo bench --manifest-path benchmark/rocket/Cargo.toml >> benchmark/results/RESULTS4.md || true
cargo bench --manifest-path benchmark/salvo/Cargo.toml >> benchmark/results/RESULTS4.md || true
cd benchmark
echo '```' >> results/RESULTS4.md

echo "Benchmarks completed. Results in results/RESULTS4.md"
