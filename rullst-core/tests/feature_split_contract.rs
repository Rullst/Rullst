//! One percentage-rollout and A/B-split contract, asserted against every
//! feature driver: Env, TOML, Memory and DB (on SQLite).
//!
//! `--all-features` selects the strict PostgreSQL pool type, so this SQLite
//! contract runs with `--features orm`, like `feature_db_cache`.
#![cfg(all(
    feature = "orm",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::feature::{
    DbFeatureDriver, EnvFeatureDriver, FeatureDriver, MemoryFeatureDriver, TomlFeatureDriver,
    calculate_hash_bucket, parse_variants, resolve_variant,
};

/// Each A/B split flag and the value every driver stores for it.
const SPLITS: [(&str, &str); 3] = [
    ("contract-narrow-split", "a:10,b:10"),
    ("contract-enabled-split", "enabled:50,control:50"),
    ("contract-late-enabled-split", "control:50,enabled:50"),
];
const ROLLOUT_FLAG: &str = "contract-rollout";
const ROLLOUT_PERCENT: u32 = 30;

fn env_key(flag: &str) -> String {
    format!("FEATURE_{}", flag.to_uppercase().replace('-', "_"))
}

/// The variant every driver must assign `identifier` for a split.
fn expected_variant(split: &str, flag: &str, identifier: &str) -> String {
    resolve_variant(
        &parse_variants(split),
        calculate_hash_bucket(flag, identifier),
    )
    .unwrap_or_else(|| "disabled".to_string())
}

async fn assert_split_contract(name: &str, driver: &impl FeatureDriver) {
    for (flag, split) in SPLITS {
        assert_eq!(
            driver.enabled(flag).await,
            Some(false),
            "{name}: an A/B split is not enabled without an identifier ({split})"
        );
        let mut gated_in = 0_usize;
        for index in 0..200 {
            let identifier = format!("user-{index}");
            let expected = expected_variant(split, flag, &identifier);
            assert_eq!(
                driver.variant(flag, &identifier).await.as_deref(),
                Some(expected.as_str()),
                "{name}: {split} variant of {identifier}"
            );
            let enabled = driver.enabled_for(flag, &identifier).await;
            assert_eq!(
                enabled,
                Some(expected == "enabled"),
                "{name}: {split} gate of {identifier}"
            );
            gated_in += usize::from(enabled == Some(true));
        }
        if split.contains("enabled:") {
            assert!((50..150).contains(&gated_in), "{name}: {split} {gated_in}");
        } else {
            assert_eq!(gated_in, 0, "{name}: {split}");
        }
    }

    assert_eq!(
        driver.enabled(ROLLOUT_FLAG).await,
        Some(false),
        "{name}: a rollout is not enabled without an identifier"
    );
    for index in 0..200 {
        let identifier = format!("user-{index}");
        let inside = calculate_hash_bucket(ROLLOUT_FLAG, &identifier) < ROLLOUT_PERCENT;
        assert_eq!(
            driver.enabled_for(ROLLOUT_FLAG, &identifier).await,
            Some(inside),
            "{name}: rollout gate of {identifier}"
        );
        assert_eq!(
            driver.variant(ROLLOUT_FLAG, &identifier).await.as_deref(),
            Some(if inside { "enabled" } else { "disabled" }),
            "{name}: rollout variant of {identifier}"
        );
    }
}

/// One test owns the process environment, working directory and ORM pool.
#[tokio::test]
async fn every_driver_evaluates_rollouts_and_ab_splits_alike() {
    let directory = tempfile::tempdir().expect("temporary directory");

    // Env. SAFETY: this binary runs a single test, so nothing else reads or
    // writes the process environment concurrently.
    let env_values = SPLITS
        .map(|(flag, split)| (env_key(flag), split.to_string()))
        .into_iter()
        .chain([(env_key(ROLLOUT_FLAG), format!("{ROLLOUT_PERCENT}%"))])
        .collect::<Vec<_>>();
    for (key, value) in &env_values {
        unsafe { std::env::set_var(key, value) };
    }
    assert_split_contract("env", &EnvFeatureDriver::new()).await;
    for (key, _) in &env_values {
        unsafe { std::env::remove_var(key) };
    }

    // TOML reads `Rullst.toml` from the working directory.
    let mut toml = String::from("[features]\n");
    for (flag, split) in SPLITS {
        toml.push_str(&format!("{flag} = \"{split}\"\n"));
    }
    toml.push_str(&format!("{ROLLOUT_FLAG} = \"{ROLLOUT_PERCENT}%\"\n"));
    std::fs::write(directory.path().join("Rullst.toml"), toml).expect("write Rullst.toml");
    let previous = std::env::current_dir().expect("working directory");
    std::env::set_current_dir(directory.path()).expect("enter temporary directory");
    let toml_driver = TomlFeatureDriver::new();
    std::env::set_current_dir(previous).expect("restore working directory");
    assert_split_contract("toml", &toml_driver).await;

    // Memory
    let memory = MemoryFeatureDriver::new();
    for (flag, split) in SPLITS {
        memory.override_variants(flag, parse_variants(split));
    }
    memory.override_rollout(ROLLOUT_FLAG, ROLLOUT_PERCENT);
    assert_split_contract("memory", &memory).await;

    // DB on SQLite
    let database = directory.path().join("flags.sqlite");
    rullst_orm::Orm::init_with_options(&format!("sqlite:{}?mode=rwc", database.display()), 4, 5)
        .await
        .expect("SQLite ORM initialization");
    let pool = rullst_core::db::safe_pool().expect("initialized pool");
    sqlx::query(
        "CREATE TABLE rullst_feature_flags (name TEXT PRIMARY KEY, enabled INTEGER NOT NULL, \
         rollout_percentage INTEGER, variants TEXT)",
    )
    .execute(pool)
    .await
    .expect("create rullst_feature_flags");
    for (flag, split) in SPLITS {
        sqlx::query("INSERT INTO rullst_feature_flags (name, enabled, variants) VALUES (?, 1, ?)")
            .bind(flag)
            .bind(split)
            .execute(pool)
            .await
            .expect("insert split flag");
    }
    sqlx::query(
        "INSERT INTO rullst_feature_flags (name, enabled, rollout_percentage) VALUES (?, 1, ?)",
    )
    .bind(ROLLOUT_FLAG)
    .bind(i64::from(ROLLOUT_PERCENT))
    .execute(pool)
    .await
    .expect("insert rollout flag");
    assert_split_contract("db", &DbFeatureDriver::new()).await;
}
