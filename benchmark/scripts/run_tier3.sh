#!/bin/bash

set -e

RESULTS_DIR="benchmark/results"
mkdir -p "$RESULTS_DIR"

echo "=================================================="
echo "    Tier 3 Benchmark Automation (CPU/RAM)       "
echo "=================================================="

cd benchmark && docker compose up -d
sleep 5

echo "Capturing IDLE stats..."
# Capture idle stats
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-leptos-1 benchmark-dioxus-1 > "../$RESULTS_DIR/tier3_idle_stats.txt"

echo "Applying load for LOAD stats capture..."
# Run bombardier in the background to generate load while we capture stats
docker run --rm --network host alpine/bombardier -c 500 -d 30s "http://localhost:3001/text" > /dev/null 2>&1 &
BOMB_PID_LEPTOS=$!

sleep 10 # let load build up
echo "Capturing LOAD stats for Leptos..."
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-leptos-1 >> "../$RESULTS_DIR/tier3_load_stats.txt"

wait $BOMB_PID_LEPTOS

# Now for dioxus
docker run --rm --network host alpine/bombardier -c 500 -d 30s "http://localhost:3002/text" > /dev/null 2>&1 &
BOMB_PID_DIOXUS=$!

sleep 10
echo "Capturing LOAD stats for Dioxus..."
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-dioxus-1 >> "../$RESULTS_DIR/tier3_load_stats.txt"

wait $BOMB_PID_DIOXUS

echo "Cleaning up containers..."
docker compose down

echo "Done! Tier 3 stats saved."
