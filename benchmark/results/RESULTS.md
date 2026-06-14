# Black-Box HTTP Load Test Results

Generated using `bombardier` version 2.0.2 stress-testing each containerized service in isolation.

| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |
|---|---|---|---|---|---|
| **Rullst** | Rust | 13,063.31 | 9.71ms | 10,847.17 | 11.58ms |
| **Axum** | Rust | 27,418.26 | 4.55ms | 19,468.15 | 6.46ms |
| **Actix-web** | Rust | 25,250.49 | 5.00ms | 21,216.37 | 6.03ms |
| **Loco** | Rust | 90,338.83 | 1.38ms | 103,706.65 | 1.20ms |
| **Rocket** | Rust | 19,909.55 | 6.31ms | 19,073.33 | 6.55ms |
| **Leptos** | Rust | 43,919.60 | 2.86ms | 61,585.17 | 2.03ms |
| **Gin (Go)** | Go | 34,484.73 | 3.61ms | 35,856.79 | 3.48ms |
| **Fiber (Go)** | Go | 114,878.31 | 1.08ms | 110,825.31 | 1.12ms |
| **Django** | Python | 489.30 | 254.28ms | 692.12 | 179.23ms |
| **Laravel Octane** | PHP | 566.08 | 218.65ms | 536.45 | 233.78ms |
| **Next.js** | JavaScript | 82.00 | 1.39s | 190.91 | 799.50ms |
| **NestJS** | JavaScript | 1,084.24 | 116.34ms | 1,565.11 | 82.38ms |
| **Zap (Zig)** | Zig | 29,933.82 | 4.17ms | 25,616.38 | 4.87ms |
| **Spring Boot** | Java | 7,483.75 | 16.74ms | 3,480.63 | 37.72ms |
| **Ruby on Rails** | Ruby | 5,084.95 | 34.94ms | 62.88 | 2.00s |
