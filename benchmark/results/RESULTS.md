# Black-Box HTTP Load Test Results

Generated using `bombardier` version 2.0.2 stress-testing each containerized service in isolation.

| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |
|---|---|---|---|---|---|
| **Rullst** | Rust | 9,039.96 | 16.86ms | 12,098.06 | 10.56ms |
| **Axum** | Rust | 25,035.23 | 4.97ms | 32,889.89 | 4.54ms |
| **Actix-web** | Rust | 47,975.01 | 2.15ms | 44,916.85 | 2.78ms |
| **Loco** | Rust | 68.18 | 1.81s | 63.54 | 1.99s |
| **Rocket** | Rust | 16,382.96 | 7.68ms | 11,801.87 | 10.63ms |
| **Leptos** | Rust | 62.04 | 1.99s | 49.85 | 2.00s |
| **Gin (Go)** | Go | 21,452.38 | 5.87ms | 23,126.24 | 5.44ms |
| **Fiber (Go)** | Go | 11,928.16 | 10.66ms | 15,689.65 | 7.99ms |
| **Django** | Python | 489.30 | 254.28ms | 692.12 | 179.23ms |
| **Laravel Octane** | PHP | 62.85 | 2.00s | 62.07 | 1.99s |
| **Next.js** | JavaScript | 82.00 | 1.39s | 190.91 | 799.50ms |
| **NestJS** | JavaScript | 1,084.24 | 116.34ms | 1,565.11 | 82.38ms |
| **Zap (Zig)** | Zig | 29,933.82 | 4.17ms | 25,616.38 | 4.87ms |
| **Spring Boot** | Java | 7,483.75 | 16.74ms | 3,480.63 | 37.72ms |
