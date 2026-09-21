use super::{
    Backend, RedisBroker,
    transport::{corrupt, digest, field},
};
use crate::{
    AckToken, DeadLetter, DeadLetterQuery, Delivery, FailureCode, MessageAdmin, MessageBroker,
    MessageEnvelope, MessageId, PublishReceipt, PublishRequest, PurgeReceipt, PurgeRequest,
    ReceiveRequest, Result, RetryDisposition, StartPosition, SubscriptionReceipt,
    SubscriptionRequest, WireEnvelopeCodec,
};
use redis::Value;
use std::time::Duration;

const PUBLISH: &str = include_str!("scripts/publish.lua");
const SUBSCRIBE: &str = include_str!("scripts/subscribe.lua");
const RECEIVE: &str = include_str!("scripts/receive.lua");
const SETTLE: &str = include_str!("scripts/settle.lua");
const ADMIN: &str = include_str!("scripts/admin.lua");

fn bytes(value: impl ToString) -> Vec<u8> {
    value.to_string().into_bytes()
}
fn topic(value: &str) -> Vec<u8> {
    digest(value.as_bytes()).into_bytes()
}
fn group(topic: &str, name: &str) -> Vec<u8> {
    format!("{}:{}", digest(topic.as_bytes()), digest(name.as_bytes())).into_bytes()
}

impl MessageBroker for RedisBroker {
    async fn publish(&self, request: PublishRequest) -> Result<PublishReceipt> {
        let Backend::Remote(remote) = &self.backend else {
            let Backend::Mock(mock) = &self.backend else {
                return Err(corrupt());
            };
            return mock.publish(request).await;
        };
        let config = &remote.config.broker;
        request.validate_payload(config.max_payload_bytes())?;
        let id = MessageId::random();
        // The script patches exactly this timestamp field with authoritative
        // server time before appending. No second network round trip is needed.
        let envelope =
            MessageEnvelope::from_request(&request, config.namespace().clone(), id.clone(), 0);
        let offset = 8
            + 10
            + id.as_str().len()
            + config.namespace().as_str().len()
            + request.topic().as_str().len()
            + request.event_kind().as_str().len()
            + request.content_type().as_str().len();
        let response = remote
            .run(
                PUBLISH,
                vec![
                    topic(request.topic().as_str()),
                    bytes(request.idempotency_key().as_str()),
                    digest(&request.fingerprint()?).into_bytes(),
                    bytes(id.as_str()),
                    bytes(offset),
                    WireEnvelopeCodec::encode(&envelope, config)?,
                ],
            )
            .await?;
        if response.len() != 3 {
            return Err(corrupt());
        }
        let duplicate: u8 = field(&response, 1)?;
        let published: i64 = field(&response, 2)?;
        if duplicate > 1 || published < 0 {
            return Err(corrupt());
        }
        Ok(PublishReceipt::new(
            MessageId::from_stored(field(&response, 0)?)?,
            duplicate == 1,
            published,
        ))
    }

    async fn subscribe(&self, request: SubscriptionRequest) -> Result<SubscriptionReceipt> {
        match &self.backend {
            Backend::Mock(mock) => mock.subscribe(request).await,
            Backend::Remote(remote) => {
                let start = match request.start() {
                    StartPosition::Earliest => "earliest",
                    StartPosition::Latest => "latest",
                };
                let values = remote
                    .run(
                        SUBSCRIBE,
                        vec![
                            topic(request.topic().as_str()),
                            group(request.topic().as_str(), request.group().as_str()),
                            bytes(start),
                        ],
                    )
                    .await?;
                if values.len() != 2 {
                    return Err(corrupt());
                }
                let created: u8 = field(&values, 0)?;
                let pending: usize = field(&values, 1)?;
                if created > 1 || pending > remote.config.broker.max_retained_messages() {
                    return Err(corrupt());
                }
                Ok(SubscriptionReceipt::new(created == 1, pending))
            }
        }
    }

    async fn receive(&self, request: ReceiveRequest) -> Result<Vec<Delivery>> {
        match &self.backend {
            Backend::Mock(mock) => mock.receive(request).await,
            Backend::Remote(remote) => {
                let tokens: Vec<_> = (0..request.max_messages())
                    .map(|_| AckToken::random())
                    .collect();
                let mut args = vec![
                    group(request.topic().as_str(), request.group().as_str()),
                    bytes(request.max_messages()),
                    bytes(request.lease_millis()),
                ];
                args.extend(tokens.iter().map(|token| bytes(token.as_str())));
                let values = remote.run(RECEIVE, args).await?;
                if values.len() > tokens.len() {
                    return Err(corrupt());
                }
                let mut deliveries = Vec::with_capacity(values.len());
                for (index, value) in values.iter().enumerate() {
                    let Value::Array(row) = value else {
                        return Err(corrupt());
                    };
                    if row.len() != 4 {
                        return Err(corrupt());
                    }
                    let wire: Vec<u8> = field(row, 0)?;
                    let envelope = WireEnvelopeCodec::decode(&wire, &remote.config.broker)?;
                    let attempt: u32 = field(row, 1)?;
                    let expires: i64 = field(row, 2)?;
                    let token: String = field(row, 3)?;
                    let expected = tokens.get(index).ok_or_else(corrupt)?;
                    if envelope.topic() != request.topic()
                        || attempt == 0
                        || attempt > remote.config.broker.max_attempts()
                        || expires < 0
                        || expected.as_str() != token
                    {
                        return Err(corrupt());
                    }
                    deliveries.push(Delivery::new(
                        envelope,
                        request.group().clone(),
                        request.consumer().clone(),
                        attempt,
                        expires,
                        expected.clone(),
                    ));
                }
                Ok(deliveries)
            }
        }
    }

    async fn ack(&self, token: &AckToken) -> Result<()> {
        match &self.backend {
            Backend::Mock(mock) => mock.ack(token).await,
            Backend::Remote(remote) => {
                let values = remote
                    .run(
                        SETTLE,
                        vec![bytes(token.as_str()), bytes("ack"), bytes(0), bytes("")],
                    )
                    .await?;
                if !values.is_empty() {
                    return Err(corrupt());
                }
                Ok(())
            }
        }
    }

    async fn retry(
        &self,
        token: &AckToken,
        delay: Duration,
        failure_code: FailureCode,
    ) -> Result<RetryDisposition> {
        match &self.backend {
            Backend::Mock(mock) => mock.retry(token, delay, failure_code).await,
            Backend::Remote(remote) => {
                if delay.as_millis() > u128::from(crate::validation::MAX_RETRY_MILLIS) {
                    return Err(super::config::invalid(
                        "retry delay",
                        "must not exceed seven days",
                    ));
                }
                let values = remote
                    .run(
                        SETTLE,
                        vec![
                            bytes(token.as_str()),
                            bytes("retry"),
                            bytes(delay.as_millis()),
                            bytes(failure_code.as_str()),
                        ],
                    )
                    .await?;
                if values.len() != 2 {
                    return Err(corrupt());
                }
                let state: String = field(&values, 0)?;
                let timestamp: i64 = field(&values, 1)?;
                match state.as_str() {
                    "retry" if timestamp >= 0 => Ok(RetryDisposition::Scheduled {
                        available_at_ms: timestamp,
                    }),
                    "dead" if timestamp == 0 => Ok(RetryDisposition::DeadLettered),
                    _ => Err(corrupt()),
                }
            }
        }
    }

    async fn dead_letter(&self, token: &AckToken, failure_code: FailureCode) -> Result<()> {
        match &self.backend {
            Backend::Mock(mock) => mock.dead_letter(token, failure_code).await,
            Backend::Remote(remote) => {
                let values = remote
                    .run(
                        SETTLE,
                        vec![
                            bytes(token.as_str()),
                            bytes("dead"),
                            bytes(0),
                            bytes(failure_code.as_str()),
                        ],
                    )
                    .await?;
                if !values.is_empty() {
                    return Err(corrupt());
                }
                Ok(())
            }
        }
    }
}

impl MessageAdmin for RedisBroker {
    async fn dead_letters(&self, query: DeadLetterQuery) -> Result<Vec<DeadLetter>> {
        match &self.backend {
            Backend::Mock(mock) => mock.dead_letters(query).await,
            Backend::Remote(remote) => {
                let values = remote
                    .run(
                        ADMIN,
                        vec![
                            bytes("dead"),
                            group(query.topic().as_str(), query.group().as_str()),
                            bytes(query.limit()),
                        ],
                    )
                    .await?;
                if values.len() > query.limit() {
                    return Err(corrupt());
                }
                values
                    .iter()
                    .map(|value| {
                        let Value::Array(row) = value else {
                            return Err(corrupt());
                        };
                        if row.len() != 4 {
                            return Err(corrupt());
                        }
                        let wire: Vec<u8> = field(row, 0)?;
                        let envelope = WireEnvelopeCodec::decode(&wire, &remote.config.broker)?;
                        let attempts: u32 = field(row, 1)?;
                        let timestamp: i64 = field(row, 3)?;
                        let failure: String = field(row, 2)?;
                        if envelope.topic() != query.topic()
                            || attempts == 0
                            || attempts > remote.config.broker.max_attempts()
                            || timestamp < 0
                        {
                            return Err(corrupt());
                        }
                        Ok(DeadLetter::new(
                            envelope,
                            query.group().clone(),
                            attempts,
                            FailureCode::try_new(failure).map_err(|_| corrupt())?,
                            timestamp,
                        ))
                    })
                    .collect()
            }
        }
    }

    async fn purge_terminal(&self, request: PurgeRequest) -> Result<PurgeReceipt> {
        match &self.backend {
            Backend::Mock(mock) => mock.purge_terminal(request).await,
            Backend::Remote(remote) => {
                let values = remote
                    .run(
                        ADMIN,
                        vec![
                            bytes("purge"),
                            topic(request.topic().as_str()),
                            bytes(request.limit()),
                        ],
                    )
                    .await?;
                if values.len() != 1 {
                    return Err(corrupt());
                }
                let removed: usize = field(&values, 0)?;
                if removed > request.limit().min(100) {
                    return Err(corrupt());
                }
                Ok(PurgeReceipt::new(removed))
            }
        }
    }
}
