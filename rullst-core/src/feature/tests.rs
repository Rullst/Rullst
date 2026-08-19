#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::driver::FeatureDriver;
    use super::super::manager::FeatureManager;
    use super::super::memory::MemoryFeatureDriver;
    use super::super::resolvers::{
        calculate_hash_bucket, parse_feature_string_value, parse_rollout, parse_variants,
        resolve_variant,
    };
    use super::super::env::EnvFeatureDriver;
    use super::super::toml::TomlFeatureDriver;

    #[test]
    fn test_calculate_hash_bucket() {
        let b1 = calculate_hash_bucket("flag-a", "user-1");
        let b2 = calculate_hash_bucket("flag-a", "user-1");
        let b3 = calculate_hash_bucket("flag-a", "user-2");
        assert_eq!(b1, b2);
        assert!(b1 < 100);
        assert!(b3 < 100);
    }

    #[test]
    fn test_parse_rollout() {
        assert_eq!(parse_rollout("30%"), Some(30));
        assert_eq!(parse_rollout("  100% "), Some(100));
        assert_eq!(parse_rollout("0"), Some(0));
        assert_eq!(parse_rollout("abc"), None);
    }

    #[test]
    fn test_parse_variants() {
        let parsed = parse_variants("control:50,treatment:50");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "control");
        assert_eq!(parsed[0].1, 50);
        assert_eq!(parsed[1].0, "treatment");
        assert_eq!(parsed[1].1, 50);

        let parsed_empty = parse_variants("invalid");
        assert!(parsed_empty.is_empty());
    }

    #[test]
    fn test_resolve_variant() {
        let variants = vec![("control".to_string(), 30), ("treatment".to_string(), 70)];
        // bucket 10 should fall into control (0-29)
        assert_eq!(resolve_variant(&variants, 10), Some("control".to_string()));
        // bucket 50 should fall into treatment (30-99)
        assert_eq!(
            resolve_variant(&variants, 50),
            Some("treatment".to_string())
        );
        // bucket 101 is out of bounds
        assert_eq!(resolve_variant(&variants, 101), None);
    }

    #[tokio::test]
    async fn test_memory_driver_override_enabled() {
        let driver = MemoryFeatureDriver::new();
        assert_eq!(driver.enabled("test-flag").await, None);

        driver.override_enabled("test-flag", true);
        assert_eq!(driver.enabled("test-flag").await, Some(true));
        assert_eq!(driver.enabled_for("test-flag", "user1").await, Some(true));
        assert_eq!(
            driver.variant("test-flag", "user1").await,
            Some("enabled".to_string())
        );

        driver.override_enabled("test-flag-2", false);
        assert_eq!(driver.enabled("test-flag-2").await, Some(false));
        assert_eq!(
            driver.enabled_for("test-flag-2", "user1").await,
            Some(false)
        );
        assert_eq!(
            driver.variant("test-flag-2", "user1").await,
            Some("disabled".to_string())
        );
    }

    #[tokio::test]
    async fn test_memory_driver_rollout() {
        let driver = MemoryFeatureDriver::new();
        driver.override_rollout("rollout-flag", 50); // 50%

        let bucket = calculate_hash_bucket("rollout-flag", "user-in");
        // We just verify it doesn't crash and returns boolean based on bucket
        let res = driver.enabled_for("rollout-flag", "user-in").await.unwrap();
        assert_eq!(res, bucket < 50);

        let variant = driver.variant("rollout-flag", "user-in").await.unwrap();
        assert_eq!(variant, if bucket < 50 { "enabled" } else { "disabled" });
    }

    #[tokio::test]
    async fn test_memory_driver_variants() {
        let driver = MemoryFeatureDriver::new();
        driver.override_variants("variant-flag", vec![("a".to_string(), 100)]);
        assert_eq!(
            driver.variant("variant-flag", "user1").await,
            Some("a".to_string())
        );
    }

    #[test]
    fn test_parse_feature_string_value() {
        assert_eq!(
            parse_feature_string_value(" true ", "f", None),
            Some("enabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value(" 1 ", "f", None),
            Some("enabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value(" yes ", "f", None),
            Some("enabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value(" false ", "f", None),
            Some("disabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value(" 0 ", "f", None),
            Some("disabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value(" no ", "f", None),
            Some("disabled".to_string())
        );
        assert_eq!(parse_feature_string_value("", "f", None), None);
        assert_eq!(
            parse_feature_string_value("100%", "f", Some("u")),
            Some("enabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value("0%", "f", Some("u")),
            Some("disabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value("0%", "f", None),
            Some("disabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value("a:100", "f", Some("u")),
            Some("a".to_string())
        );
        assert_eq!(
            parse_feature_string_value("custom-string", "f", None),
            Some("custom-string".to_string())
        );
    }

    #[tokio::test]
    async fn test_env_driver() {
        let driver = EnvFeatureDriver::new();
        unsafe {
            std::env::set_var("FEATURE_MY_FLAG", "true");
        }
        assert_eq!(driver.enabled("my-flag").await, Some(true));
        assert_eq!(driver.enabled_for("my-flag", "u").await, Some(true));
        assert_eq!(
            driver.variant("my-flag", "u").await,
            Some("enabled".to_string())
        );
    }

    #[tokio::test]
    async fn test_feature_manager_default() {
        let manager = FeatureManager::default();
        assert!(!manager.enabled("non-existent").await);
        assert!(!manager.enabled_for("non-existent", "user").await);
        assert_eq!(manager.variant("non-existent", "user").await, None);
    }

    #[test]
    fn test_toml_feature_driver_load() {
        let driver = TomlFeatureDriver::new();
        driver.load_from_str(
            "
            [features]
            flag1 = true
            
            # this comment has an = sign
            flag2 = false
            ",
        );
        assert_eq!(driver.config.get("flag1").unwrap().value(), "true");
        assert_eq!(driver.config.get("flag2").unwrap().value(), "false");
        assert_eq!(driver.config.len(), 2);
    }

    #[tokio::test]
    async fn test_feature_manager() {
        let driver = MemoryFeatureDriver::new();
        driver.override_enabled("global-flag", true);

        let manager = FeatureManager::new().add_driver(Box::new(driver));
        assert!(manager.enabled("global-flag").await);
        assert!(manager.enabled_for("global-flag", "u").await);
        assert_eq!(
            manager.variant("global-flag", "u").await.unwrap(),
            "enabled"
        );

        assert!(!manager.enabled("unknown").await);
        assert!(!manager.enabled_for("unknown", "u").await);
        assert_eq!(manager.variant("unknown", "u").await, None);
    }

    #[test]
    fn test_feature_init() {
        use super::super::{init, manager};
        let manager1 = FeatureManager::new();
        let _ = init(manager1);
        let manager2 = FeatureManager::new();
        assert!(init(manager2).is_err());
        let _m = manager();
    }

    #[test]
    fn test_resolve_variant_boundary() {
        let variants = vec![("a".to_string(), 50), ("b".to_string(), 50)];
        // If bucket is exactly 50, it should hit the second variant because accumulator for 'a' is 50,
        // and 50 < 50 is false. So it moves to 'b'.
        let v = resolve_variant(&variants, 50);
        assert_eq!(v, Some("b".to_string()));
    }

    #[tokio::test]
    async fn test_toml_driver_empty_lines() {
        let driver = TomlFeatureDriver::new();
        driver.load_from_str("\n\n[features]\nflag = true\n# comment\n");
        assert_eq!(driver.enabled("flag").await, Some(true));
    }
}
