// src/resolver.rs — Dynamic multi-tenant mail driver resolver.

use crate::drivers::{MailDriver, MailError};
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Dynamic Multi-Tenant Mail Resolver.
///
/// In multi-tenant B2B SaaS applications, each tenant or organization may use
/// their own SMTP credentials, custom domain sender addresses, or dedicated REST API keys
/// (e.g. Resend, SendGrid, Postmark, AWS SES).
///
/// `TenantMailResolver` dynamically routes outbound emails to the driver configured
/// for the specified tenant, falling back to a global default driver if configured.
pub struct TenantMailResolver {
    drivers: RwLock<HashMap<String, Arc<dyn MailDriver>>>,
    default_driver: Option<Arc<dyn MailDriver>>,
}

impl Default for TenantMailResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantMailResolver {
    /// Creates a new empty `TenantMailResolver`.
    pub fn new() -> Self {
        Self {
            drivers: RwLock::new(HashMap::new()),
            default_driver: None,
        }
    }

    /// Creates a new `TenantMailResolver` with a fallback default mail driver.
    pub fn with_default(default_driver: impl MailDriver + 'static) -> Self {
        Self {
            drivers: RwLock::new(HashMap::new()),
            default_driver: Some(Arc::new(default_driver)),
        }
    }

    /// Creates a new `TenantMailResolver` with an `Arc`-wrapped default mail driver.
    pub fn with_default_arc(default_driver: Arc<dyn MailDriver>) -> Self {
        Self {
            drivers: RwLock::new(HashMap::new()),
            default_driver: Some(default_driver),
        }
    }

    /// Registers a custom `MailDriver` for a specific tenant ID.
    pub fn register(&self, tenant_id: impl Into<String>, driver: impl MailDriver + 'static) {
        if let Ok(mut map) = self.drivers.write() {
            map.insert(tenant_id.into(), Arc::new(driver));
        }
    }

    /// Registers an `Arc`-wrapped `MailDriver` for a specific tenant ID.
    pub fn register_arc(&self, tenant_id: impl Into<String>, driver: Arc<dyn MailDriver>) {
        if let Ok(mut map) = self.drivers.write() {
            map.insert(tenant_id.into(), driver);
        }
    }

    /// Removes a tenant's registered driver.
    pub fn remove(&self, tenant_id: &str) -> Option<Arc<dyn MailDriver>> {
        self.drivers
            .write()
            .ok()
            .and_then(|mut map| map.remove(tenant_id))
    }

    /// Retrieves the driver registered for a tenant, if present.
    pub fn get_driver(&self, tenant_id: &str) -> Option<Arc<dyn MailDriver>> {
        self.drivers
            .read()
            .ok()
            .and_then(|map| map.get(tenant_id).cloned())
    }

    /// Checks if a driver is registered for the specified tenant.
    pub fn has_tenant(&self, tenant_id: &str) -> bool {
        self.drivers
            .read()
            .map(|map| map.contains_key(tenant_id))
            .unwrap_or(false)
    }

    /// Returns the total number of registered tenant drivers.
    pub fn tenant_count(&self) -> usize {
        self.drivers.read().map(|map| map.len()).unwrap_or(0)
    }

    /// Dispatches an email using the driver registered for the specified tenant.
    ///
    /// If no driver is explicitly registered for `tenant_id`, the message is dispatched
    /// using the `default_driver` if one was provided; otherwise, returns `MailError::ConfigError`.
    pub async fn send_for_tenant(
        &self,
        tenant_id: &str,
        message: &Message,
    ) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare_for_tenant(tenant_id, message)?;
        let message = prepared.message();
        if let Some(driver) = self.get_driver(tenant_id) {
            driver.send(message).await
        } else if let Some(ref default) = self.default_driver {
            default.send(message).await
        } else {
            Err(MailError::ConfigError(format!(
                "No mail driver registered for tenant '{}' and no default fallback driver configured",
                tenant_id
            )))
        }
    }
}

#[async_trait]
impl MailDriver for TenantMailResolver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if let Some(ref default) = self.default_driver {
            default.send(message).await
        } else {
            Err(MailError::ConfigError(
                "TenantMailResolver invoked without tenant context and no default fallback driver is configured"
                    .to_string(),
            ))
        }
    }

    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        TenantMailResolver::send_for_tenant(self, tenant_id, message).await
    }
}
