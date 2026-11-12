//! The application's device registry: the `iot_devices` ORM model, its
//! migration and a [`DeviceDirectory`] over them.
//!
//! This is application code, not a Rullst API: copy it and change the table
//! as the application needs. [`SqlDeviceDirectory::register`] returns the
//! token once; [`IotDevice`] derives `Nexus`, so the admin lists devices and
//! can set `revoked_at`, while the token digest stays hidden there and in
//! the model's JSON. The directory uses the ORM's global pool, so call it
//! after `Orm::init` (`Server::with_db` does this).

use rullst::db::schema::{Migration, Schema};
use rullst::db::{Nexus, Orm, RowSnapshot};
use rullst::iot_ingest::{
    DeviceDirectory, DeviceRecord, DeviceToken, DirectoryError, TokenHash, validate_device,
};
use rullst::orm::Error;
use std::time::{SystemTime, UNIX_EPOCH};

/// One registered device: a row of `iot_devices`.
#[derive(Clone, Debug, Orm, Nexus)]
#[orm(table = "iot_devices")]
#[nexus(label = "IoT devices", icon = "📟")]
pub struct IotDevice {
    /// Row id.
    pub id: i32,
    /// Stable identifier the device puts in its readings (unique).
    pub device_id: String,
    /// Human-readable name.
    pub name: String,
    /// SHA-256 of the device's bearer token, as `TokenHash` hex (unique).
    /// Hidden from the model's JSON and from Nexus.
    #[orm(hidden)]
    pub token_hash: String,
    /// Firmware version the gateway reported last.
    #[nexus(readonly)]
    pub firmware_version: Option<String>,
    /// Unix time (seconds) of the last accepted telemetry.
    #[nexus(readonly)]
    pub last_seen_at: Option<i64>,
    /// Unix time (seconds) of the registration.
    #[nexus(readonly)]
    pub registered_at: i64,
    /// Unix time (seconds) of the revocation; a revoked device is refused.
    pub revoked_at: Option<i64>,
    /// Lets `save()` write only the columns it changed.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl IotDevice {
    /// Whether the token was revoked.
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    /// The device as a [`DeviceRecord`].
    pub fn record(&self) -> DeviceRecord {
        let mut record = DeviceRecord::new(self.device_id.clone(), self.name.clone());
        record.firmware_version = self.firmware_version.clone();
        record.last_seen_at = self.last_seen_at.and_then(|at| u64::try_from(at).ok());
        record.revoked = self.is_revoked();
        record
    }
}

/// Creates `iot_devices` (bounded `VARCHAR` columns for the unique device id
/// and token digest, so MySQL/MariaDB can index them).
pub struct CreateIotDevicesTable;

#[rullst::async_trait]
impl Migration for CreateIotDevicesTable {
    fn name(&self) -> &'static str {
        "m20261110000000_create_iot_devices_table"
    }

    async fn up(&self) -> Result<(), Error> {
        Schema::create("iot_devices", |table| {
            table.id();
            let device_id = table.string("device_id");
            device_id.col_type = "VARCHAR(64)".to_owned();
            device_id.not_null().unique();
            table.string("name").not_null();
            let token_hash = table.string("token_hash");
            token_hash.col_type = "VARCHAR(64)".to_owned();
            token_hash.not_null().unique();
            table.string("firmware_version").nullable();
            table.big_integer("last_seen_at").nullable();
            table.big_integer("registered_at").not_null();
            table.big_integer("revoked_at").nullable();
        })
        .await
    }

    async fn down(&self) -> Result<(), Error> {
        Schema::drop_if_exists("iot_devices").await
    }
}

/// A [`DeviceDirectory`] over the `iot_devices` table.
#[derive(Clone, Copy, Debug, Default)]
pub struct SqlDeviceDirectory;

fn unavailable(error: Error) -> DirectoryError {
    DirectoryError::Unavailable(error.to_string())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

impl SqlDeviceDirectory {
    /// The directory; it holds no state of its own.
    pub fn new() -> Self {
        Self
    }

    /// Registers a device and returns its row and its new token, the only
    /// time the secret is available.
    pub async fn register(
        &self,
        device_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<(IotDevice, DeviceToken), DirectoryError> {
        let (device_id, name) = (device_id.into(), name.into());
        validate_device(&device_id, &name)?;
        let token = DeviceToken::generate();
        let mut device = IotDevice {
            id: 0,
            device_id,
            name,
            token_hash: token.hash().as_str().to_owned(),
            firmware_version: None,
            last_seen_at: None,
            registered_at: unix_now(),
            revoked_at: None,
            snapshot: RowSnapshot::default(),
        };
        match device.save().await {
            Ok(()) => Ok((device, token)),
            Err(Error::UniqueViolation { .. }) => Err(DirectoryError::AlreadyRegistered),
            Err(error) => Err(unavailable(error)),
        }
    }

    /// Revokes a device's token; it is refused from the next request on.
    /// Returns whether the device was active.
    pub async fn revoke(&self, device_id: &str) -> Result<bool, DirectoryError> {
        let mut device = self
            .find(device_id)
            .await?
            .ok_or(DirectoryError::NotFound)?;
        if device.is_revoked() {
            return Ok(false);
        }
        device.revoked_at = Some(unix_now());
        device.save().await.map_err(unavailable)?;
        Ok(true)
    }

    /// The device with this id.
    pub async fn find(&self, device_id: &str) -> Result<Option<IotDevice>, DirectoryError> {
        IotDevice::query()
            .where_eq("device_id", device_id)
            .first()
            .await
            .map_err(unavailable)
    }

    /// Every device, ordered by device id (at most 1,000).
    pub async fn list(&self) -> Result<Vec<IotDevice>, DirectoryError> {
        IotDevice::query()
            .order_by("device_id")
            .limit(1000)
            .get()
            .await
            .map_err(unavailable)
    }
}

#[rullst::async_trait]
impl DeviceDirectory for SqlDeviceDirectory {
    async fn find_active(&self, hash: &TokenHash) -> Result<Option<DeviceRecord>, DirectoryError> {
        let device = IotDevice::query()
            .where_eq("token_hash", hash.as_str())
            .where_null("revoked_at")
            .first()
            .await
            .map_err(unavailable)?;
        Ok(device.map(|device| device.record()))
    }

    async fn record_seen(
        &self,
        device_id: &str,
        seen_at: u64,
        firmware_version: Option<&str>,
    ) -> Result<(), DirectoryError> {
        let mut device = self
            .find(device_id)
            .await?
            .ok_or(DirectoryError::NotFound)?;
        device.last_seen_at = Some(i64::try_from(seen_at).unwrap_or(i64::MAX));
        if let Some(version) = firmware_version {
            device.firmware_version = Some(version.to_owned());
        }
        device.save().await.map_err(unavailable)
    }
}
