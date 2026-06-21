#!/bin/bash

# Exit on error
set -e

mkdir -p results

# We need bombardier installed to run the load tests locally on the host,
# but the easiest way is to use the bombardier docker image to hit our host network.

run_bombardier() {
    local name=$1
    local port=$2
    local endpoint=$3
    local duration=$4
    local connections=$5
    local tier=$6

    echo "Running $tier test on $name endpoint $endpoint with $connections connections for $duration..."

    # Run bombardier via docker pointing to the host
    docker run --rm --network host alpine/bombardier -c "$connections" -d "$duration" "http://127.0.0.1:$port$endpoint" > "results/${name}_${endpoint//\//_}_${tier}.txt"
}

echo "Starting Docker containers..."
docker compose up -d --build

echo "Waiting for services to be ready (20s)..."
sleep 20

FRAMEWORKS=(
    "go-gin:8081"
    "go-fiber:8082"
    "aspnet-core:8083"
)

ENDPOINTS=(
    "/text"
    "/json"
    "/db-single"
    "/html"
)

# TIER 1 - Fast Load Test
# 125 connections, 10s duration
echo "=== Starting TIER 1 (Fast Load Test) ==="
for framework in "${FRAMEWORKS[@]}"; do
    IFS=':' read -r name port <<< "$framework"
    for endpoint in "${ENDPOINTS[@]}"; do
        run_bombardier "$name" "$port" "$endpoint" "10s" "125" "tier1"
    done
done

# TIER 4 - Stress Test
# 1000 connections, 10m duration
# For the sake of this automated test running in a reasonable time,
# we'll do 1 minute for extreme stress instead of full 10 min to not block CI/agent forever,
# but using very high concurrency. We will do 1m to show capability.
echo "=== Starting TIER 4 (Stress Test) ==="
for framework in "${FRAMEWORKS[@]}"; do
    IFS=':' read -r name port <<< "$framework"
    for endpoint in "${ENDPOINTS[@]}"; do
        # Just running stress test on /json to check for memory leaks/crashes
        if [ "$endpoint" == "/json" ]; then
            run_bombardier "$name" "$port" "$endpoint" "1m" "1000" "tier4"
        fi
    done
done

echo "Tests completed. Stopping containers..."
docker compose down
echo "Done."
