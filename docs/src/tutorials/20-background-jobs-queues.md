# Tutorial 20: Background Jobs & Task Queues ⚙️

Rullst Core provides named JSON jobs, SQLite and Redis storage drivers, and a
bounded-concurrency worker. Job handlers are closures registered against a
stable name; there is no `Job` trait or process-global `Queue::dispatch` API.

---

## Step 1: Scaffold a handler module

```bash
cargo rullst make:worker EmailWorker
```

This creates `src/workers/email_worker.rs`, registers it in
`src/workers/mod.rs`, and provides `start_workers`. Replace the generated log-only
body with the application operation. Avoid logging the complete job payload,
because it can contain personal or secret data.

---

## Step 2: Create a queue and keep its worker handle alive

```rust,no_run
use rullst::queue::{Queue, QueueError, Worker, WorkerHandle};
use serde_json::json;

fn start_email_worker(queue: &Queue) -> Result<WorkerHandle, QueueError> {
    let mut worker = Worker::new(queue)
        .max_concurrency(8)
        .poll_interval(250);
    worker.register("email", |payload| async move {
        let recipient = payload
            .get("recipient")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "missing recipient")
            })?;

        // Call an idempotent mail service with a stable delivery key here.
        let _ = recipient;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });
    worker.run()
}

async fn configure() -> Result<(Queue, WorkerHandle), QueueError> {
    let queue = Queue::sqlite("sqlite://queue.db?mode=rwc").await?;
    let handle = start_email_worker(&queue)?;
    queue
        .dispatch(
            "email",
            json!({"recipient": "learner@example.test", "delivery_key": "welcome:42"}),
        )
        .await?;
    Ok((queue, handle))
}
```

Dropping `WorkerHandle` stops processing. On graceful shutdown, call
`handle.shutdown().await` and inspect its typed error.

Use `Queue::redis(redis_url)` with the `queue-redis` feature when independent
processes must share work. SQLite is durable local state and supports atomic
claims, but it is not a distributed queue.

---

## Failure, retry, and delivery semantics

- A handler error, panic, or timeout is recorded as failed. Core does not
  automatically retry it with exponential backoff.
- `retry_failed_job(job_id)` explicitly returns a failed job to pending state.
- SQLite and Redis recover stale processing leases and support scheduled jobs,
  but execution time is the first worker poll after the due timestamp.
- Queue processing is at least once around crashes and external side effects.
  Give provider operations stable idempotency keys and reconcile ambiguous
  outcomes.
- Successful payloads are deleted by default. SQLite completed history is an
  explicit bounded opt-in with its own privacy/access-control obligations.

See the [Redis Architecture Guide](../redis-guide.md) for deployment boundaries.
