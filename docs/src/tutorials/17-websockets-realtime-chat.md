# Tutorial 17: In-process realtime and presence

Rullst Core provides bounded in-process broadcast channels and presence state.
These primitives do not create an HTTP WebSocket endpoint by themselves and do
not synchronize independent server processes.

## Subscribe before publishing

```rust
use std::sync::Arc;

use rullst::realtime::{BroadcastManager, RealtimeError, RealtimeMessage};

async fn local_exchange() -> Result<RealtimeMessage, RealtimeError> {
    let manager = Arc::new(BroadcastManager::new());
    let mut receiver = manager.get_or_create("room:general").subscribe();

    manager.publish(
        "room:general",
        "message.created",
        r#"{"sender":"alice","content":"hello"}"#,
    )?;

    receiver
        .recv()
        .await
        .map_err(|error| RealtimeError::BroadcastError(error.to_string()))
}
```

`Channel` uses `tokio::sync::broadcast`: a slow receiver can lag, and publishing
without any receiver returns `RealtimeError::BroadcastError`.

## Bind channels and presence to an authenticated tenant

Construct `TenantContext` only from trusted authentication/membership state.
The wrappers validate logical names and ensure that identical room names use
different backend namespaces:

```rust
use std::sync::Arc;

use rullst::realtime::{
    BroadcastManager, PresenceTracker, TenantPresence, TenantRealtime,
};
use rullst::security::TenantContext;

fn publish_for_school() -> Result<(), Box<dyn std::error::Error>> {
    let context = TenantContext::try_new("school-alpha")?;
    let realtime = TenantRealtime::from_context(
        Arc::new(BroadcastManager::new()),
        &context,
    );
    let presence = TenantPresence::from_context(
        Arc::new(PresenceTracker::new()),
        &context,
    );

    let mut receiver = realtime.subscribe("course/42")?;
    presence.user_joined("course/42", "learner-7")?;
    realtime.publish("course/42", "lesson.completed", r#"{"lesson_id":9}"#)?;

    assert_eq!(presence.count_online("course/42")?, 1);
    assert!(receiver.try_recv().is_ok());
    Ok(())
}
```

The tenant wrapper limits names and payloads (64 KiB), but the application still
owns authentication, room-level authorization, connection lifecycle and replay.
Use `rullst::live::live_ws_handler` for the separate per-connection LiveComponent
flow, or build an Axum WebSocket handler around these primitives. A distributed
transport is still roadmap work.
