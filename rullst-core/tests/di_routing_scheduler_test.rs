// tests/di_routing_scheduler_test.rs — Comprehensive unit tests for DI, Routing & Scheduler.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::di::Container;
use rullst_core::scheduler::Scheduler;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
struct UserService {
    prefix: String,
}

impl UserService {
    fn format_name(&self, name: &str) -> String {
        format!("{}: {}", self.prefix, name)
    }
}

#[test]
fn test_di_container_registration_and_resolution() {
    let mut container = Container::new();

    // Unregistered resolution error
    let err = container.resolve::<UserService>();
    assert!(err.is_err());
    assert!(matches!(
        err.unwrap_err(),
        rullst_core::di::DiError::NotRegistered(_)
    ));

    // Register instance
    container.register(UserService {
        prefix: "User".to_string(),
    });

    let service = container.resolve::<UserService>().unwrap();
    assert_eq!(service.format_name("Alice"), "User: Alice");

    // Register Arc instance
    let mut container2 = Container::new();
    container2.register_arc(Arc::new(UserService {
        prefix: "Admin".to_string(),
    }));
    let service2 = container2.resolve::<UserService>().unwrap();
    assert_eq!(service2.format_name("Bob"), "Admin: Bob");
}

#[tokio::test]
async fn test_scheduler_task_registration_and_helpers() {
    let fired = Arc::new(AtomicBool::new(false));
    let fired_clone = fired.clone();

    let scheduler_res = Scheduler::new().task("* * * * *", move || {
        let f = fired_clone.clone();
        async move {
            f.store(true, Ordering::SeqCst);
        }
    });

    assert!(scheduler_res.is_ok());

    // Invalid cron should fail gracefully
    let invalid_res = Scheduler::new().task("invalid_cron_syntax", || async {});
    assert!(invalid_res.is_err());
}
