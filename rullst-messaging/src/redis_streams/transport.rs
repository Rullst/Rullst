use super::RedisBrokerConfig;
use crate::{MessagingError, Result};
use redis::{AsyncConnectionConfig, Client, FromRedisValue, Value, aio::MultiplexedConnection};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

pub(super) struct Remote {
    pub(super) config: RedisBrokerConfig,
    client: Client,
    connection: Mutex<Option<MultiplexedConnection>>,
    admission: Arc<Semaphore>,
    prefix: String,
}

const COMMON: &str = include_str!("scripts/common.lua");

impl Remote {
    pub(super) async fn open(config: RedisBrokerConfig, provision: bool) -> Result<Self> {
        let info = config.connection_info()?;
        if matches!(info.addr(), redis::ConnectionAddr::TcpTls { .. })
            && rustls::crypto::CryptoProvider::get_default().is_none()
        {
            // Redis builds its TLS configuration from the process provider.
            // Feature unification can enable both ring and aws-lc-rs, making
            // Rustls' implicit selection panic. Preserve an application choice;
            // if another thread installs first, its provider remains in force.
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let client = match &config.ca_certificate {
            Some(certificate) => Client::build_with_tls(
                info,
                redis::TlsCertificates {
                    client_tls: None,
                    root_cert: Some(certificate.clone()),
                },
            ),
            None => Client::open(info),
        }
        .map_err(|_| unavailable())?;
        let prefix = format!(
            "rullst:messaging:v1:{}:",
            digest(config.broker.namespace().as_str().as_bytes())
        );
        let remote = Self {
            config,
            client,
            connection: Mutex::new(None),
            admission: Arc::new(Semaphore::new(8)),
            prefix,
        };
        remote
            .run(
                include_str!("scripts/open.lua"),
                vec![if provision {
                    b"1".to_vec()
                } else {
                    b"0".to_vec()
                }],
            )
            .await?;
        Ok(remote)
    }

    pub(super) async fn run(&self, script: &'static str, args: Vec<Vec<u8>>) -> Result<Vec<Value>> {
        let permit = self.admission.clone().try_acquire_owned().map_err(|_| {
            MessagingError::CapacityExceeded {
                resource: "concurrent Redis operations",
                limit: 8,
            }
        })?;
        let result = tokio::time::timeout(self.config.timeout, async {
            let mut connection = {
                let mut slot = self.connection.lock().await;
                if slot.is_none() {
                    let settings = AsyncConnectionConfig::new()
                        .set_connection_timeout(Some(self.config.timeout))
                        .set_response_timeout(Some(self.config.timeout))
                        .set_pipeline_buffer_size(8);
                    *slot = Some(
                        self.client
                            .get_multiplexed_async_connection_with_config(&settings)
                            .await
                            .map_err(|_| unavailable())?,
                    );
                }
                slot.as_ref().cloned().ok_or_else(unavailable)?
            };
            let source = format!("{COMMON}\n{script}");
            let response: std::result::Result<Value, _> = redis::cmd("EVAL")
                .arg(source)
                .arg(1)
                .arg(format!("{}meta", self.prefix))
                .arg(self.config.signature())
                .arg(self.config.broker.max_retained_messages())
                .arg(self.config.broker.max_subscriptions())
                .arg(self.config.broker.max_attempts())
                .arg(args)
                .query_async(&mut connection)
                .await;
            match response {
                Ok(value) => self.decode(value),
                Err(_) => {
                    *self.connection.lock().await = None;
                    Err(unavailable())
                }
            }
        })
        .await;
        drop(permit);
        match result {
            Ok(result) => result,
            Err(_) => {
                // Discard for future calls; a timed-out command may still commit.
                if let Ok(mut connection) = self.connection.try_lock() {
                    *connection = None;
                }
                Err(unavailable())
            }
        }
    }

    fn decode(&self, value: Value) -> Result<Vec<Value>> {
        let Value::Array(mut values) = value else {
            return Err(corrupt());
        };
        if values.is_empty() {
            return Err(corrupt());
        }
        let tag: String = convert(&values.remove(0))?;
        match tag.as_str() {
            "ok" => Ok(values),
            "conflict" => Err(MessagingError::IdempotencyConflict),
            "config" => Err(MessagingError::ConfigurationConflict),
            "missing" | "corrupt" => Err(corrupt()),
            "subscription" => Err(MessagingError::SubscriptionNotFound),
            "lease" => Err(MessagingError::LeaseNotFound),
            "expired" => Err(MessagingError::LeaseExpired),
            "clock" => Err(MessagingError::ClockOutOfRange),
            "messages" => Err(MessagingError::CapacityExceeded {
                resource: "retained messages",
                limit: self.config.broker.max_retained_messages(),
            }),
            "subscriptions" => Err(MessagingError::CapacityExceeded {
                resource: "message subscriptions",
                limit: self.config.broker.max_subscriptions(),
            }),
            "bytes" => Err(MessagingError::CapacityExceeded {
                resource: "Redis retained envelope bytes",
                limit: 64 * 1024 * 1024,
            }),
            "sequence" => Err(MessagingError::CapacityExceeded {
                resource: "Redis publication sequence",
                limit: usize::try_from(9_007_199_254_740_000_u64).unwrap_or(usize::MAX),
            }),
            _ => Err(corrupt()),
        }
    }
}

pub(super) fn convert<T: FromRedisValue>(value: &Value) -> Result<T> {
    T::from_redis_value(value.clone()).map_err(|_| corrupt())
}

pub(super) fn field<T: FromRedisValue>(values: &[Value], index: usize) -> Result<T> {
    convert(values.get(index).ok_or_else(corrupt)?)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn corrupt() -> MessagingError {
    MessagingError::CorruptStorage {
        context: "Redis namespace",
    }
}

fn unavailable() -> MessagingError {
    MessagingError::StorageUnavailable {
        operation: "Redis command",
    }
}
