#!/bin/bash

RESULTS_FILE="RESULTS7.md"
STATS_FILE="stats_temp.txt"

echo "## Tier 3 Results (Resource Efficiency)" >> $RESULTS_FILE
echo "" >> $RESULTS_FILE

echo "Starting containers for stats capture..."
docker compose up -d --build

echo "Waiting for containers to initialize (30 seconds)..."
sleep 30

echo "### Idle Resource Usage" >> $RESULTS_FILE
echo "\`\`\`" >> $RESULTS_FILE
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-springboot-1 benchmark-quarkus-1 >> $RESULTS_FILE
echo "\`\`\`" >> $RESULTS_FILE
echo "" >> $RESULTS_FILE

echo "Starting load to measure max usage..."
# Start a background bombardier process for Spring Boot
docker run --rm --network host alpine/bombardier -c 100 -d 30s http://localhost:8080/text &
PID_SPRING=$!

# Start a background bombardier process for Quarkus
docker run --rm --network host alpine/bombardier -c 100 -d 30s http://localhost:8081/text &
PID_QUARKUS=$!

echo "Waiting 15 seconds for load to peak..."
sleep 15

echo "### Max Load Resource Usage" >> $RESULTS_FILE
echo "\`\`\`" >> $RESULTS_FILE
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" benchmark-springboot-1 benchmark-quarkus-1 >> $RESULTS_FILE
echo "\`\`\`" >> $RESULTS_FILE
echo "" >> $RESULTS_FILE

# Wait for tests to finish
wait $PID_SPRING
wait $PID_QUARKUS

echo "Cleaning up..."
docker compose down

echo "Stats captured."
