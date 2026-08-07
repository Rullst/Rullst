# ⚡ Redis Architecture, In-Memory Caching & Distributed Queues

Rullst provides first-class support for **Redis** as a zero-cost, high-performance in-memory caching layer and distributed task queue driver.

This guide explains how Redis integrates with Rullst, how the **Dual-Mode (RAM Fallback)** engine operates, when to enable Redis in your CLI projects, and best practices for cost-effective deployment with primary databases like **Turso (libSQL)**, **SQLite**, and **PostgreSQL**.

---

## 🎯 What is Redis in Rullst?

In the Rullst ecosystem, Redis acts as a **database turbocharger** and **distributed job coordinator**:

1. **In-Memory Query & Response Caching**: Caches expensive database queries, API responses, and session payloads in RAM for sub-millisecond retrievals.
2. **Distributed Job Queues**: Coordinates background workers (`rullst::queue`) across multiple server nodes so jobs (such as email sending, PDF generation, and webhooks) are executed reliably without duplication.
3. **Pub/Sub & Real-Time Sync**: Facilitates real-time WebSocket state synchronization across horizontally scaled application instances.

---

## 🔄 Dual-Mode Architecture & Graceful Fallback

Rullst is designed with a **Zero-Downtime Graceful Fallback** model. You can safely enable the `redis` feature in your project without needing a Redis server running locally during development!

```
                    ┌─────────────────────────────────────────┐
                    │            Rullst Application           │
                    └────────────────────┬────────────────────┘
                                         │
                   Is REDIS_URL configured & reachable?
                                  ┌──────┴──────┐
                             YES  │             │  NO / OFFLINE
                                  ▼             ▼
                     ┌──────────────────┐  ┌──────────────────────┐
                     │   Redis Driver   │  │ Local RAM Fallback   │
                     │  (Cache/Queues)  │  │ (DashMap + Tokio MPMC)│
                     └──────────────────┘  └──────────────────────┘
```

* **Offline / Local Development Mode**: If no `REDIS_URL` environment variable is defined or the Redis server is unreachable, Rullst automatically falls back to its **built-in in-memory engine** using high-concurrency `DashMap` for caching and Tokio channels for task queues. Your application runs seamlessly in local development with **zero external dependencies**.
* **Production Distributed Mode**: Simply provide `REDIS_URL=redis://127.0.0.1:6379` (or a managed cloud URL) in your `.env` file. Rullst instantly switches to full Redis distributed caching and shared task queues without requiring code changes.

---

## 🛠️ Enabling Redis via CLI Wizard

When creating a new application with `cargo rullst new`, you will be prompted:

```text
✔ 🚀 Enable Redis? (Ultra-fast in-memory cache & distributed jobs; auto-falls back to RAM if offline) · yes
```

### Should I select "YES" even if I am not using Redis right away?
**Yes, recommended!** Selecting "YES" includes the optional `redis` feature in your `Cargo.toml`. Because Rullst gracefully falls back to local RAM when offline, your application remains lightweight and fast locally. When your project scales in production, you can attach a Redis instance without modifying your application codebase.

---

## 🤝 Turso / SQLite + Redis: The Perfect Pair

Combining an Edge or relational database with Redis creates an optimal full-stack architecture:

| Component | Role | Resource Characteristic |
| :--- | :--- | :--- |
| **Turso (libSQL) / SQLite / Postgres** | Primary Relational Storage | Persistent records (Users, Orders, Transactions) |
| **Redis / In-Memory RAM Layer** | Accelerator & Queue Broker | Sub-millisecond transient cache & async jobs |

### Why Redis + Turso?
- **Offload Heavy Read Operations**: Frequent queries (e.g. user profiles, pricing plans, catalog listings) are served directly from Redis RAM in < 1ms, reducing network roundtrips to Turso.
- **Asynchronous Task Execution**: Offload heavy mutations (e.g. sending Stripe receipts or processing AI embeddings) to background workers using Redis queues.

---

## 💰 Cost-Effective Deployment Strategies

### Strategy A: Single VPS / Single Container (Startups & MVPs)
Thanks to Rust's tiny memory footprint, you can deploy your entire stack on a single **$4 - $6/month VPS** (e.g. Hetzner, DigitalOcean, Linode) using Docker Compose:

- **Rullst Rust App**: ~15 - 30 MB RAM
- **Redis Container**: ~15 - 25 MB RAM
- **Postgres / SQLite**: ~50 - 100 MB RAM
- **Total Footprint**: **< 200 MB RAM total!**

### Strategy B: Managed Cloud Redis (High Availability & Scale)
When your traffic expands to hundreds of thousands of active users across multiple load-balanced application servers, attach a managed Redis service:
- **Serverless Redis**: Upstash Redis (`REDIS_URL=rediss://default:token@...upstash.io:6379`)
- **Cloud Providers**: AWS ElastiCache, GCP MemoryStore, or Redis Enterprise.

---

## 💻 Code Usage Examples

### 1. In-Memory Caching with `rullst::cache`

```rust
use rullst::cache::Cache;

pub async fn get_featured_products() -> Result<Vec<Product>, AppError> {
    // Tries to get from Redis / RAM cache first; if missing, fetches from DB and caches for 10 minutes.
    let products = Cache::remember("featured_products", 600, || async {
        Product::query().where_eq("is_featured", true).get().await
    }).await?;

    Ok(products)
}
```

### 2. Dispatching Background Jobs with `rullst::queue`

```rust
use rullst::queue::Queue;
use crate::workers::SendWebhookJob;

pub async fn trigger_webhook(user_id: i64) -> Result<(), AppError> {
    Queue::dispatch(SendWebhookJob { user_id }).await?;
    Ok(())
}
```
