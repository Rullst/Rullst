//! Internal guard for implicit executor access from borrowed mutation callbacks.

use std::future::Future;

tokio::task_local! {
    static IN_MUTATION_CALLBACK: ();
}

/// Generated mutation policies/hooks borrow an executor that cannot be lent
/// again through the process-global API. This scope restores itself on return,
/// cancellation, or unwind and does not propagate into independently spawned tasks.
pub async fn run<F: Future>(callback: F) -> F::Output {
    IN_MUTATION_CALLBACK.scope((), callback).await
}

pub fn ensure_allowed() -> Result<(), crate::Error> {
    if IN_MUTATION_CALLBACK.try_with(|()| ()).is_ok() {
        return Err(crate::Error::Validation(
            "reentrant ORM access from a mutation policy or lifecycle callback is unsupported while its transaction is borrowed; perform database authorization before the mutation and defer only post-commit effects to after_commit".to_string(),
        ));
    }
    Ok(())
}
