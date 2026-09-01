//! Type-indexed dependency injection container (`rullst::di`).
//!
//! Provides type-indexed registration and resolution for services,
//! repositories, and background handlers without runtime reflection. The
//! container still performs synchronization and type lookup at runtime.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

/// Strongly-typed error domain for Rullst Dependency Injection.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum DiError {
    /// The requested service type was not registered in the DI Container.
    #[error("Dependency of type '{0}' is not registered in the DI Container")]
    NotRegistered(&'static str),

    /// Downcasting the registered service to the target type failed.
    #[error("Failed to downcast dependency of type '{0}'")]
    DowncastFailed(&'static str),

    /// Injection constructor failed.
    #[error("Dependency injection failed: {0}")]
    InjectionFailed(String),
}

/// Trait implemented by types that can be automatically injected by Rullst DI.
pub trait Injectable: Sized + Send + Sync + 'static {
    /// Factory constructor for the service instance.
    fn inject(container: &Container) -> Result<Self, DiError>;
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
    pub fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, DiError> {
        let type_id = TypeId::of::<T>();
        if let Some(service) = self.services.get(&type_id) {
            if let Ok(downcasted) = service.clone().downcast::<T>() {
                return Ok(downcasted);
            }
            return Err(DiError::DowncastFailed(std::any::type_name::<T>()));
        }
        Err(DiError::NotRegistered(std::any::type_name::<T>()))
    }
}

/// Axum extractor for dependencies managed by Rullst DI Container.
///
/// Usage in route handlers:
/// ```rust,no_run
/// use axum::Json;
/// use rullst_core::di::Inject;
///
/// #[derive(Clone)]
/// struct User;
/// struct UserService;
///
/// impl UserService {
///     async fn list_users(&self) -> Vec<User> { vec![] }
/// }
///
/// async fn index(Inject(user_svc): Inject<UserService>) -> Json<Vec<User>> {
///     Json(user_svc.list_users().await)
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
                Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
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
    use axum::extract::FromRequestParts;
    use axum::http::Request;

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

    #[tokio::test]
    async fn extractor_resolves_container_direct_extension_and_missing_state() {
        let mut container = Container::new();
        container.register(DatabaseService {
            connection_string: "sqlite://container".to_string(),
        });
        let mut parts = Request::new(()).into_parts().0;
        parts.extensions.insert(Arc::new(container));

        let injected = Inject::<DatabaseService>::from_request_parts(&mut parts, &())
            .await
            .expect("registered container service");
        assert_eq!(injected.connection_string, "sqlite://container");

        let direct = Arc::new(DatabaseService {
            connection_string: "sqlite://extension".to_string(),
        });
        let mut direct_parts = Request::new(()).into_parts().0;
        direct_parts.extensions.insert(direct.clone());
        let injected = Inject::<DatabaseService>::from_request_parts(&mut direct_parts, &())
            .await
            .expect("direct typed extension");
        assert!(Arc::ptr_eq(&injected.0, &direct));

        let mut missing_parts = Request::new(()).into_parts().0;
        let Err((status, message)) =
            Inject::<DatabaseService>::from_request_parts(&mut missing_parts, &()).await
        else {
            panic!("missing dependency must fail closed");
        };
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(message.contains(std::any::type_name::<DatabaseService>()));
        assert!(!message.contains("sqlite://"));
    }

    #[test]
    fn typed_errors_do_not_hide_the_requested_dependency() {
        let missing = DiError::NotRegistered(std::any::type_name::<DatabaseService>());
        assert!(missing.to_string().contains("DatabaseService"));

        let injection = DiError::InjectionFailed("invalid service graph".to_string());
        assert_eq!(
            injection.to_string(),
            "Dependency injection failed: invalid service graph"
        );
    }
}
