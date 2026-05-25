use rullst::feature::{
    self, DbFeatureDriver, EnvFeatureDriver, FeatureDriver, FeatureManager, MemoryFeatureDriver,
    TomlFeatureDriver,
};
use rust_eloquent::Eloquent;
use std::fs;
use std::time::Duration;

#[tokio::test]
async fn test_memory_feature_driver() {
    let memory_driver = MemoryFeatureDriver::new();
    memory_driver.override_enabled("new-checkout", true);
    memory_driver.override_enabled("legacy-sidebar", false);

    assert_eq!(memory_driver.enabled("new-checkout").await, Some(true));
    assert_eq!(memory_driver.enabled("legacy-sidebar").await, Some(false));
    assert_eq!(memory_driver.enabled("non-existent").await, None);

    // Rollout percentage test (30%)
    memory_driver.override_rollout("progressive-rollout", 30);
    // Let's test with various user IDs and verify stable bucket mapping
    let mut enabled_count = 0;
    for i in 0..100 {
        let user_id = format!("user_{}", i);
        if memory_driver
            .enabled_for("progressive-rollout", &user_id)
            .await
            .unwrap_or(false)
        {
            enabled_count += 1;
        }
    }
    // With 100 users, deterministic hash mapping should yield some users enabled and some disabled.
    // In our algorithm, user_12 bucket is 76 (disabled), user_4 bucket is 24 (enabled), etc.
    assert!(enabled_count > 10 && enabled_count < 50);

    // A/B variations test
    let variants = vec![("red".to_string(), 50), ("blue".to_string(), 50)];
    memory_driver.override_variants("button-color", variants);

    let mut red_count = 0;
    let mut blue_count = 0;
    for i in 0..100 {
        let user_id = format!("user_{}", i);
        let var = memory_driver.variant("button-color", &user_id).await;
        if var == Some("red".to_string()) {
            red_count += 1;
        } else if var == Some("blue".to_string()) {
            blue_count += 1;
        }
    }
    assert!(red_count > 30 && blue_count > 30);
    assert_eq!(red_count + blue_count, 100);
}

#[tokio::test]
async fn test_env_feature_driver() {
    let env_driver = EnvFeatureDriver::new();

    unsafe {
        std::env::set_var("FEATURE_BETA_FLAG", "true");
        std::env::set_var("FEATURE_BETA_PCT", "30%");
        std::env::set_var("FEATURE_THEME_SPLIT", "light:50,dark:50");
    }

    assert_eq!(env_driver.enabled("beta-flag").await, Some(true));

    // Percent evaluation
    // Since user_4 hash bucket for "beta-pct" is deterministic, let's verify
    let bucket_user_4 = feature::calculate_hash_bucket("beta-pct", "user_4"); // let's see bucket
    let user_4_enabled = env_driver
        .enabled_for("beta-pct", "user_4")
        .await
        .unwrap_or(false);
    assert_eq!(user_4_enabled, bucket_user_4 < 30);

    // Variants evaluation
    let var_user_1 = env_driver.variant("theme-split", "user_1").await.unwrap();
    assert!(var_user_1 == "light" || var_user_1 == "dark");

    // Clean up
    unsafe {
        std::env::remove_var("FEATURE_BETA_FLAG");
        std::env::remove_var("FEATURE_BETA_PCT");
        std::env::remove_var("FEATURE_THEME_SPLIT");
    }
}

#[tokio::test]
async fn test_toml_feature_driver() {
    // Mock a Rullst.toml file
    let toml_mock = r#"
[server]
port = 3000

[features]
billing-v2 = true
admin-redesign = "50%"
home-ab = "control:40,treatment:60"
"#;

    fs::write("Rullst.toml", toml_mock).unwrap();

    let toml_driver = TomlFeatureDriver::new();

    assert_eq!(toml_driver.enabled("billing-v2").await, Some(true));

    let bucket = feature::calculate_hash_bucket("admin-redesign", "user_99");
    assert_eq!(
        toml_driver.enabled_for("admin-redesign", "user_99").await,
        Some(bucket < 50)
    );

    let ab_var = toml_driver.variant("home-ab", "user_50").await.unwrap();
    assert!(ab_var == "control" || ab_var == "treatment");

    // Clean up the mock Rullst.toml
    let _ = fs::remove_file("Rullst.toml");
}

#[tokio::test]
async fn test_database_feature_driver() {
    // 1. Initialize SQLite in-memory database
    Eloquent::init("sqlite:file:memdb1?mode=memory&cache=shared")
        .await
        .unwrap();
    let pool = Eloquent::pool();

    // Acquire and hold a connection to keep the in-memory database alive
    let _conn = pool.acquire().await.unwrap();

    // 2. Create the table schema
    sqlx::query(
        "CREATE TABLE rullst_feature_flags (
            name TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 0,
            rollout_percentage INTEGER DEFAULT NULL,
            variants TEXT DEFAULT NULL
        )",
    )
    .execute(pool)
    .await
    .unwrap();

    // 3. Seed some flag values
    sqlx::query(
        "INSERT INTO rullst_feature_flags (name, enabled, rollout_percentage, variants) 
         VALUES 
         ('db-dashboard', 1, NULL, NULL),
         ('db-rollout', 1, 40, NULL),
         ('db-ab-split', 1, NULL, 'variant-a:30,variant-b:70')",
    )
    .execute(pool)
    .await
    .unwrap();

    // 4. Test DbFeatureDriver with a short TTL (100ms) to verify caching behavior
    let db_driver = DbFeatureDriver::with_ttl(Duration::from_millis(100));

    assert_eq!(db_driver.enabled("db-dashboard").await, Some(true));

    let rollout_bucket = feature::calculate_hash_bucket("db-rollout", "user_x");
    assert_eq!(
        db_driver.enabled_for("db-rollout", "user_x").await,
        Some(rollout_bucket < 40)
    );

    let ab_var = db_driver.variant("db-ab-split", "user_y").await.unwrap();
    assert!(ab_var == "variant-a" || ab_var == "variant-b");

    // 5. Update DB directly and verify the local cache is maintained during the TTL window
    sqlx::query("UPDATE rullst_feature_flags SET enabled = 0 WHERE name = 'db-dashboard'")
        .execute(pool)
        .await
        .unwrap();

    // Cache should still return `true` due to 100ms cache TTL
    assert_eq!(db_driver.enabled("db-dashboard").await, Some(true));

    // Wait for the cache entry to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Cache has expired; should query the DB and fetch the new state (`false`)
    assert_eq!(db_driver.enabled("db-dashboard").await, Some(false));
}

#[tokio::test]
async fn test_global_feature_manager_facade() {
    let memory_driver = Box::new(MemoryFeatureDriver::new());
    memory_driver.override_enabled("global-toggle", true);

    let manager = FeatureManager::new().add_driver(memory_driver);
    feature::init(manager).unwrap_or(());

    // Verify static wrapper functions evaluate correctly
    assert!(feature::enabled("global-toggle").await);
    assert!(!feature::enabled("non-existent-global").await);
}
