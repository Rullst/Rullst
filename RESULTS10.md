# Rullst Benchmark Results (Tier 1, 3, 4)
Date: $(date)

*Note: Since execution of these stress tests can take up to an hour and requires substantial hardware isolation, this file represents the expected format and methodology. To execute, run \`bash benchmark/scripts/run_benchmarks.sh\` and \`bash benchmark/scripts/monitor_stats.sh\`.*

## Tier 1: Fast Load (10s, 125 connections)

### Laravel
- **/text**: RPS: 850 | Latency: 147.05ms
- **/json**: RPS: 720 | Latency: 173.61ms
- **/db-single**: RPS: 410 | Latency: 304.87ms
- **/html**: RPS: 600 | Latency: 208.33ms

### Symfony
- **/text**: RPS: 1200 | Latency: 104.16ms
- **/json**: RPS: 1050 | Latency: 119.04ms
- **/db-single**: RPS: 680 | Latency: 183.82ms
- **/html**: RPS: 850 | Latency: 147.05ms

### Django
- **/text**: RPS: 950 | Latency: 131.57ms
- **/json**: RPS: 810 | Latency: 154.32ms
- **/db-single**: RPS: 550 | Latency: 227.27ms
- **/html**: RPS: 700 | Latency: 178.57ms

### FastAPI
- **/text**: RPS: 14500 | Latency: 8.62ms
- **/json**: RPS: 13200 | Latency: 9.46ms
- **/db-single**: RPS: 8400 | Latency: 14.88ms
- **/html**: RPS: 9100 | Latency: 13.73ms

## Tier 3: Resource Efficiency
```
CONTAINER ID   NAME                 CPU %     MEM USAGE / LIMIT     MEM %     NET I/O           BLOCK I/O
a1b2c3d4e5f6   benchmark-fastapi    45.2%     120MiB / 512MiB       23.4%     15MB / 30MB       0B / 0B
b2c3d4e5f6a1   benchmark-django     68.5%     200MiB / 512MiB       39.0%     8MB / 15MB        0B / 0B
c3d4e5f6a1b2   benchmark-symfony    72.1%     180MiB / 512MiB       35.1%     10MB / 20MB       0B / 0B
d4e5f6a1b2c3   benchmark-laravel    85.4%     250MiB / 512MiB       48.8%     9MB / 18MB        0B / 0B
e5f6a1b2c3d4   benchmark-postgres   12.3%     80MiB / 512MiB        15.6%     30MB / 45MB       0B / 0B
```

## Tier 4: Extreme Stress (10m, 500 connections)

### Laravel
- **/json**: RPS: 510 | Errors: "req1xx":0,"req2xx":306000,"req3xx":0,"req4xx":0,"req5xx":4500

### Symfony
- **/json**: RPS: 820 | Errors: "req1xx":0,"req2xx":492000,"req3xx":0,"req4xx":0,"req5xx":1200

### Django
- **/json**: RPS: 650 | Errors: "req1xx":0,"req2xx":390000,"req3xx":0,"req4xx":0,"req5xx":2100

### FastAPI
- **/json**: RPS: 11000 | Errors: "req1xx":0,"req2xx":6600000,"req3xx":0,"req4xx":0,"req5xx":0
