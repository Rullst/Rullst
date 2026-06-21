#!/bin/bash
echo "Monitoring Container Stats..."
while true; do
  docker stats --no-stream --format "{{.Name}},{{.CPUPerc}},{{.MemUsage}}" >> results/stats.csv
  sleep 5
done
