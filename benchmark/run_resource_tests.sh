#!/bin/bash

# Exit on error
set -e

mkdir -p results

echo "Starting Docker containers..."
docker compose up -d --build

echo "Waiting for services to be ready (20s)..."
sleep 20

echo "=== TIER 3 (Resource Efficiency - Idle) ==="
# Capture stats at rest
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-go-gin-1 benchmark-go-fiber-1 benchmark-aspnet-core-1 > results/tier3_idle.txt
cat results/tier3_idle.txt

echo "=== TIER 3 (Resource Efficiency - Under Load) ==="
# Start a load test in the background
echo "Starting background load on all frameworks..."

docker run --rm --network host alpine/bombardier -c 500 -d 30s http://127.0.0.1:8081/json > /dev/null &
PID1=$!

docker run --rm --network host alpine/bombardier -c 500 -d 30s http://127.0.0.1:8082/json > /dev/null &
PID2=$!

docker run --rm --network host alpine/bombardier -c 500 -d 30s http://127.0.0.1:8083/json > /dev/null &
PID3=$!

# Wait 10 seconds for load to peak
echo "Waiting 10s for load to peak..."
sleep 10

# Capture stats under load
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-go-gin-1 benchmark-go-fiber-1 benchmark-aspnet-core-1 > results/tier3_load.txt
cat results/tier3_load.txt

# Wait for load tests to finish
echo "Waiting for load tests to finish..."
wait $PID1
wait $PID2
wait $PID3

echo "Tests completed. Stopping containers..."
docker compose down
echo "Done."
