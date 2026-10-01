//! `#[rullst_orm::test]`: runs an async test inside a task-scoped ORM
//! transaction that is always rolled back.

use proc_macro2::TokenStream;
use quote::quote;

/// Expands the test function. Its declared return type is kept and the
/// body's value is returned after the rollback, so a test returning `Err`
/// fails like any other `#[tokio::test]`.
pub(crate) fn expand(input_fn: &syn::ItemFn) -> TokenStream {
    let fn_name = &input_fn.sig.ident;
    let vis = &input_fn.vis;
    let block = &input_fn.block;
    let attrs = &input_fn.attrs;
    let output = &input_fn.sig.output;

    quote! {
        #(#attrs)*
        #[::tokio::test]
        #vis async fn #fn_name() #output {
            // The body keeps the declared return type, so `?` and `return`
            // resolve against it.
            async fn __rullst_test_body() #output #block

            // Ensure DB is initialized (if already initialized in parallel, it ignores the error)
            let _ = ::rullst_orm::Orm::init(&::std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string())).await;

            // SQLite permits only one concurrent schema writer. Keep generated
            // sandbox tests deterministic while preserving parallel execution
            // for database servers that support it.
            let _sqlite_sandbox_guard = if matches!(::rullst_orm::Orm::try_driver(), Ok("sqlite")) {
                Some(::rullst_orm::TEST_SANDBOX_LOCK.lock().await)
            } else {
                None
            };

            // Start transaction for Sandbox isolation
            let tx = ::rullst_orm::Orm::begin_transaction().await.expect("Failed to begin sandbox transaction");
            let tx_arc = ::std::sync::Arc::new(::tokio::sync::Mutex::new(Some(tx)));

            // Scope the transaction globally for this tokio task. Its
            // post-commit callbacks are collected like those of
            // `Orm::transaction`, instead of running while a mutation still
            // holds the transaction; the sandbox never commits, so they are
            // discarded with the scope, as a rollback discards them.
            let __rullst_post_commit = ::rullst_orm::post_commit::PostCommitScope::new();
            let __rullst_test_result = __rullst_post_commit
                .run(::rullst_orm::CURRENT_TX.scope(tx_arc.clone(), __rullst_test_body()))
                .await;

            // Automatic Rollback
            if let Some(tx) = tx_arc.lock().await.take() {
                let _ = tx.rollback().await;
            }
            drop(__rullst_post_commit);
            __rullst_test_result
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    #[test]
    fn keeps_the_declared_return_type_and_returns_the_body_result() {
        let input: syn::ItemFn = parse_quote! {
            async fn fallible() -> Result<(), rullst_orm::Error> {
                missing().await?;
                Ok(())
            }
        };
        let expanded = super::expand(&input).to_string();
        assert!(expanded.contains("async fn fallible () -> Result < () , rullst_orm :: Error > {"));
        assert!(
            expanded.contains(
                "async fn __rullst_test_body () -> Result < () , rullst_orm :: Error > {"
            )
        );
        let after_rollback = expanded.split_once("tx . rollback ()").expect("rollback").1;
        assert!(
            after_rollback
                .trim_end()
                .ends_with("__rullst_test_result }")
        );
    }

    #[test]
    fn runs_the_body_under_a_post_commit_scope_that_is_never_committed() {
        let input: syn::ItemFn = parse_quote! {
            async fn sandboxed() {}
        };
        let expanded = super::expand(&input).to_string();
        assert!(
            expanded.contains("__rullst_post_commit . run (:: rullst_orm :: CURRENT_TX . scope")
        );
        assert!(expanded.contains("drop (__rullst_post_commit)"));
        assert!(!expanded.contains("__rullst_post_commit . commit"));
    }
}
