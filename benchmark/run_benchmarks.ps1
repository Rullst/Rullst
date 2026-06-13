$ErrorActionPreference = "Stop"

# Create results directory
if (!(Test-Path "results")) {
    New-Item -ItemType Directory -Path "results" | Out-Null
}

$ports = @{
    "rullst" = 3000
    "axum" = 3000
    "actix" = 3000
    "loco" = 3000
    "rocket" = 8000
    "leptos" = 3000
    "gin" = 3000
    "fiber" = 3000
    "django" = 8000
    "laravel" = 8000
    "nextjs" = 3000
    "nestjs" = 3000
    "zap" = 3000
    "springboot" = 8080
    "rails" = 3000
}

$frameworks = @(
    "rullst", "axum", "actix", "loco", "rocket", "leptos", "gin", "fiber", "django", "laravel", "nextjs", "nestjs", "zap", "springboot", "rails"
)

Write-Host "=================================================="
Write-Host "Starting Black-Box HTTP Benchmark Suite (Bombardier v2.0.3)"
Write-Host "=================================================="

Write-Host "Building bombardier load tester container..."
docker compose build bombardier

foreach ($id in $frameworks) {
    $port = $ports[$id]
    Write-Host ""
    Write-Host "--------------------------------------------------"
    Write-Host "Benchmarking framework: $id"
    Write-Host "--------------------------------------------------"

    Write-Host "Building service $id..."
    docker compose build $id
    
    Write-Host "Starting service $id..."
    docker compose up -d $id

    Write-Host "Waiting 5 seconds for $id to initialize..."
    Start-Sleep -Seconds 5

    Write-Host "Running Plaintext benchmark (http://$id`:$port/)..."
    bombardier -c 125 -d 10s "http://$id`:$port/" > "results/${id}_plaintext.txt"

    Write-Host "Running JSON benchmark (http://$id`:$port/json)..."
    bombardier -c 125 -d 10s "http://$id`:$port/json" > "results/${id}_json.txt"

    Write-Host "Stopping and cleaning up service $id..."
    docker compose stop $id
    docker compose rm -f $id
}

Write-Host ""
Write-Host "All benchmarks completed. Generating results..."
python parse_results.py results
