#!/bin/bash
set -e

# Run tests
BOMBARDIER_IMAGE="alpine/bombardier"
RESULTS_DIR="results"
mkdir -p $RESULTS_DIR

echo "Starting Bombardier benchmarking..."

run_benchmark() {
    local framework=$1
    local port=$2
    local endpoint=$3
    local duration=$4
    local connections=$5
    local label=$6

    echo "Running $label on $framework ($endpoint) for $duration..."
    docker run --rm --network host $BOMBARDIER_IMAGE -c $connections -d $duration -l http://127.0.0.1:$port$endpoint > "$RESULTS_DIR/${framework}_${label}.txt"
}

# Tier 1 (Fast Load): 10 seconds, 125 connections
for fw in poem warp; do
    if [ "$fw" == "poem" ]; then port=3001; else port=3002; fi

    echo "--- Benchmarking $fw (Tier 1) ---"
    for ep in /text /json /db-single /html; do
        run_benchmark $fw $port $ep "10s" 125 "tier1$(echo $ep | tr -d '/')"
    done
done

# Tier 4 (Extreme Stress): 10 minutes, 1000 connections
for fw in poem warp; do
    if [ "$fw" == "poem" ]; then port=3001; else port=3002; fi

    echo "--- Benchmarking $fw (Tier 4 Stress) ---"
    # We test on json for stress test
    run_benchmark $fw $port "/json" "10m" 1000 "tier4_stress"
done

echo "Benchmarking complete."
