use rullst_core::{
    frontend::{
        DioxusAdapter, HtmlAdapter, LeptosAdapter, LiveViewAdapter, PicoAdapter, TemplateAdapter,
        TopcoatAdapter, WasmIslandAdapter,
    },
    realtime::{BroadcastManager, PresenceTracker},
};

#[test]
fn test_frontend_adapters() {
    let html = HtmlAdapter("<h1>Hello World</h1>".to_string());
    assert_eq!(html.0, "<h1>Hello World</h1>");

    let pico = PicoAdapter("<button>Click</button>".to_string());
    assert_eq!(pico.0, "<button>Click</button>");

    let topcoat: TopcoatAdapter = PicoAdapter("<button>Click</button>".to_string());
    assert_eq!(topcoat.0, "<button>Click</button>");

    let template = TemplateAdapter("<div>Rendered Jinja2 Template</div>".to_string());
    assert_eq!(template.0, "<div>Rendered Jinja2 Template</div>");

    let live = LiveViewAdapter::new("<div>LiveView Server UI</div>");
    assert_eq!(live.view, "<div>LiveView Server UI</div>");

    let island = WasmIslandAdapter::new("<div>Wasm Island Client UI</div>");
    assert_eq!(island.component, "<div>Wasm Island Client UI</div>");

    let leptos = LeptosAdapter::new("<div>Compatibility View</div>");
    assert_eq!(leptos.view, "<div>Compatibility View</div>");

    let dioxus = DioxusAdapter::new("<div>Compatibility Component</div>");
    assert_eq!(dioxus.component, "<div>Compatibility Component</div>");
}

#[tokio::test]
async fn test_realtime_broadcast_and_presence() {
    let bm = BroadcastManager::new();
    let ch = bm.get_or_create("chat-room-1");
    let mut rx = ch.subscribe();

    bm.publish("chat-room-1", "user_joined", "{\"user\":\"alice\"}")
        .unwrap();

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
