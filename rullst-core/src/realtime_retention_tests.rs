//! Retention and race tests for the process-local realtime registries.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::security::TenantMembership;

#[test]
fn publishing_to_a_channel_without_subscribers_retains_nothing() {
    let manager = BroadcastManager::new();

    let result = manager.publish("user/offline", "notification", "{}");

    assert_eq!(
        result,
        Err(RealtimeError::BroadcastError("channel closed".to_string()))
    );
    assert_eq!(manager.channels.len(), 0);
}

#[test]
fn a_channel_is_released_after_its_last_subscriber_leaves() {
    let manager = BroadcastManager::new();
    let first = manager.get_or_create("course/1").subscribe();
    let second = manager.get_or_create("course/1").subscribe();
    assert_eq!(manager.publish("course/1", "tick", "{}"), Ok(2));

    drop(first);
    assert_eq!(manager.publish("course/1", "tick", "{}"), Ok(1));
    assert_eq!(manager.channels.len(), 1);

    drop(second);
    assert!(manager.publish("course/1", "tick", "{}").is_err());
    assert_eq!(manager.channels.len(), 0);
}

#[test]
fn abandoned_channels_do_not_accumulate() {
    let manager = BroadcastManager::new();
    let kept = manager.get_or_create("kept").subscribe();

    for index in 0..10_000 {
        drop(manager.get_or_create(&format!("room/{index}")).subscribe());
    }

    assert!(
        manager.channels.len() <= 64,
        "{} channels retained",
        manager.channels.len()
    );
    assert_eq!(manager.publish("kept", "tick", "{}"), Ok(1));
    drop(kept);
}

#[test]
fn tenant_notifications_to_offline_users_retain_nothing() {
    let membership = TenantMembership::try_new(["school-alpha"]).expect("membership");
    let context = membership.select("school-alpha").expect("context");
    let manager = Arc::new(BroadcastManager::new());
    let realtime = TenantRealtime::from_context(Arc::clone(&manager), &context);

    for user in 0..1_000 {
        assert!(
            realtime
                .publish(&format!("user/{user}"), "notification", "{}")
                .is_err()
        );
    }

    assert_eq!(manager.channels.len(), 0);
}

#[test]
fn an_empty_presence_room_is_removed() {
    let tracker = PresenceTracker::new();
    tracker.user_joined("course/1", "learner-1");
    tracker.user_joined("course/1", "learner-2");

    tracker.user_left("course/1", "learner-1");
    assert_eq!(tracker.count_online("course/1"), 1);
    assert_eq!(tracker.online_users.len(), 1);

    tracker.user_left("course/1", "learner-2");
    tracker.user_left("never-joined", "learner-3");
    assert_eq!(tracker.count_online("course/1"), 0);
    assert_eq!(tracker.online_users.len(), 0);

    tracker.user_joined("course/1", "learner-1");
    assert_eq!(tracker.count_online("course/1"), 1);
}

#[test]
fn idle_sweeps_never_detach_a_live_subscriber() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let manager = Arc::new(BroadcastManager::new());
    let stop = Arc::new(AtomicBool::new(false));
    let sweeper = {
        let manager = Arc::clone(&manager);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                manager.remove_idle_channels();
            }
        })
    };
    let subscribers: Vec<_> = (0..4)
        .map(|worker| {
            let manager = Arc::clone(&manager);
            std::thread::spawn(move || {
                for round in 0..2_000 {
                    let name = format!("room/{worker}/{}", round % 4);
                    let mut receiver = manager.get_or_create(&name).subscribe();
                    assert_eq!(manager.publish(&name, "tick", "{}"), Ok(1));
                    assert_eq!(receiver.try_recv().expect("delivered").event, "tick");
                }
            })
        })
        .collect();

    for subscriber in subscribers {
        subscriber.join().unwrap();
    }
    stop.store(true, Ordering::Relaxed);
    sweeper.join().unwrap();
    manager.remove_idle_channels();
    assert_eq!(manager.channels.len(), 0);
}
