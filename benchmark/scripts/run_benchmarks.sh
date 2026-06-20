#!/bin/bash
set -e

# Make sure bombardier is available
if ! command -v bombardier &> /dev/null; then
    echo "Bombardier not found. Downloading..."
    wget -qO- https://github.com/codesenberg/bombardier/releases/download/v1.2.6/bombardier-linux-amd64 > bombardier
    chmod +x bombardier
    export PATH=$PATH:$(pwd)
fi

echo "# Rullst Benchmark Results (Tier 1, 3, 4)" > RESULTS10.md
echo "Date: $(date)" >> RESULTS10.md
echo "" >> RESULTS10.md

FRAMEWORKS=("laravel" "symfony" "django" "fastapi")
PORTS=(8001 8002 8003 8004)
ENDPOINTS=("/text" "/json" "/db-single" "/html")

# Tier 1 - Fast Load
echo "## Tier 1: Fast Load (10s, 125 connections)" >> RESULTS10.md

for i in "${!FRAMEWORKS[@]}"; do
    fw="${FRAMEWORKS[$i]}"
    port="${PORTS[$i]}"
    echo "### $fw" >> RESULTS10.md
    for ep in "${ENDPOINTS[@]}"; do
        echo "Testing $fw on $ep..."
        bombardier -c 125 -d 10s -o json http://localhost:$port$ep > res.json
        RPS=$(grep -o '"mean":[^,]*' res.json | head -1 | cut -d: -f2)
        LAT=$(grep -o '"mean":[^,]*' res.json | tail -n +2 | head -1 | cut -d: -f2)
        # Convert lat to ms
        LAT_MS=$(awk "BEGIN {print $LAT/1000}")
        echo "- **$ep**: RPS: $RPS | Latency: ${LAT_MS}ms" >> RESULTS10.md
    done
    echo "" >> RESULTS10.md
done

# Tier 4 - Extreme Stress
echo "## Tier 4: Extreme Stress (10m, 500 connections)" >> RESULTS10.md

for i in "${!FRAMEWORKS[@]}"; do
    fw="${FRAMEWORKS[$i]}"
    port="${PORTS[$i]}"
    echo "### $fw" >> RESULTS10.md
    # We will test the JSON endpoint for stress testing
    echo "Stress testing $fw on /json for 10 minutes..."
    bombardier -c 500 -d 10m -o json http://localhost:$port/json > res.json
    RPS=$(grep -o '"mean":[^,]*' res.json | head -1 | cut -d: -f2)
    ERRORS=$(grep -o '"req1xx":0,"req2xx":[^,]*,"req3xx":0,"req4xx":0,"req5xx":[^,]*' res.json)
    echo "- **/json**: RPS: $RPS | Errors: $ERRORS" >> RESULTS10.md
    echo "" >> RESULTS10.md
done

echo "Benchmarks finished!"
