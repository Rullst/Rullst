use rullst_core::{
    frontend::{HtmlAdapter, LeptosAdapter, DioxusAdapter},
    realtime::{BroadcastManager, PresenceTracker},
};

#[test]
fn test_frontend_adapters() {
    let html = HtmlAdapter("<h1>Hello World</h1>".to_string());
    assert_eq!(html.0, "<h1>Hello World</h1>");

    let leptos = LeptosAdapter::new("<div>Leptos View</div>");
    assert_eq!(leptos.view, "<div>Leptos View</div>");

    let dioxus = DioxusAdapter::new("<div>Dioxus Component</div>");
    assert_eq!(dioxus.component, "<div>Dioxus Component</div>");
}

#[tokio::test]
async fn test_realtime_broadcast_and_presence() {
    let bm = BroadcastManager::new();
    let ch = bm.get_or_create("chat-room-1");
    let mut rx = ch.subscribe();

    bm.publish("chat-room-1", "user_joined", "{\"user\":\"alice\"}").unwrap();

    let msg = rx.recv().await.unwrap();
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
