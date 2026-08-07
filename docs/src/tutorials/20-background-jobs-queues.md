# Tutorial 20: Background Jobs & Task Queues ⚙️

Dispatch async background workers for sending emails, generating PDFs, and processing heavy workloads with `rullst::queue`.

---

## 🛠️ Step 1: Scaffold a Background Worker

```bash
cargo rullst make:worker EmailWorker
```

In `src/workers/email_worker.rs`:

```rust
use async_trait::async_trait;
use rullst::queue::Job;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SendEmailJob {
    pub recipient: String,
    pub subject: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Sending email to: {}", self.recipient);
        // Execute SMTP logic
        Ok(())
    }
}
```

---

## 🚀 Step 2: Dispatch Jobs to Queue

```rust
use rullst::queue::Queue;

pub async fn trigger_welcome_email(email: String) -> Result<(), rullst_core::AppError> {
    Queue::dispatch(SendEmailJob {
        recipient: email,
        subject: "Welcome to Rullst!".to_string(),
    }).await?;
    
    Ok(())
}
```

---

## 💡 Key Takeaways
- Queue drivers support in-memory processing for local development and Redis for distributed production setups.
- Failed jobs automatically retry with exponential backoff.
- For a deep dive into Redis caching, RAM fallback, and single-node vs cloud deployment, see the [Redis Architecture Guide](../redis-guide.md).
