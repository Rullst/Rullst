#!/bin/bash

set -e

RESULTS_DIR="benchmark/results"
mkdir -p "$RESULTS_DIR"

echo "=================================================="
echo "    Tier 1 & Tier 4 Benchmark Automation        "
echo "=================================================="

# Function to run bombardier with specific parameters
run_bombardier() {
    local target_name=$1
    local port=$2
    local endpoint=$3
    local duration=$4
    local connections=$5
    local output_file=$6

    echo "Running benchmark against $target_name on endpoint /$endpoint ($connections connections, ${duration}s)"
    docker run --rm -it --network host alpine/bombardier -c "$connections" -d "${duration}s" "http://localhost:${port}/${endpoint}" > "$RESULTS_DIR/$output_file"
    echo "Saved output to $RESULTS_DIR/$output_file"
    echo "--------------------------------------------------"
}

# Ensure containers are running
echo "Starting containers..."
cd benchmark && docker compose up -d
sleep 5 # Wait for initialization

# Setup database dummy table for ORM test
echo "Setting up database..."
docker exec -it benchmark-db-1 psql -U postgres -d bench -c "CREATE TABLE IF NOT EXISTS records (id SERIAL PRIMARY KEY, name VARCHAR(50));"
docker exec -it benchmark-db-1 psql -U postgres -d bench -c "INSERT INTO records (name) VALUES ('Test Record') ON CONFLICT DO NOTHING;"

echo "Containers and DB are ready!"

# --- TIER 1: Global Load (10s, 125 connections) ---
echo "--- Starting TIER 1 (Global Load) ---"

# Leptos Tier 1
run_bombardier "Leptos" 3001 "text" 10 125 "leptos_tier1_text.txt"
run_bombardier "Leptos" 3001 "json" 10 125 "leptos_tier1_json.txt"
run_bombardier "Leptos" 3001 "db-single" 10 125 "leptos_tier1_db.txt"
run_bombardier "Leptos" 3001 "html" 10 125 "leptos_tier1_html.txt"

# Dioxus Tier 1
run_bombardier "Dioxus" 3002 "text" 10 125 "dioxus_tier1_text.txt"
run_bombardier "Dioxus" 3002 "json" 10 125 "dioxus_tier1_json.txt"
run_bombardier "Dioxus" 3002 "db-single" 10 125 "dioxus_tier1_db.txt"
run_bombardier "Dioxus" 3002 "html" 10 125 "dioxus_tier1_html.txt"


# --- TIER 4: Resilience/Stress (10 minutes, 1000 connections) ---
echo "--- Starting TIER 4 (Resilience / Stress / Memory Leaks) ---"

# Leptos Tier 4
run_bombardier "Leptos" 3001 "text" 600 1000 "leptos_tier4_stress.txt"

# Dioxus Tier 4
run_bombardier "Dioxus" 3002 "text" 600 1000 "dioxus_tier4_stress.txt"

echo "Cleaning up containers..."
docker compose down

echo "Done! Results saved in $RESULTS_DIR"
