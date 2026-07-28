use crate::RullstValue;
use std::future::Future;

tokio::task_local! {
    pub static CURRENT_TENANT: RullstValue;
}

pub async fn with_tenant<T, F, R>(tenant_id: T, f: F) -> R
where
    T: Into<RullstValue>,
    F: Future<Output = R>,
{
    CURRENT_TENANT.scope(tenant_id.into(), f).await
}

pub fn get_tenant_id() -> Option<RullstValue> {
    CURRENT_TENANT.try_with(|t| t.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tenant_id_returns_none_outside_scope() {
        // Outside any with_tenant scope, get_tenant_id must return None.
        let id = get_tenant_id();
        assert!(id.is_none(), "Expected None outside a tenant scope");
    }

    #[tokio::test]
    async fn test_with_tenant_sets_and_restores() {
        // Inside with_tenant the id is visible; outside, it is gone.
        let result = with_tenant("acme", async { get_tenant_id() }).await;
        assert!(matches!(result, Some(RullstValue::String(ref s)) if s == "acme"));
        // After the scope, it should be None again.
        assert!(get_tenant_id().is_none());
    }

    #[tokio::test]
    async fn test_nested_tenant_scopes() {
        let _ = with_tenant("outer", async {
            let outer_id = get_tenant_id();
            assert!(matches!(outer_id, Some(RullstValue::String(ref s)) if s == "outer"));

            let _ = with_tenant("inner", async {
                let inner_id = get_tenant_id();
                assert!(matches!(inner_id, Some(RullstValue::String(ref s)) if s == "inner"));
            })
            .await;

            let restored_outer_id = get_tenant_id();
            assert!(matches!(restored_outer_id, Some(RullstValue::String(ref s)) if s == "outer"));
        })
        .await;
    }

    #[tokio::test]
    async fn test_with_tenant_panic_cleanup() {
        let result = tokio::spawn(async {
            with_tenant("faulty_tenant", async {
                std::panic::panic_any("Something went wrong!");
            })
            .await;
        })
        .await;

        assert!(result.is_err());
        assert!(get_tenant_id().is_none());
    }
}
