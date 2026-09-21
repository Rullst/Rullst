use super::{
    LiveCommand, LiveRecoveryConfig, LiveRecoveryError, LiveResult, LiveScope, LiveSnapshot,
    RecoverableLiveView, types::SnapshotWire,
};
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use std::future::Future;
use tokio::time::{Instant, MissedTickBehavior};

pub(super) async fn run<C: RecoverableLiveView>(
    mut socket: WebSocket,
    config: LiveRecoveryConfig,
    scope: LiveScope,
    component: C,
) {
    let result = tokio::time::timeout(
        config.max_lifetime,
        session(&mut socket, &config, &scope, &component),
    )
    .await;
    let (code, reason) = match result {
        Ok(Ok(())) => (1000, "complete"),
        Ok(Err(LiveRecoveryError::Unauthorized)) => (4401, "access denied"),
        Ok(Err(LiveRecoveryError::Invalid)) => (4400, "invalid protocol"),
        _ => (1013, "refresh required"),
    };
    let _ = tokio::time::timeout(
        config.operation_timeout,
        socket.send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        }))),
    )
    .await;
}

async fn bounded<T>(
    config: &LiveRecoveryConfig,
    future: impl Future<Output = LiveResult<T>>,
) -> LiveResult<T> {
    tokio::time::timeout(config.operation_timeout, future)
        .await
        .map_err(|_| LiveRecoveryError::Unavailable)?
}

async fn send_snapshot<C: RecoverableLiveView>(
    socket: &mut WebSocket,
    config: &LiveRecoveryConfig,
    scope: &LiveScope,
    component: &C,
    snapshot: &LiveSnapshot,
    id: Option<&str>,
    outcome: &'static str,
) -> LiveResult<()> {
    // Recheck after domain I/O: an authorization that expired while awaiting a
    // snapshot/action must not authorize disclosure of the resulting state.
    bounded(config, component.authorize(scope)).await?;
    let wire = SnapshotWire {
        version: 1,
        kind: "snapshot",
        revision: snapshot.revision().to_string(),
        html: snapshot.html(),
        id,
        outcome,
    };
    let encoded = serde_json::to_string(&wire).map_err(|_| LiveRecoveryError::Invalid)?;
    if encoded.len() > 512 * 1024 {
        return Err(LiveRecoveryError::Invalid);
    }
    bounded(config, async {
        socket
            .send(Message::Text(encoded.into()))
            .await
            .map_err(|_| LiveRecoveryError::Unavailable)
    })
    .await
}

async fn session<C: RecoverableLiveView>(
    socket: &mut WebSocket,
    config: &LiveRecoveryConfig,
    scope: &LiveScope,
    component: &C,
) -> LiveResult<()> {
    bounded(config, component.authorize(scope)).await?;
    let initial = bounded(config, component.snapshot(scope)).await?;
    send_snapshot(
        socket,
        config,
        scope,
        component,
        &initial,
        None,
        "recovered",
    )
    .await?;
    let mut revision = initial.revision();
    let mut actions = 0usize;
    let mut tick = tokio::time::interval_at(
        Instant::now() + config.revalidate_every,
        config.revalidate_every,
    );
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_peer = Instant::now();
    loop {
        tokio::select! {
            _ = tick.tick() => {
                bounded(config, component.authorize(scope)).await?;
                if last_peer.elapsed() > config.revalidate_every.saturating_mul(3) { return Err(LiveRecoveryError::Unavailable); }
                bounded(config, async { socket.send(Message::Ping(Vec::new().into())).await.map_err(|_| LiveRecoveryError::Unavailable) }).await?;
            }
            incoming = socket.recv() => {
                let message = match incoming { Some(Ok(message)) => message, Some(Err(_)) => return Err(LiveRecoveryError::Invalid), None => return Ok(()) };
                last_peer = Instant::now();
                match message {
                    Message::Text(text) => {
                        if actions >= config.max_actions { return Err(LiveRecoveryError::Unavailable); }
                        actions += 1;
                        let command = LiveCommand::decode(&text)?;
                        bounded(config, component.authorize(scope)).await?;
                        let (snapshot, outcome) = if command.expected_revision() != revision {
                            (bounded(config, component.snapshot(scope)).await?, "conflict")
                        } else {
                            match bounded(config, component.apply(scope, &command)).await {
                                Ok(snapshot) if snapshot.revision() > revision => (snapshot, "applied"),
                                Ok(_) => return Err(LiveRecoveryError::Unavailable),
                                Err(LiveRecoveryError::Conflict) => (bounded(config, component.snapshot(scope)).await?, "conflict"),
                                Err(error) => return Err(error),
                            }
                        };
                        if snapshot.revision() < revision { return Err(LiveRecoveryError::Unavailable); }
                        send_snapshot(socket, config, scope, component, &snapshot, Some(command.id()), outcome).await?;
                        revision = snapshot.revision();
                    }
                    Message::Ping(_) | Message::Pong(_) => (),
                    Message::Close(_) => return Ok(()),
                    Message::Binary(_) => return Err(LiveRecoveryError::Invalid),
                }
            }
        }
    }
}
