#!/bin/bash
set -e

mkdir -p results

# Tier 3 (Resource Efficiency)
echo "Starting Tier 3 Benchmarks (Resource Efficiency)..."

capture_stats() {
    local target=$1
    local out_file=$2
    local duration=$3

    echo "Capturing stats for $target ($duration)..."
    # Capture docker stats over the duration
    timeout $duration docker stats --format "{{.Container}},{{.CPUPerc}},{{.MemUsage}}" > "results/${out_file}" || true
}

# 1. Idle Stats
echo "Capturing idle stats (30s)..."
capture_stats "benchmark-axum-1" "axum_tier3_idle.csv" "30s" &
capture_stats "benchmark-actix-1" "actix_tier3_idle.csv" "30s" &
wait

# 2. Max Load Stats
echo "Capturing max load stats while running bombardier (30s load)..."

# Start stats capture in background
capture_stats "benchmark-axum-1" "axum_tier3_load.csv" "40s" &
capture_stats "benchmark-actix-1" "actix_tier3_load.csv" "40s" &

# Run bombardier in parallel to generate load
echo "Applying load to axum..."
docker run --rm --network host alpine/bombardier -c 1000 -d 30s "http://localhost:8000/json" > /dev/null &
echo "Applying load to actix..."
docker run --rm --network host alpine/bombardier -c 1000 -d 30s "http://localhost:8001/json" > /dev/null &

wait
echo "Done running Tier 3 resource capture."
