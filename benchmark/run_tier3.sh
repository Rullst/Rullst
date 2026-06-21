#!/bin/bash

# Make sure we're in the benchmark directory
cd "$(dirname "$0")"

mkdir -p results9

FRAMEWORKS=("express" "fastify" "hono" "nestjs" "nextjs")
PORTS=(3001 3002 3003 3004 3005)

echo "======================================"
echo " Starting Benchmarks Tier 3 (CPU/RAM)"
echo "======================================"

for i in "${!FRAMEWORKS[@]}"; do
    FRAMEWORK="${FRAMEWORKS[$i]}"
    PORT="${PORTS[$i]}"

    echo ""
    echo "--------------------------------------"
    echo " Framework: $FRAMEWORK on port $PORT"
    echo "--------------------------------------"

    docker compose up -d --build $FRAMEWORK
    echo "Waiting 15 seconds for $FRAMEWORK to start..."
    sleep 15

    # Idle state
    echo ">> Capturing Idle State for 10s..."
    docker stats --no-stream benchmark-$FRAMEWORK-1 > results9/${FRAMEWORK}_tier3_idle.txt

    # Peak load state (run bombardier in background, then capture stats)
    echo ">> Applying Load for Peak State capture..."
    docker run --rm --network host alpine/bombardier -c 200 -d 30s http://127.0.0.1:$PORT/json &
    BOMBARDIER_PID=$!

    echo "Waiting 10s for load to stabilize..."
    sleep 10

    echo ">> Capturing Peak State..."
    docker stats --no-stream benchmark-$FRAMEWORK-1 > results9/${FRAMEWORK}_tier3_peak.txt

    wait $BOMBARDIER_PID

    echo "Stopping $FRAMEWORK container..."
    docker compose stop $FRAMEWORK
    docker compose rm -f $FRAMEWORK
    echo "Done with $FRAMEWORK."
done

echo "All Tier 3 benchmarks completed."
