use rullst_core::realtime::{BroadcastManager, PresenceTracker};

#[tokio::test]
async fn test_realtime_broadcast_and_presence() {
    let bm = BroadcastManager::new();
    let ch = bm.get_or_create("chat-room-1");
    let mut rx = ch.subscribe();

    bm.publish("chat-room-1", "user_joined", "{\"user\":\"alice\"}")
        .unwrap();

    // A deadline turns a message that never arrives into a failure.
    let msg = tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
        .await
        .expect("message within the deadline")
        .unwrap();
    assert_eq!(msg.channel, "chat-room-1");
    assert_eq!(msg.event, "user_joined");
    assert_eq!(msg.payload, "{\"user\":\"alice\"}");

    let presence = PresenceTracker::new();
    assert_eq!(presence.count_online("chat-room-1"), 0);

    presence.user_joined("chat-room-1", "usr_100");
    assert_eq!(presence.count_online("chat-room-1"), 1);

    presence.user_left("chat-room-1", "usr_100");
    assert_eq!(presence.count_online("chat-room-1"), 0);
}
