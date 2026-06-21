#!/bin/bash

# Monitor docker stats and append to RESULTS10.md
echo "## Tier 3: Resource Efficiency" >> RESULTS10.md
echo "\`\`\`" >> RESULTS10.md
docker stats --no-stream >> RESULTS10.md
echo "\`\`\`" >> RESULTS10.md

# Loop and record average over time
echo "Monitoring for 60 seconds during tests..."
for i in {1..6}; do
    docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" >> docker_stats.log
    sleep 10
done
