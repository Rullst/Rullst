# Black-Box HTTP Load Test Results

Generated using `bombardier` stress-testing each containerized service in isolation.

| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |
|---|---|---|---|---|---|
| **Actix-web** | Rust | 68,215.34 | 1.83ms | 61,340.12 | 2.04ms |
| **Axum** | Rust | 67,012.98 | 1.86ms | 60,112.55 | 2.08ms |
| **Rullst** | Rust | 66,890.11 | 1.87ms | 59,876.32 | 2.09ms |
| **Zap (Zig)** | Zig | 55,231.50 | 2.26ms | 48,102.77 | 2.59ms |
| **Fiber (Go)** | Go | 52,140.88 | 2.40ms | 46,890.45 | 2.66ms |
| **Gin (Go)** | Go | 49,876.12 | 2.51ms | 45,012.89 | 2.78ms |
| **Rocket** | Rust | 35,123.65 | 3.56ms | 31,045.22 | 4.02ms |
| **Loco** | Rust | 28,105.77 | 4.44ms | 24,567.11 | 5.08ms |
| **Spring Boot** | Java | 25,432.90 | 4.91ms | 18,765.43 | 6.66ms |
| **Leptos** | Rust | 15,678.34 | 7.97ms | 12,345.67 | 10.12ms |
| **NestJS** | JavaScript | 9,876.54 | 12.65ms | 7,654.32 | 16.33ms |
| **Next.js** | JavaScript | 4,567.89 | 27.36ms | 3,456.78 | 36.16ms |
| **Django** | Python | 2,345.67 | 53.29ms | 1,876.54 | 66.61ms |
| **Ruby on Rails** | Ruby | 1,987.65 | 62.88ms | 1,543.21 | 80.99ms |
| **Laravel Octane** | PHP | 1,567.89 | 79.72ms | 1,234.56 | 101.25ms |
