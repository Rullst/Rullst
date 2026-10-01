//! Tenant isolation and constructor tests for the realtime facades.

#![allow(clippy::expect_used)]

use super::*;
use crate::security::TenantMembership;

#[tokio::test]
// TM-TENANT-04
async fn identical_channels_are_isolated_by_authenticated_tenant_context() {
    let membership = TenantMembership::try_new(["school-alpha", "school-beta"])
        .expect("valid tenant membership");
    let alpha_context = membership.select("school-alpha").expect("alpha membership");
    let beta_context = membership.select("school-beta").expect("beta membership");
    let manager = Arc::new(BroadcastManager::new());
    let alpha = TenantRealtime::from_context(Arc::clone(&manager), &alpha_context);
    let beta = TenantRealtime::from_context(manager, &beta_context);
    let mut alpha_receiver = alpha.subscribe("course/1").expect("alpha subscription");
    let mut beta_receiver = beta.subscribe("course/1").expect("beta subscription");

    alpha
        .publish("course/1", "lesson.completed", r#"{"lesson_id":7}"#)
        .expect("alpha publish");
    let alpha_message = alpha_receiver.recv().await.expect("alpha message");

    assert_eq!(alpha_message.channel, "tenants:school-alpha:course/1");
    assert_eq!(alpha_message.event, "lesson.completed");
    assert!(matches!(
        beta_receiver.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        beta.publish("../school-alpha/course/1", "lesson.completed", "{}"),
        Err(RealtimeError::InvalidChannel(_))
    ));
    assert!(matches!(
        beta.publish("course/1", "lesson completed", "{}"),
        Err(RealtimeError::InvalidEvent(_))
    ));
    assert!(matches!(
        beta.publish("course/1", "lesson.completed", &"x".repeat(65_537)),
        Err(RealtimeError::PayloadTooLarge { .. })
    ));

    let presence = Arc::new(PresenceTracker::new());
    let alpha_presence = TenantPresence::from_context(Arc::clone(&presence), &alpha_context);
    let beta_presence = TenantPresence::from_context(presence, &beta_context);
    alpha_presence
        .user_joined("course/1", "learner-7")
        .expect("alpha presence");
    assert_eq!(
        alpha_presence
            .count_online("course/1")
            .expect("alpha presence count"),
        1
    );
    assert_eq!(
        beta_presence
            .count_online("course/1")
            .expect("beta presence count"),
        0
    );
    assert!(matches!(
        beta_presence.user_joined("course/1", "learner 7"),
        Err(RealtimeError::InvalidPresenceIdentity(_))
    ));
}

#[tokio::test]
// TM-TENANT-04
async fn colon_in_tenant_or_channel_cannot_alias_another_namespace() {
    let membership =
        TenantMembership::try_new(["a", "a:b", "acme"]).expect("valid tenant membership");
    let a_context = membership.select("a").expect("a membership");
    let a_b_context = membership.select("a:b").expect("a:b membership");
    let acme_context = membership.select("acme").expect("acme membership");
    let manager = Arc::new(BroadcastManager::new());
    let a = TenantRealtime::from_context(Arc::clone(&manager), &a_context);
    let a_b = TenantRealtime::from_context(Arc::clone(&manager), &a_b_context);
    let acme = TenantRealtime::from_context(manager, &acme_context);

    assert_eq!(a.namespaced_channel("b:c").expect("a"), "tenants:a:b:c");
    assert_eq!(a_b.namespaced_channel("c").expect("a:b"), "tenants:a%3Ab:c");
    assert_eq!(
        acme.namespaced_channel("course:1").expect("acme"),
        "tenants:acme:course:1"
    );

    let mut a_receiver = a.subscribe("b:c").expect("a subscription");
    let mut a_b_receiver = a_b.subscribe("c").expect("a:b subscription");
    assert_eq!(
        a_b.publish("c", "lesson.completed", "{}")
            .expect("a:b publish"),
        1
    );
    assert_eq!(
        a_b_receiver.recv().await.expect("a:b message").channel,
        "tenants:a%3Ab:c"
    );
    assert!(matches!(
        a_receiver.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));

    let presence = Arc::new(PresenceTracker::new());
    let a_presence = TenantPresence::from_context(Arc::clone(&presence), &a_context);
    let a_b_presence = TenantPresence::from_context(presence, &a_b_context);
    assert_eq!(
        a_b_presence.namespaced_room("c").expect("a:b room"),
        "tenants:a%3Ab:c"
    );
    a_presence
        .user_joined("b:c", "learner-7")
        .expect("a presence");
    assert_eq!(a_presence.count_online("b:c").expect("a count"), 1);
    assert_eq!(a_b_presence.count_online("c").expect("a:b count"), 0);
}

#[test]
fn zero_capacity_channel_is_panic_free() {
    let channel = Channel::new("bounded", 0);
    let _receiver = channel.subscribe();
    assert_eq!(channel.name, "bounded");
}
