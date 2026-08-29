// src/queue/tests.rs — Comprehensive unit and integration test suite for Queue drivers and Workers.

#[cfg(test)]
mod formatting_tests {
    use super::super::*;
    #[test]
    fn test_queue_error_display() {
        assert_eq!(
            QueueError::Driver("db error".into()).to_string(),
            "Queue driver error: db error"
        );
        assert_eq!(
            QueueError::Serialization("bad json".into()).to_string(),
            "Queue serialization error: bad json"
        );
        assert_eq!(
            QueueError::HandlerNotFound("job_a".into()).to_string(),
            "No handler registered for job: job_a"
        );
        assert_eq!(
            QueueError::JobFailed("crash".into()).to_string(),
            "Job execution failed: crash"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(not(miri))]
pub mod driver_tests {
    use super::super::*;
    use async_trait::async_trait;
    use std::time::Duration;

    #[tokio::test]
    async fn test_sqlite_queue_push_pop() {
        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
        let job_id = queue
            .dispatch("test_job", serde_json::json!({"key": "value"}))
            .await
            .unwrap();
        assert!(!job_id.is_empty());

        let count = queue.pending_count().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_sqlite_queue_pop_returns_correct_job() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver
            .push("job-1", "send_email", r#"{"to":"a@b.com"}"#)
            .await
            .unwrap();
        driver
            .push("job-2", "process_image", r#"{"path":"/img.png"}"#)
            .await
            .unwrap();

        let job = driver.pop().await.unwrap().unwrap();
        assert_eq!(job.id, "job-1");
        assert_eq!(job.name, "send_email");
        assert_eq!(job.payload["to"], "a@b.com");

        driver.mark_complete("job-1").await.unwrap();

        let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM rullst_jobs WHERE id = 'job-1'")
            .fetch_optional(&driver.pool)
            .await
            .unwrap();
        assert!(row.is_none());

        let job2 = driver.pop().await.unwrap().unwrap();
        assert_eq!(job2.id, "job-2");
        assert_eq!(job2.name, "process_image");
    }

    #[tokio::test]
    async fn test_sqlite_driver_get_pool() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let pool = driver.get_pool();

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs")
            .fetch_one(pool)
            .await
            .unwrap();

        assert_eq!(row.0, 0);

        driver.push("test-job", "test_handler", "{}").await.unwrap();
        let row_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs")
            .fetch_one(pool)
            .await
            .unwrap();

        assert_eq!(row_after.0, 1);
    }

    #[tokio::test]
    async fn test_sqlite_queue_mark_failed() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver.push("fail-job", "bad_job", r#"{}"#).await.unwrap();

        let job = driver.pop().await.unwrap().unwrap();
        driver
            .mark_failed(&job.id, "Something went wrong")
            .await
            .unwrap();

        let count = driver.pending_count().await.unwrap();
        assert_eq!(count, 0);

        let status_row: (String, String) =
            sqlx::query_as("SELECT status, error FROM rullst_jobs WHERE id = ?")
                .bind(&job.id)
                .fetch_one(&driver.pool)
                .await
                .unwrap();
        assert_eq!(status_row.0, "failed");
        assert_eq!(status_row.1, "Something went wrong");
    }

    #[tokio::test]
    async fn test_sqlite_queue_empty_pop() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let result = driver.pop().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn invalid_json_is_an_error_and_the_job_is_failed() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver.push("invalid", "test", "{not-json").await.unwrap();

        let result = driver.pop().await;
        assert!(matches!(result, Err(QueueError::Serialization(_))));
        let (status, error): (String, String) =
            sqlx::query_as("SELECT status, error FROM rullst_jobs WHERE id = 'invalid'")
                .fetch_one(&driver.pool)
                .await
                .unwrap();
        assert_eq!(status, "failed");
        assert!(error.contains("invalid JSON payload"));
    }

    #[tokio::test]
    async fn stale_processing_jobs_are_recovered() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver.push("stale", "test", "{}").await.unwrap();
        let _ = driver.pop().await.unwrap().unwrap();
        sqlx::query("UPDATE rullst_jobs SET updated_at = '2000-01-01 00:00:00' WHERE id = 'stale'")
            .execute(&driver.pool)
            .await
            .unwrap();

        assert_eq!(
            driver
                .recover_stalled(Duration::from_secs(1))
                .await
                .unwrap(),
            1
        );
        assert_eq!(driver.pending_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn missing_state_transition_is_not_reported_as_success() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        assert!(matches!(
            driver.mark_complete("missing").await,
            Err(QueueError::StateTransition {
                operation: "mark_complete",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_sqlite_queue_trait_methods() {
        let driver_impl = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let driver: Box<dyn QueueDriver> = Box::new(driver_impl);

        driver.push("job-1", "test", "{}").await.unwrap();

        let job = driver.pop().await.unwrap().unwrap();
        driver.mark_failed(&job.id, "error").await.unwrap();

        assert_eq!(driver.pending_count().await.unwrap(), 0);

        driver.retry_failed_job(&job.id).await.unwrap();
        assert_eq!(driver.pending_count().await.unwrap(), 1);

        let job2 = driver.pop().await.unwrap().unwrap();
        driver.mark_failed(&job2.id, "error2").await.unwrap();

        driver.purge_failed_jobs().await.unwrap();
        let jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 0);
    }

    #[test]
    #[cfg(feature = "queue-redis")]
    fn test_queue_redis_creation() {
        let valid_result = Queue::redis("redis://127.0.0.1:6379");
        assert!(
            valid_result.is_ok(),
            "Failed to create Redis queue with valid URL"
        );

        let invalid_result = Queue::redis("invalid_url");
        assert!(
            invalid_result.is_err(),
            "Expected error for invalid Redis URL"
        );

        match invalid_result {
            Err(QueueError::Driver(msg)) => {
                assert!(
                    msg.contains("Failed to connect to Redis"),
                    "Unexpected error message: {}",
                    msg
                );
                assert!(
                    msg.contains("Redis URL did not parse"),
                    "Unexpected error message details: {}",
                    msg
                );
            }
            _ => panic!("Expected QueueError::Driver, got something else"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_happy_path() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        let jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 0);

        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES (?, ?, ?, ?)")
            .bind("job-1")
            .bind("test_job_1")
            .bind(r#"{"test": 1}"#)
            .bind("2020-01-01 10:00:00")
            .execute(&driver.pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES (?, ?, ?, ?)")
            .bind("job-2")
            .bind("test_job_2")
            .bind(r#"{"test": 2}"#)
            .bind("2020-01-01 11:00:00")
            .execute(&driver.pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES (?, ?, ?, ?)")
            .bind("job-3")
            .bind("test_job_3")
            .bind(r#"{"test": 3}"#)
            .bind("2020-01-01 12:00:00")
            .execute(&driver.pool)
            .await
            .unwrap();

        let jobs = driver.list_all_jobs(2).await.unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, "job-3");
        assert_eq!(jobs[1].id, "job-2");

        let all_jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(all_jobs.len(), 3);
        assert_eq!(all_jobs[0].id, "job-3");
        assert_eq!(all_jobs[1].id, "job-2");
        assert_eq!(all_jobs[2].id, "job-1");
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_error() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        sqlx::query("DROP TABLE rullst_jobs")
            .execute(&driver.pool)
            .await
            .unwrap();

        let result = driver.list_all_jobs(10).await;
        assert!(result.is_err());
        match result {
            Err(QueueError::Driver(msg)) => {
                assert!(msg.contains("no such table"));
            }
            _ => panic!("Expected Driver error"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_wrapper() {
        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();

        queue
            .dispatch("job1", serde_json::json!({"data": 1}))
            .await
            .unwrap();
        queue
            .dispatch("job2", serde_json::json!({"data": 2}))
            .await
            .unwrap();

        let jobs = queue.list_all_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 2);

        assert!(jobs.iter().any(|j| j.name == "job1"));
        assert!(jobs.iter().any(|j| j.name == "job2"));
    }

    #[tokio::test]
    async fn test_queue_list_all_jobs_error() {
        struct ErrorMockDriver;

        #[async_trait]
        impl QueueDriver for ErrorMockDriver {
            async fn push(
                &self,
                _id: &str,
                _job_name: &str,
                _payload: &str,
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn pending_count(&self) -> Result<u64, QueueError> {
                Ok(0)
            }
            async fn list_all_jobs(&self, _limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
                Err(QueueError::Driver("simulated db error".into()))
            }
        }

        let queue = Queue::custom(Box::new(ErrorMockDriver));
        let result = queue.list_all_jobs(10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueueError::Driver(msg) => assert_eq!(msg, "simulated db error"),
            _ => panic!("Expected QueueError::Driver"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_purge_failed_jobs() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        driver
            .push("job-to-fail", "fail_job", r#"{}"#)
            .await
            .unwrap();
        driver
            .push("job-pending", "pending_job", r#"{}"#)
            .await
            .unwrap();

        let job = driver.pop().await.unwrap().unwrap();
        assert_eq!(job.id, "job-to-fail");
        driver.mark_failed(&job.id, "Error").await.unwrap();

        driver.purge_failed_jobs().await.unwrap();

        let pending = driver.pending_count().await.unwrap();
        assert_eq!(pending, 1);

        let failed_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs WHERE status = 'failed'")
                .fetch_one(&driver.pool)
                .await
                .unwrap();
        assert_eq!(failed_count.0, 0);

        let job2 = driver.pop().await.unwrap().unwrap();
        assert_eq!(job2.id, "job-pending");

        let _retry_result = driver.retry_failed_job("job-to-fail").await;
    }

    #[tokio::test]
    async fn test_sqlite_queue_retry_failed_job() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver
            .push("retry-job", "retry_job", r#"{}"#)
            .await
            .unwrap();

        let job = driver.pop().await.unwrap().unwrap();
        driver.mark_failed(&job.id, "Err").await.unwrap();

        let failed_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs WHERE status = 'failed'")
                .fetch_one(&driver.pool)
                .await
                .unwrap();
        assert_eq!(failed_count.0, 1);

        driver.retry_failed_job(&job.id).await.unwrap();

        let pending_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs WHERE status = 'pending'")
                .fetch_one(&driver.pool)
                .await
                .unwrap();
        assert_eq!(pending_count.0, 1);
    }

    #[tokio::test]
    async fn test_sqlite_queue_purge_error() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver.pool.close().await;

        let result = driver.purge_failed_jobs().await;
        assert!(result.is_err());
        if let Err(QueueError::Driver(msg)) = result {
            assert!(
                msg.contains("PoolClosed") || msg.contains("closed"),
                "Unexpected error message: {}",
                msg
            );
        } else {
            panic!("Expected QueueError::Driver, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_driver() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        let jobs = driver.list_all_jobs(10).await.unwrap();
        assert!(jobs.is_empty());

        driver
            .push("job-1", "job_type_a", r#"{"data": 1}"#)
            .await
            .unwrap();
        driver
            .push("job-2", "job_type_b", r#"{"data": 2}"#)
            .await
            .unwrap();
        driver
            .push("job-3", "job_type_c", r#"{"data": 3}"#)
            .await
            .unwrap();

        let all_jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(all_jobs.len(), 3);

        assert!(all_jobs.iter().any(|j| j.id == "job-1"));
        assert!(all_jobs.iter().any(|j| j.id == "job-2"));
        assert!(all_jobs.iter().any(|j| j.id == "job-3"));

        let limited_jobs = driver.list_all_jobs(2).await.unwrap();
        assert_eq!(limited_jobs.len(), 2);

        driver.pool.close().await;

        let result = driver.list_all_jobs(10).await;
        assert!(result.is_err());
        match result {
            Err(QueueError::Driver(msg)) => {
                assert!(
                    msg.contains("pool timed out") || msg.contains("closed"),
                    "Unexpected error message: {}",
                    msg
                );
            }
            _ => panic!("Expected QueueError::Driver"),
        }
    }

    #[tokio::test]
    async fn test_custom_queue_driver() {
        let push_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        struct ArcMockDriver {
            push_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }

        #[async_trait]
        impl QueueDriver for ArcMockDriver {
            async fn push(
                &self,
                _id: &str,
                _job_name: &str,
                _payload: &str,
            ) -> Result<(), QueueError> {
                self.push_called
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
            async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn pending_count(&self) -> Result<u64, QueueError> {
                Ok(0)
            }
        }

        let driver = Box::new(ArcMockDriver {
            push_called: push_called.clone(),
        });

        let queue = Queue::custom(driver);
        let _id = queue
            .dispatch("test_custom_job", serde_json::json!({}))
            .await
            .unwrap();

        assert!(push_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    pub struct MockPendingCountDriver {
        pub should_fail: bool,
    }

    #[async_trait]
    impl QueueDriver for MockPendingCountDriver {
        async fn push(&self, _id: &str, _job_name: &str, _payload: &str) -> Result<(), QueueError> {
            Ok(())
        }
        async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
            Ok(None)
        }
        async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
            Ok(())
        }
        async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
            Ok(())
        }
        async fn pending_count(&self) -> Result<u64, QueueError> {
            if self.should_fail {
                Err(QueueError::Driver("mock failure".to_string()))
            } else {
                Ok(42)
            }
        }
    }

    #[tokio::test]
    async fn test_queue_pending_count_ok() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let count = queue.pending_count().await.unwrap();
        assert_eq!(count, 42);
    }

    #[tokio::test]
    async fn test_queue_pending_count_err() {
        let driver = Box::new(MockPendingCountDriver { should_fail: true });
        let queue = Queue::custom(driver);
        let err = queue.pending_count().await.unwrap_err();
        match err {
            QueueError::Driver(msg) => assert_eq!(msg, "mock failure"),
            _ => panic!("Expected QueueError::Driver"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(not(miri))]
mod tests_additional {
    use super::super::*;
    use super::driver_tests::MockPendingCountDriver;

    #[tokio::test]
    async fn test_queue_retry_failed_job() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.retry_failed_job("1").await;
        assert!(matches!(res, Err(QueueError::Unsupported(_))));
    }

    #[tokio::test]
    async fn test_queue_list_all_jobs() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.list_all_jobs(10).await;
        assert!(matches!(res, Err(QueueError::Unsupported(_))));
    }

    #[tokio::test]
    async fn test_queue_dispatch() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.dispatch("job", serde_json::json!({})).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn unsupported_driver_does_not_report_a_fake_failed_job_purge() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.purge_failed_jobs().await;
        assert!(matches!(res, Err(QueueError::Unsupported(_))));
    }

    #[tokio::test]
    async fn test_queue_pending_count() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.pending_count().await;
        assert_eq!(res.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_sqlite_driver_purge_failed_jobs() {
        let driver = crate::queue::SqliteDriver::new("sqlite::memory:")
            .await
            .unwrap();
        let pool = driver.get_pool();
        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, status) VALUES ('1', 'test', '{}', 'completed')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, status) VALUES ('2', 'test', '{}', 'failed')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, status) VALUES ('3', 'test', '{}', 'pending')")
            .execute(pool).await.unwrap();

        driver.purge_failed_jobs().await.unwrap();

        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rullst_jobs WHERE status = 'failed'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(remaining, 0);
    }

    #[tokio::test]
    async fn test_sqlite_driver_retry_failed_job() {
        let driver = crate::queue::SqliteDriver::new("sqlite::memory:")
            .await
            .unwrap();
        let pool = driver.get_pool();
        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, status, attempts, error) VALUES ('1', 'test', '{}', 'failed', 3, 'err')")
            .execute(pool).await.unwrap();

        driver.retry_failed_job("1").await.unwrap();

        let (status, attempts, error): (String, i32, Option<String>) =
            sqlx::query_as("SELECT status, attempts, error FROM rullst_jobs WHERE id = '1'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(status, "pending");
        assert_eq!(attempts, 0);
        assert!(error.is_none());
    }
}
