use rullst::queue::{Queue, Worker};
use serde_json::json;

#[tokio::test]
async fn test_queue_dispatch_and_worker() {
    let queue = Queue::sqlite("sqlite::memory:")
        .await
        .expect("Failed to create queue");

    // Dispatch a job
    queue
        .dispatch("test_job", json!({"hello": "world"}))
        .await
        .expect("Failed to dispatch");

    // Initialize worker
    let mut worker = Worker::new(&queue);

    // Register handler
    worker.register("test_job", |payload| async move {
        assert_eq!(payload["hello"], "world");
        Ok(())
    });
}
