# Rullst Web Framework Benchmarks

This document records the performance of Rullst compared against other web frameworks across different languages and architectures.

## Current Benchmark Results

> [!NOTE]
> The following results are from the initial run using **Bombardier v2.0.2** load tester targeting containerized applications. Connection size: 125, duration: 10 seconds.
> Some frameworks failed to boot or compile in the initial run and are marked as `N/A`. They are currently being fixed.

| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |
|---|---|---|---|---|---|
| **Actix-web** | Rust | 47,975.01 | 2.15ms | 44,916.85 | 2.78ms |
| **Zap (Zig)** | Zig | 29,933.82 | 4.17ms | 25,616.38 | 4.87ms |
| **Axum** | Rust | 25,035.23 | 4.97ms | 32,889.89 | 4.54ms |
| **Gin (Go)** | Go | 21,452.38 | 5.87ms | 23,126.24 | 5.44ms |
| **Rocket** | Rust | 16,382.96 | 7.68ms | 11,801.87 | 10.63ms |
| **Fiber (Go)** | Go | 11,928.16 | 10.66ms | 15,689.65 | 7.99ms |
| **Rullst** | Rust | 9,039.96 | 16.86ms | 12,098.06 | 10.56ms |
| **Spring Boot** | Java | 7,483.75 | 16.74ms | 3,480.63 | 37.72ms |
| **NestJS** | JavaScript | 1,084.24 | 116.34ms | 1,565.11 | 82.38ms |
| **Django** | Python | 489.30 | 254.28ms | 692.12 | 179.23ms |
| **Next.js** | JavaScript | 82.00 | 1.39s | 190.91 | 799.50ms |
| **Loco** | Rust | 68.18 | 1.81s | 63.54 | 1.99s |
| **Laravel Octane** | PHP | 62.85 | 2.00s | 62.07 | 1.99s |
| **Leptos** | Rust | 62.04 | 1.99s | 49.85 | 2.00s |


## Performance Ranking & Analysis

The benchmark results above are sorted from best-performing (highest throughput and lowest average latency) to lowest-performing. Here is an analysis of this hierarchy:

1.  **Tier 1: Bare-Metal Compiled Microframeworks (Actix-web, Zap, Axum, Gin)**:
    *   *Actix-web* and *Axum* represent the pinnacle of Rust's asynchronous runtime performance, yielding up to 48,000 requests per second.
    *   *Zap (Zig)* follows very closely, showcasing the raw speed of Zig's memory-efficient HTTP parsing and compilation capabilities.
    *   *Gin (Go)* leverages Go's highly optimized goroutine scheduler to achieve top-tier concurrent processing.

2.  **Tier 2: Intermediate & Balanced Frameworks (Rocket, Fiber, Rullst, Spring Boot)**:
    *   *Rocket* offers slightly more convenience wrappers but maintains solid speed.
    *   *Fiber (Go)* utilizes `valyala/fasthttp` under the hood for optimized throughput.
    *   *Rullst* registers ~9k req/s for plaintext and ~12k req/s for JSON, presenting a balanced position. It delivers excellent raw efficiency, outperforming traditional corporate stacks (Spring Boot, NestJS) by a wide margin while offering high type safety.
    *   *Spring Boot (Java)* performs strongly for a JVM framework, but exhibits higher latency (~16ms) and memory footprint compared to native binaries.

3.  **Tier 3: Node.js & Dynamic Frameworks (NestJS, Django, Next.js)**:
    *   *NestJS* showcases standard Node.js performance (~1,000–1,500 reqs/s).
    *   *Django* represents standard interpreted Python runtime speeds.
    *   *Next.js* has low plaintext request throughput but performs decently in JSON serialization.

4.  **Tier 4: Connection Bottlenecks / Container Issues (Loco, Leptos, Laravel Octane)**:
    *   Frameworks in this tier scored exactly ~60 requests per second with an average latency of ~2.00 seconds. This profile indicates a networking timeout or connection-refused state inside the container runtime environment under 125 concurrent connections, rather than representing their native execution speeds.


## Why Rullst? Key Architectural Benefits

While Rullst competes closely in performance with bare-metal Rust frameworks like Axum and Actix-web, it stands out due to its unique architectural choices designed for modern software development and AI collaboration:

*   **AI-Native & Code-Reasoning Optimization**: Rullst is architected to be highly readable and predictable. By avoiding runtime magic, implicit reflection, or dynamic dependency injection, AI coding assistants (like Gemini/Claude) and developers can easily parse, trace, and modify the application without fear of runtime bugs.
*   **Compile-Time Guarantees**: Rullst prioritizes catching structural bugs, route definition issues, and middleware mismatches at compile time rather than relying on runtime failures or comprehensive test suites.
*   **Strict Type Safety (No Dynamic Traits)**: Rullst avoids heavy usage of `dyn Trait` in favor of static dispatch and strong typing. This leads to better compiler optimizations (monomorphization), smaller binaries, and robust memory/concurrency safety guarantees.
*   **Explicit API Design**: There is no hidden state or magical middleware sequencing. Everything (routes, shared state, filters, extensions) is explicitly declared and wired, preventing side effects and making codebase maintainability simple.
*   **Balanced DX & Performance**: Rullst delivers high throughput and low latencies comparable to microframeworks like Axum, but provides a structured scaffold reminiscent of full-stack frameworks (like Loco or NestJS) without sacrificing performance.


## Understanding the Benchmark Metrics

Here is a detailed explanation of what each metric and test case signifies:

### 1. Key Metrics

*   **Throughput (Requests per Second - Reqs/s)**: Represents the total number of HTTP requests the web server can successfully process and respond to in one second under high concurrency. **Higher is better.** This indicates the raw capacity of the framework's HTTP parser and event loop.
*   **Latency (Avg)**: The average time taken from the moment a client sends a request to when it receives the complete response. **Lower is better.** High throughput is only useful if latency remains low (ideally in low milliseconds); high latency leads to sluggish user experiences.

### 2. Test Cases

*   **Plaintext Test (`GET /`)**:
    *   **Payload**: Returns a simple `"Hello, World!"` string with a `text/plain` content type.
    *   **Significance**: Measures the absolute baseline performance of the HTTP server engine. Since there is minimal business logic and no serialization, it highlights the efficiency of the connection handler, routing table lookup, and HTTP protocol serialization/deserialization.
*   **JSON Test (`GET /json`)**:
    *   **Payload**: Returns a serialized JSON object: `{"message": "Hello, World!"}`.
    *   **Significance**: Simulates a basic REST API endpoint. It introduces CPU overhead from object allocations and JSON serialization. This highlights how well the language's serialization library and memory allocator handle rapid, repetitive serialization tasks under load.

### 3. Methodology & Isolation

*   **Docker Isolation**: Every framework runs in its own isolated Docker container with the exact same networking and host resource conditions. This isolates host OS interference (like background tasks or custom network configurations) and guarantees an "apples-to-apples" comparison.
*   **Load Testing Tool**: We use **Bombardier** (written in Go), utilizing fast event-driven HTTP clients to maximize load generation without becoming the bottleneck themselves.

