// tests/validation_storage_multitenant_test.rs — Comprehensive coverage for Storage & Multitenant.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::multitenant::{TENANT_CONTEXT, TenantConfig, TenantStrategy, current_tenant_id};
use rullst_core::storage::Storage;
use std::cell::RefCell;

#[tokio::test]
async fn test_local_storage_crud() {
    let tmp_path = std::env::temp_dir().join("rullst_test_storage_123");
    let storage = Storage::local(tmp_path.to_str().unwrap());

    // 1. Put
    let put_res = storage.put("uploads/avatar.png", b"fake_png_data").await;
    assert!(put_res.is_ok());

    // 2. Get
    let data = storage.get("uploads/avatar.png").await.unwrap();
    assert_eq!(data, b"fake_png_data");

    // Clean up
    let _ = std::fs::remove_dir_all(tmp_path);
}

#[test]
fn test_multitenant_config_and_context() {
    let config = TenantConfig::new(TenantStrategy::Header)
        .with_header_name("X-Custom-Tenant")
        .with_parameter_name("tenant_param")
        .with_domain_fallback("default_tenant");

    assert_eq!(config.header_name, "X-Custom-Tenant");
    assert_eq!(config.parameter_name, "tenant_param");
    assert_eq!(config.domain_fallback.as_deref(), Some("default_tenant"));

    // Scoped task-local tenant execution
    TENANT_CONTEXT.sync_scope(RefCell::new(Some("tenant_99".to_string())), || {
        assert_eq!(current_tenant_id(), Some("tenant_99".to_string()));
    });
    assert_eq!(current_tenant_id(), None);
}
