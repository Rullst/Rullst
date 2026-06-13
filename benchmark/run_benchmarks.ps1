# PowerShell script to run the multi-language benchmark suite
# Make sure Docker is running on your machine.

$ErrorActionPreference = "Stop"

# Create results directory
New-Item -ItemType Directory -Force -Path "results" | Out-Null

$frameworks = @(
    # --- Fast & Ready (Rust/Go/Python/Node/Zig microframeworks) ---
    @{ id = "axum"; port = 3000 },
    @{ id = "actix"; port = 3000 },
    @{ id = "rocket"; port = 8000 },
    @{ id = "rullst"; port = 3000 },
    @{ id = "gin"; port = 3000 },
    @{ id = "fiber"; port = 3000 },
    @{ id = "django"; port = 8000 },
    @{ id = "nextjs"; port = 3000 },
    @{ id = "nestjs"; port = 3000 },
    @{ id = "zap"; port = 3000 },

    # --- Slower to compile/boot (Full-stack Rust / Java / PHP) ---
    @{ id = "springboot"; port = 8080 },
    @{ id = "loco"; port = 3000 },
    @{ id = "leptos"; port = 3000 }
)

Write-Host "==================================================" -ForegroundColor Green
Write-Host "Starting Black-Box HTTP Benchmark Suite (Bombardier v2.0.2)" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green

# Build the bombardier container first
Write-Host "Building bombardier load tester container..."
docker compose build bombardier

foreach ($fw in $frameworks) {
    $id = $fw.id
    $port = $fw.port

    Write-Host "`n--------------------------------------------------" -ForegroundColor Cyan
    Write-Host "Benchmarking framework: $id" -ForegroundColor Cyan
    Write-Host "--------------------------------------------------" -ForegroundColor Cyan

    # Build and start container
    Write-Host "Building service $id..."
    docker compose build $id
    
    Write-Host "Starting service $id..."
    docker compose up -d $id

    # Wait for the service to boot and bind
    Write-Host "Waiting 5 seconds for $id to initialize..."
    Start-Sleep -Seconds 5

    # Run plaintext benchmark
    Write-Host "Running Plaintext benchmark (http://$id`:$port/)..."
    docker compose run --rm bombardier -c 125 -d 10s "http://$id`:$port/" | Out-File -Encoding utf8 "results/$id`_plaintext.txt"

    # Run JSON benchmark
    Write-Host "Running JSON benchmark (http://$id`:$port/json)..."
    docker compose run --rm bombardier -c 125 -d 10s "http://$id`:$port/json" | Out-File -Encoding utf8 "results/$id`_json.txt"

    # Stop and clean up container to free resources
    Write-Host "Stopping and cleaning up service $id..."
    docker compose stop $id
    docker compose rm -f $id
}

Write-Host "`nAll benchmarks completed. Generating results..." -ForegroundColor Green
python parse_results.py results
