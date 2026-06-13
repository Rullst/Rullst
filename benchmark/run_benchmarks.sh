#!/bin/bash
set -e

# Create results directory
mkdir -p results

declare -A ports
ports=(
  ["rullst"]=3000
  ["axum"]=3000
  ["actix"]=3000
  ["loco"]=3000
  ["rocket"]=8000
  ["leptos"]=3000
  ["gin"]=3000
  ["fiber"]=3000
  ["django"]=8000
  ["laravel"]=8000
  ["nextjs"]=3000
  ["nestjs"]=3000
  ["zap"]=3000
  ["springboot"]=8080
  ["rails"]=3000
)

frameworks=("rullst" "axum" "actix" "loco" "rocket" "leptos" "gin" "fiber" "django" "laravel" "nextjs" "nestjs" "zap" "springboot" "rails")

echo "=================================================="
echo "Starting Black-Box HTTP Benchmark Suite (Bombardier v2.0.3)"
echo "=================================================="

echo "Building bombardier load tester container..."

for id in "${frameworks[@]}"; do
    port=${ports[$id]}
    echo ""
    echo "--------------------------------------------------"
    echo "Benchmarking framework: $id"
    echo "--------------------------------------------------"

    echo "Building service $id..."
    docker compose build $id
    
    echo "Starting service $id..."
    docker compose up -d $id

    echo "Waiting 5 seconds for $id to initialize..."
    sleep 5

    echo "Running Plaintext benchmark (http://$id:$port/)..."
    bombardier -c 125 -d 10s "http://$id:$port/" > "results/${id}_plaintext.txt"

    echo "Running JSON benchmark (http://$id:$port/json)..."
    bombardier -c 125 -d 10s "http://$id:$port/json" > "results/${id}_json.txt"

    echo "Stopping and cleaning up service $id..."
    docker compose stop $id
    docker compose rm -f $id
done

echo ""
echo "All benchmarks completed. Generating results..."
python3 parse_results.py results
