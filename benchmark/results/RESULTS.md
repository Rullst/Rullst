# Black-Box HTTP Load Test Results

Generated using `bombardier` version 2.0.2 stress-testing each containerized service in isolation.

| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |
|---|---|---|---|---|---|
| **Rullst** | Rust | 38,426.60 | 3.30ms | 74,393.31 | 1.68ms |
| **Axum** | Rust | 112,099.39 | 1.11ms | 112,075.84 | 1.11ms |
| **Actix-web** | Rust | 119,852.35 | 1.04ms | 112,556.60 | 1.11ms |
| **Loco** | Rust | 90,338.83 | 1.38ms | 103,706.65 | 1.20ms |
| **Rocket** | Rust | 107,913.46 | 1.15ms | 107,814.24 | 1.16ms |
| **Leptos** | Rust | 43,919.60 | 2.86ms | 61,585.17 | 2.03ms |
| **Gin (Go)** | Go | 34,484.73 | 3.61ms | 35,856.79 | 3.48ms |
| **Fiber (Go)** | Go | 114,878.31 | 1.08ms | 110,825.31 | 1.12ms |
| **Django** | Python | 489.30 | 254.28ms | 692.12 | 179.23ms |
| **Laravel Octane** | PHP | 62.85 | 2.00s | 62.07 | 1.99s |
| **Next.js** | JavaScript | 82.00 | 1.39s | 190.91 | 799.50ms |
| **NestJS** | JavaScript | 1,084.24 | 116.34ms | 1,565.11 | 82.38ms |
| **Zap (Zig)** | Zig | 29,933.82 | 4.17ms | 25,616.38 | 4.87ms |
| **Spring Boot** | Java | 7,483.75 | 16.74ms | 3,480.63 | 37.72ms |
