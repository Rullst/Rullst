#!/bin/bash

# Configuration
DURATION_FAST="10s"
DURATION_STRESS="10m"
CONNECTIONS=100
STRESS_CONNECTIONS=10000
RESULTS_FILE="RESULTS7.md"

ENDPOINTS=(
  "/text"
  "/json"
  "/db-single"
  "/html"
)

FRAMEWORKS=(
  "Spring Boot|http://localhost:8080"
  "Quarkus|http://localhost:8081"
)

echo "# Java Benchmark Results" > $RESULTS_FILE
echo "" >> $RESULTS_FILE
echo "## Configuration" >> $RESULTS_FILE
echo "- Tier 1 (Fast Load): $DURATION_FAST duration, $CONNECTIONS connections" >> $RESULTS_FILE
echo "- Tier 4 (Stress): $DURATION_STRESS duration, $STRESS_CONNECTIONS connections" >> $RESULTS_FILE
echo "" >> $RESULTS_FILE

run_bombardier() {
  local framework_name=$1
  local url=$2
  local duration=$3
  local connections=$4
  local endpoint=$5
  local tier_name=$6

  echo "### $tier_name: $framework_name - $endpoint" >> $RESULTS_FILE
  echo "\`\`\`" >> $RESULTS_FILE

  echo "Running $tier_name for $framework_name at $endpoint..."
  bombardier -c $connections -d $duration $url$endpoint >> $RESULTS_FILE 2>&1

  echo "\`\`\`" >> $RESULTS_FILE
  echo "" >> $RESULTS_FILE
}

echo "Starting Database and Containers..."
docker compose up -d --build

echo "Waiting for containers to be ready (30 seconds)..."
sleep 30

echo "## Tier 1 Results (Fast Load)" >> $RESULTS_FILE
for framework in "${FRAMEWORKS[@]}"; do
  IFS="|" read -r name url <<< "$framework"
  for endpoint in "${ENDPOINTS[@]}"; do
    run_bombardier "$name" "$url" "$DURATION_FAST" "$CONNECTIONS" "$endpoint" "Tier 1"
  done
done

echo "## Tier 4 Results (Stress/Resilience)" >> $RESULTS_FILE
for framework in "${FRAMEWORKS[@]}"; do
  IFS="|" read -r name url <<< "$framework"
  # For stress test, just hit the /text endpoint to maximize load on the router
  run_bombardier "$name" "$url" "$DURATION_STRESS" "$STRESS_CONNECTIONS" "/text" "Tier 4"
done

echo "Benchmarks completed. Results saved in $RESULTS_FILE."

echo "Cleaning up..."
docker compose down
