//! The process-wide lazy-loading switch, in its own binary because it changes
//! global state. Unlike `lazy_load_test`, it needs no database, so it also
//! runs in the all-feature build.

#[test]
fn prevent_lazy_loading_toggles_the_flag_generated_loaders_read() {
    assert!(!rullst_orm::is_lazy_loading_prevented());
    rullst_orm::prevent_lazy_loading(true);
    assert!(rullst_orm::is_lazy_loading_prevented());
    rullst_orm::prevent_lazy_loading(false);
    assert!(!rullst_orm::is_lazy_loading_prevented());
}
