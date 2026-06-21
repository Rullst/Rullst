#!/bin/bash
set -e

mkdir -p results

echo "Waiting for services to be ready..."
sleep 10 # Adjust if necessary for docker-compose to fully boot up services

run_bombardier() {
    local target=$1
    local port=$2
    local endpoint=$3
    local duration=$4
    local out_file=$5

    echo "Running benchmark for $target on $endpoint ($duration)..."
    docker run --rm --network host alpine/bombardier -c 125 -d $duration "http://localhost:$port$endpoint" > "results/${out_file}"
}

# Tier 1 (Fast load)
echo "Starting Tier 1 Benchmarks (10s load)..."
run_bombardier "axum" 8000 "/text" "10s" "axum_tier1_text.txt"
run_bombardier "axum" 8000 "/json" "10s" "axum_tier1_json.txt"
run_bombardier "axum" 8000 "/html" "10s" "axum_tier1_html.txt"
run_bombardier "axum" 8000 "/db-single" "10s" "axum_tier1_db.txt"

run_bombardier "actix" 8001 "/text" "10s" "actix_tier1_text.txt"
run_bombardier "actix" 8001 "/json" "10s" "actix_tier1_json.txt"
run_bombardier "actix" 8001 "/html" "10s" "actix_tier1_html.txt"
run_bombardier "actix" 8001 "/db-single" "10s" "actix_tier1_db.txt"

# Tier 4 (Stress/Resilience - 10 minutes)
# We will just test /json for the 10-minute stress test to look for memory leaks/breaking
echo "Starting Tier 4 Benchmarks (10m stress load)..."
# Setting up concurrent bombardier might skew results if run exactly at the same time,
# so we run them sequentially but long
run_bombardier "axum" 8000 "/json" "10m" "axum_tier4_json_stress.txt"
run_bombardier "actix" 8001 "/json" "10m" "actix_tier4_json_stress.txt"

echo "Done running bombardier tests."
