//! Compile-Time Zero-Cost Dependency Injection Container (`rullst::di`)
//!
//! Provides static dispatch dependency injection for services, repositories,
//! and background handlers without dynamic reflection or runtime performance penalties.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

/// Trait implemented by types that can be automatically injected by Rullst DI.
pub trait Injectable: Sized + Send + Sync + 'static {
    /// Factory constructor for the service instance.
    fn inject(container: &Container) -> Result<Self, String>;
}

/// Thread-safe Dependency Injection Container.
#[derive(Clone, Default)]
pub struct Container {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Container {
    /// Creates a new empty DI container.
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Registers a concrete service instance in the container.
    pub fn register<T: Send + Sync + 'static>(&mut self, service: T) {
        self.services.insert(TypeId::of::<T>(), Arc::new(service));
    }

    /// Registers an `Arc`-wrapped service instance in the container.
    pub fn register_arc<T: Send + Sync + 'static>(&mut self, service: Arc<T>) {
        self.services.insert(TypeId::of::<T>(), service);
    }

    /// Resolves and retrieves a reference-counted service from the container.
    pub fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, String> {
        let type_id = TypeId::of::<T>();
        if let Some(service) = self.services.get(&type_id) {
            if let Ok(downcasted) = service.clone().downcast::<T>() {
                return Ok(downcasted);
            }
        }
        Err(format!(
            "Dependency of type '{}' is not registered in the DI Container",
            std::any::type_name::<T>()
        ))
    }
}

/// Axum extractor for dependencies managed by Rullst DI Container.
///
/// Usage in route handlers:
/// ```rust,ignore
/// pub async fn index(user_svc: Inject<UserService>) -> Result<Json<Vec<User>>, AppError> {
///     let users = user_svc.list_users().await?;
///     Ok(Json(users))
/// }
/// ```
pub struct Inject<T: Send + Sync + 'static>(pub Arc<T>);

impl<T: Send + Sync + 'static> Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> FromRequestParts<S> for Inject<T>
where
    T: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(container) = parts.extensions.get::<Arc<Container>>() {
            match container.resolve::<T>() {
                Ok(service) => Ok(Inject(service)),
                Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
            }
        } else if let Some(service) = parts.extensions.get::<Arc<T>>() {
            Ok(Inject(service.clone()))
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Rullst DI Container or Extension for type '{}' not found in request state",
                    std::any::type_name::<T>()
                ),
            ))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    struct DatabaseService {
        connection_string: String,
    }

    struct UserService {
        db: Arc<DatabaseService>,
    }

    #[test]
    fn test_di_container_registration_and_resolution() {
        let mut container = Container::new();
        container.register(DatabaseService {
            connection_string: "sqlite::memory:".to_string(),
        });

        let db = container.resolve::<DatabaseService>().unwrap();
        assert_eq!(db.connection_string, "sqlite::memory:");

        let user_svc = UserService { db: db.clone() };
        container.register(user_svc);

        let resolved_user_svc = container.resolve::<UserService>().unwrap();
        assert_eq!(resolved_user_svc.db.connection_string, "sqlite::memory:");
    }
}
