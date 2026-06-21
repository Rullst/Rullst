#!/bin/bash
set -e

RESULTS_DIR="results"
mkdir -p $RESULTS_DIR

capture_stats() {
    local label=$1
    local duration=$2
    echo "Capturing docker stats for $label for $duration seconds..."

    # Run stats capture in background
    docker stats --format "{{.Name}},{{.CPUPerc}},{{.MemUsage}}" poem warp db > "$RESULTS_DIR/stats_${label}.csv" &
    STATS_PID=$!

    sleep $duration
    kill $STATS_PID
}

echo "Starting Tier 3 (Resource Efficiency)..."

# 1. Capture Idle Stats
echo "Capturing Idle Stats (10s)..."
capture_stats "idle" 10

# 2. Capture Under Load Stats
echo "Starting load to capture stats..."
# Run a quick load test in background and capture stats simultaneously
docker run --rm --network host alpine/bombardier -c 125 -d 30s -l http://127.0.0.1:3001/json > /dev/null &
docker run --rm --network host alpine/bombardier -c 125 -d 30s -l http://127.0.0.1:3002/json > /dev/null &

capture_stats "load" 30

echo "Tier 3 Stats capture complete."
