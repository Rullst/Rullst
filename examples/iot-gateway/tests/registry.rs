//! The `iot_devices` registry on SQLite: migration, registration, token
//! lookup, delivery bookkeeping, revocation and the Nexus projection.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst::db::schema::Migration;
use rullst::iot_ingest::{DeviceDirectory, DirectoryError, TokenHash};
use rullst_iot_gateway_example::registry::{CreateIotDevicesTable, IotDevice, SqlDeviceDirectory};

/// The database the compiled ORM pool speaks: "SQLite" or "Any" normally,
/// "PostgreSQL" or "MySQL" when Cargo feature unification (`cargo test
/// --workspace --all-features`) enables a strict driver. These tests need
/// SQLite and skip otherwise; `cargo test -p rullst-iot-gateway-example`
/// runs them.
fn sqlite_compiled() -> bool {
    fn name<DB: rullst_orm::sqlx::Database>(
        _: Option<&rullst_orm::sqlx::Pool<DB>>,
    ) -> &'static str {
        DB::NAME
    }
    let compiled = name(None::<&rullst_orm::RullstPool>);
    let usable = matches!(compiled, "SQLite" | "Any");
    if !usable {
        eprintln!("skipping: this example needs SQLite, but the ORM is compiled for {compiled}");
    }
    usable
}

#[tokio::test]
async fn devices_are_registered_authenticated_seen_and_revoked() {
    if !sqlite_compiled() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("iot.db").display());
    rullst_orm::Orm::init(&url).await.unwrap();
    CreateIotDevicesTable.up().await.unwrap();
    assert_eq!(
        CreateIotDevicesTable.name(),
        "m20261110000000_create_iot_devices_table"
    );

    let registry = SqlDeviceDirectory::new();
    let (uno_a, token_a) = registry.register("uno-a", "Greenhouse Uno").await.unwrap();
    let (_, token_b) = registry.register("uno-b", "Workshop Uno").await.unwrap();
    assert!(uno_a.id > 0);
    assert_eq!(uno_a.token_hash, token_a.hash().as_str());
    assert!(!uno_a.is_revoked());
    assert_eq!(
        registry.register("uno-a", "again").await.unwrap_err(),
        DirectoryError::AlreadyRegistered
    );
    assert_eq!(
        registry.register("bad id", "x").await.unwrap_err(),
        DirectoryError::InvalidDevice
    );

    // The digest finds the device; the token itself is never stored.
    let found = registry
        .find_active(&token_a.hash())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (found.device_id.as_str(), found.name.as_str()),
        ("uno-a", "Greenhouse Uno")
    );
    assert!(
        registry
            .find_active(&TokenHash::of("unknown"))
            .await
            .unwrap()
            .is_none()
    );
    let json = uno_a.to_json();
    assert!(!json.contains(token_a.hash().as_str()), "{json}");
    assert!(!json.contains(token_a.expose()), "{json}");

    registry
        .record_seen("uno-b", 1_800_000_000, Some("uno-sensor-1.0"))
        .await
        .unwrap();
    registry
        .record_seen("uno-b", 1_800_000_005, None)
        .await
        .unwrap();
    let uno_b = registry.find("uno-b").await.unwrap().unwrap();
    assert_eq!(uno_b.last_seen_at, Some(1_800_000_005));
    assert_eq!(uno_b.firmware_version.as_deref(), Some("uno-sensor-1.0"));
    assert_eq!(
        registry.record_seen("ghost", 1, None).await,
        Err(DirectoryError::NotFound)
    );

    assert!(registry.revoke("uno-a").await.unwrap());
    assert!(!registry.revoke("uno-a").await.unwrap());
    assert_eq!(
        registry.revoke("ghost").await,
        Err(DirectoryError::NotFound)
    );
    assert!(
        registry
            .find_active(&token_a.hash())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        registry
            .find_active(&token_b.hash())
            .await
            .unwrap()
            .is_some()
    );

    let devices = registry.list().await.unwrap();
    let ids: Vec<_> = devices
        .iter()
        .map(|device| device.device_id.as_str())
        .collect();
    assert_eq!(ids, ["uno-a", "uno-b"]);
    assert!(devices[0].record().revoked);
    assert_eq!(devices[1].record().last_seen_at, Some(1_800_000_005));

    CreateIotDevicesTable.down().await.unwrap();
    assert!(matches!(
        registry.find("uno-b").await,
        Err(DirectoryError::Unavailable(_))
    ));
}

#[test]
fn nexus_shows_devices_but_seals_their_token_digest() {
    use rullst::nexus::NexusModel;

    assert_eq!(IotDevice::nexus_table(), "iot_devices");
    assert_eq!(IotDevice::nexus_label(), "IoT devices");
    let fields = IotDevice::nexus_fields();
    let field = |name: &str| {
        fields
            .iter()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert!(!field("device_id").hidden && !field("revoked_at").readonly);
    assert!(field("token_hash").hidden && field("token_hash").readonly);
    for readonly in ["firmware_version", "last_seen_at", "registered_at"] {
        assert!(field(readonly).readonly, "{readonly}");
    }
    assert!(fields.iter().all(|field| field.name != "snapshot"));
}
