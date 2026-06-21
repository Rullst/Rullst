#!/bin/bash

# Make sure we're in the benchmark directory
cd "$(dirname "$0")"

mkdir -p results9

FRAMEWORKS=("express" "fastify" "hono" "nestjs" "nextjs")
PORTS=(3001 3002 3003 3004 3005)

ENDPOINTS=("/text" "/json" "/db-single" "/html")

echo "======================================"
echo " Starting Benchmarks Tier 1 & 4"
echo "======================================"

for i in "${!FRAMEWORKS[@]}"; do
    FRAMEWORK="${FRAMEWORKS[$i]}"
    PORT="${PORTS[$i]}"

    echo ""
    echo "--------------------------------------"
    echo " Framework: $FRAMEWORK on port $PORT"
    echo "--------------------------------------"

    # Start the container
    docker compose up -d --build $FRAMEWORK

    echo "Waiting 15 seconds for $FRAMEWORK to start..."
    sleep 15

    for ENDPOINT in "${ENDPOINTS[@]}"; do
        echo ">> Tier 1 (Load): $FRAMEWORK $ENDPOINT (10s)"
        docker run --rm --network host alpine/bombardier -c 100 -d 10s http://127.0.0.1:$PORT$ENDPOINT > results9/${FRAMEWORK}_tier1_${ENDPOINT//\//}.txt
    done

    echo ">> Tier 4 (Stress/Memory Leak): $FRAMEWORK /json (10m extreme stress)"
    docker run --rm --network host alpine/bombardier -c 500 -d 10m http://127.0.0.1:$PORT/json > results9/${FRAMEWORK}_tier4.txt

    echo "Stopping $FRAMEWORK container..."
    docker compose stop $FRAMEWORK
    docker compose rm -f $FRAMEWORK
    echo "Done with $FRAMEWORK."
done

echo "All Tier 1 & 4 benchmarks completed."
