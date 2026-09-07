use crate::Error;

/// A trait for defining declarative authorization policies on Rullst Models.
/// If a model defines a policy via `#[orm(policy = "MyPolicy")]`, the ORM
/// will automatically invoke these methods before generated instance mutations.
/// Bulk `delete_all` is rejected for these models because it cannot authorize
/// individual rows. Use instance deletions inside a managed transaction instead.
/// Policies invoked by full `save`/`delete` run while the executor is borrowed.
/// Reentrant ordinary ORM access from those callbacks fails validation. Perform
/// database checks/locking explicitly before the mutation in the managed
/// transaction, then validate trusted context and model values in the policy.
/// By default, all operations are allowed.
#[async_trait::async_trait]
pub trait Policy<T: Send + Sync>: Send + Sync {
    /// Determines if the given model can be created (saved for the first time).
    async fn can_create(_model: &T) -> Result<bool, Error> {
        Ok(true)
    }

    /// Determines if the given model can be updated.
    async fn can_update(_model: &T) -> Result<bool, Error> {
        Ok(true)
    }

    /// Determines if the given model can be deleted.
    async fn can_delete(_model: &T) -> Result<bool, Error> {
        Ok(true)
    }

    /// Determines if the given model can be restored (if using soft deletes).
    async fn can_restore(_model: &T) -> Result<bool, Error> {
        Ok(true)
    }

    /// Determines if the given model can be force deleted.
    async fn can_force_delete(_model: &T) -> Result<bool, Error> {
        Ok(true)
    }
}
