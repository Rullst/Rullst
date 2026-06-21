#!/bin/bash
set -e

# Frameworks to test
FRAMEWORKS=("rails_app" "phoenix_app")
PORTS=("3000" "4000")
ENDPOINTS=("/text" "/json" "/db-single" "/html")

# Create results directory if it doesn't exist
mkdir -p results

# Start docker compose
echo "Starting services..."
docker compose up -d

# Wait for services to be ready
echo "Waiting for services to boot..."
sleep 20

for i in "${!FRAMEWORKS[@]}"; do
  FRAMEWORK="${FRAMEWORKS[$i]}"
  PORT="${PORTS[$i]}"

  echo "====================================="
  echo "Testing $FRAMEWORK on port $PORT"
  echo "====================================="

  # Tier 1: Fast Load
  for ENDPOINT in "${ENDPOINTS[@]}"; do
    echo "Running Tier 1 on $FRAMEWORK $ENDPOINT..."
    docker run --rm --network host alpine/bombardier -c 100 -d 10s "http://localhost:$PORT$ENDPOINT" > results/${FRAMEWORK}_tier1_${ENDPOINT//\//}.txt
  done

  # Tier 4: Stress / Resilience (Reduced to 2 mins for PR validation as requested)
  echo "Running Tier 4 (Stress) on $FRAMEWORK /text..."
  docker run --rm --network host alpine/bombardier -c 5000 -d 2m "http://localhost:$PORT/text" > results/${FRAMEWORK}_tier4.txt
done

# Stop services
echo "Stopping services..."
docker compose down

echo "Done! Results saved in benchmark/results/"
