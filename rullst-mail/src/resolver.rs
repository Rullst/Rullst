// src/resolver.rs — Dynamic multi-tenant mail driver resolver.

use crate::drivers::{MailDriver, MailError};
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;
use rullst_core::security::TenantContext;
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

    /// Registers a custom `MailDriver` for a validated tenant ID.
    pub fn register(
        &self,
        tenant_id: impl Into<String>,
        driver: impl MailDriver + 'static,
    ) -> Result<(), MailError> {
        self.register_arc(tenant_id, Arc::new(driver))
    }

    /// Registers a custom driver using tenant identity established by trusted auth middleware.
    pub fn register_for_context(
        &self,
        context: &TenantContext,
        driver: impl MailDriver + 'static,
    ) -> Result<(), MailError> {
        self.register(&context.tenant_id, driver)
    }

    /// Registers an `Arc`-wrapped `MailDriver` for a validated tenant ID.
    pub fn register_arc(
        &self,
        tenant_id: impl Into<String>,
        driver: Arc<dyn MailDriver>,
    ) -> Result<(), MailError> {
        let tenant_id = tenant_id.into();
        DeliveryPipeline::validate_tenant_id(&tenant_id)?;
        let mut drivers = self.drivers.write().map_err(|_| registry_unavailable())?;
        drivers.insert(tenant_id, driver);
        Ok(())
    }

    /// Registers an `Arc`-wrapped driver for an authenticated tenant context.
    pub fn register_arc_for_context(
        &self,
        context: &TenantContext,
        driver: Arc<dyn MailDriver>,
    ) -> Result<(), MailError> {
        self.register_arc(&context.tenant_id, driver)
    }

    /// Removes a tenant's registered driver.
    pub fn remove(&self, tenant_id: &str) -> Result<Option<Arc<dyn MailDriver>>, MailError> {
        DeliveryPipeline::validate_tenant_id(tenant_id)?;
        Ok(self
            .drivers
            .write()
            .map_err(|_| registry_unavailable())?
            .remove(tenant_id))
    }

    /// Retrieves the driver registered for a tenant, if present.
    pub fn get_driver(&self, tenant_id: &str) -> Result<Option<Arc<dyn MailDriver>>, MailError> {
        DeliveryPipeline::validate_tenant_id(tenant_id)?;
        Ok(self
            .drivers
            .read()
            .map_err(|_| registry_unavailable())?
            .get(tenant_id)
            .cloned())
    }

    /// Retrieves the driver selected by an authenticated tenant context.
    pub fn get_driver_for_context(
        &self,
        context: &TenantContext,
    ) -> Result<Option<Arc<dyn MailDriver>>, MailError> {
        self.get_driver(&context.tenant_id)
    }

    /// Checks if a driver is registered for the specified tenant.
    pub fn has_tenant(&self, tenant_id: &str) -> Result<bool, MailError> {
        Ok(self.get_driver(tenant_id)?.is_some())
    }

    /// Returns the total number of registered tenant drivers.
    pub fn tenant_count(&self) -> Result<usize, MailError> {
        Ok(self
            .drivers
            .read()
            .map_err(|_| registry_unavailable())?
            .len())
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
        if let Some(driver) = self.get_driver(tenant_id)? {
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

    /// Dispatches with credentials selected directly from a trusted tenant context.
    ///
    /// The context is intentionally passed explicitly: the resolver does not use task-local or
    /// process-global ambient identity that could leak across concurrent requests.
    pub async fn send_for_context(
        &self,
        context: &TenantContext,
        message: &Message,
    ) -> Result<(), MailError> {
        self.send_for_tenant(&context.tenant_id, message).await
    }
}

fn registry_unavailable() -> MailError {
    MailError::ConfigError("tenant mail driver registry is unavailable".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::MemoryDriver;

    #[tokio::test]
    async fn poisoned_registry_fails_closed_without_default_delivery() {
        let (default_driver, default_store) = MemoryDriver::isolated();
        let resolver = Arc::new(TenantMailResolver::with_default(default_driver));
        let poison_target = Arc::clone(&resolver);
        let poisoner = std::thread::spawn(move || {
            let _guard = poison_target
                .drivers
                .write()
                .expect("registry lock before intentional poison");
            panic!("intentional test-only registry poison");
        });
        assert!(poisoner.join().is_err());

        let result = resolver
            .send_for_tenant(
                "tenant_acme",
                &Message::new()
                    .to("owner@acme.example")
                    .subject("Fail closed"),
            )
            .await;

        assert!(matches!(result, Err(MailError::ConfigError(_))));
        assert!(default_store.lock().expect("default store").is_empty());
    }
}
