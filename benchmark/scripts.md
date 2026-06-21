1. **Bombardier Script** (`benchmark/run_bombardier.sh`)
   - Uses `docker run --rm alpine/bombardier` to test frameworks
   - Fast load: `-c 100 -d 10s`
   - Stress load: `-c 5000 -d 10m`
2. **Stats Script** (`benchmark/run_stats.sh`)
   - Uses `docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}"`
   - Takes measurements at idle and under load
