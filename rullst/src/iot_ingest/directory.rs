//! Where devices and their token digests live: the [`DeviceDirectory`]
//! contract and an in-memory implementation.

use super::token::{DeviceToken, TokenHash};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, MutexGuard, PoisonError};
use subtle::ConstantTimeEq;

/// Longest device id, in bytes.
pub(crate) const MAX_DEVICE_ID_BYTES: usize = 64;
/// Longest device name, in characters.
const MAX_NAME_CHARS: usize = 128;

/// One registered device, without its token.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeviceRecord {
    /// Stable identifier the device puts in every reading's `device_id`.
    pub device_id: String,
    /// Human-readable name.
    pub name: String,
    /// Firmware version the device's gateway reported last.
    pub firmware_version: Option<String>,
    /// Unix time (seconds) of the last accepted telemetry.
    pub last_seen_at: Option<u64>,
    /// Whether the token was revoked; a revoked device is never authenticated.
    pub revoked: bool,
}

impl DeviceRecord {
    /// A new, active device that has not reported yet.
    pub fn new(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            name: name.into(),
            firmware_version: None,
            last_seen_at: None,
            revoked: false,
        }
    }
}

/// Failure of a [`DeviceDirectory`]. Messages never contain tokens.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DirectoryError {
    /// The device id is not 1–64 bytes of ASCII letters, digits, `.`, `_` or
    /// `-`, or the name is empty, longer than 128 characters or holds
    /// control characters.
    InvalidDevice,
    /// A device with this id is already registered.
    AlreadyRegistered,
    /// No device has this id.
    NotFound,
    /// The store could not answer; the request fails closed.
    Unavailable(String),
}

impl fmt::Display for DirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDevice => formatter.write_str("invalid device id or name"),
            Self::AlreadyRegistered => formatter.write_str("device is already registered"),
            Self::NotFound => formatter.write_str("device not found"),
            Self::Unavailable(reason) => {
                write!(formatter, "device directory unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for DirectoryError {}

/// Checks a device id and name before a directory stores them: the id must
/// be 1–64 bytes of ASCII letters, digits, `.`, `_` or `-` (the receiver
/// matches it against every reading's `device_id`), and the name 1–128
/// characters without control characters. A directory over the
/// application's own table calls this in its registration code.
pub fn validate_device(device_id: &str, name: &str) -> Result<(), DirectoryError> {
    let id_ok = valid_device_id(device_id);
    let name_ok = !name.trim().is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && !name.chars().any(char::is_control);
    if id_ok && name_ok {
        Ok(())
    } else {
        Err(DirectoryError::InvalidDevice)
    }
}

pub(crate) fn valid_device_id(device_id: &str) -> bool {
    (1..=MAX_DEVICE_ID_BYTES).contains(&device_id.len())
        && device_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// The registry the telemetry receiver authenticates devices against.
///
/// [`super::TelemetryReceiver`] calls [`Self::find_active`] for every request
/// and [`Self::record_seen`] after it accepts telemetry. An implementation
/// reads current state (never a positive cache, so a revocation applies to
/// the next request) and fails closed with [`DirectoryError::Unavailable`].
/// [`MemoryDeviceDirectory`] keeps devices in memory; an application
/// implements this trait over its own table (the repository's
/// `examples/iot-gateway` does so with an ORM model).
///
/// Added in v13.0.0-alpha.5.
#[async_trait::async_trait]
pub trait DeviceDirectory: Send + Sync + 'static {
    /// The device whose token has `hash`, unless it is revoked or unknown.
    async fn find_active(&self, hash: &TokenHash) -> Result<Option<DeviceRecord>, DirectoryError>;

    /// Records that `device_id` delivered telemetry at `seen_at` (Unix
    /// seconds), with the firmware version its gateway reported, if any.
    async fn record_seen(
        &self,
        device_id: &str,
        seen_at: u64,
        firmware_version: Option<&str>,
    ) -> Result<(), DirectoryError>;
}

struct Entry {
    record: DeviceRecord,
    hash: TokenHash,
}

/// Devices kept in process memory: for tests, local tools and single-process
/// deployments that register their devices at startup. Nothing survives a
/// restart.
///
/// Added in v13.0.0-alpha.5.
#[derive(Default)]
pub struct MemoryDeviceDirectory {
    devices: Mutex<HashMap<String, Entry>>,
}

impl MemoryDeviceDirectory {
    /// An empty directory.
    pub fn new() -> Self {
        Self::default()
    }

    fn devices(&self) -> MutexGuard<'_, HashMap<String, Entry>> {
        self.devices.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers a device and returns its new token, the only time the
    /// secret is available.
    pub fn register(
        &self,
        device_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<DeviceToken, DirectoryError> {
        let (device_id, name) = (device_id.into(), name.into());
        validate_device(&device_id, &name)?;
        let mut devices = self.devices();
        if devices.contains_key(&device_id) {
            return Err(DirectoryError::AlreadyRegistered);
        }
        let token = DeviceToken::generate();
        let entry = Entry {
            record: DeviceRecord::new(device_id.clone(), name),
            hash: token.hash(),
        };
        devices.insert(device_id, entry);
        Ok(token)
    }

    /// Revokes a device's token; it is refused from the next request on.
    /// Returns whether the device was active.
    pub fn revoke(&self, device_id: &str) -> Result<bool, DirectoryError> {
        let mut devices = self.devices();
        let entry = devices.get_mut(device_id).ok_or(DirectoryError::NotFound)?;
        let was_active = !entry.record.revoked;
        entry.record.revoked = true;
        Ok(was_active)
    }

    /// The device with this id.
    pub fn get(&self, device_id: &str) -> Option<DeviceRecord> {
        self.devices()
            .get(device_id)
            .map(|entry| entry.record.clone())
    }

    /// Every device, ordered by id.
    pub fn list(&self) -> Vec<DeviceRecord> {
        let mut records: Vec<_> = self.devices().values().map(|e| e.record.clone()).collect();
        records.sort_by(|a, b| a.device_id.cmp(&b.device_id));
        records
    }
}

#[async_trait::async_trait]
impl DeviceDirectory for MemoryDeviceDirectory {
    async fn find_active(&self, hash: &TokenHash) -> Result<Option<DeviceRecord>, DirectoryError> {
        // Every entry is compared, so the time taken does not tell which one matched.
        let mut found = None;
        for entry in self.devices().values() {
            if bool::from(
                entry
                    .hash
                    .as_str()
                    .as_bytes()
                    .ct_eq(hash.as_str().as_bytes()),
            ) {
                found = Some(entry.record.clone());
            }
        }
        Ok(found.filter(|record| !record.revoked))
    }

    async fn record_seen(
        &self,
        device_id: &str,
        seen_at: u64,
        firmware_version: Option<&str>,
    ) -> Result<(), DirectoryError> {
        let mut devices = self.devices();
        let entry = devices.get_mut(device_id).ok_or(DirectoryError::NotFound)?;
        entry.record.last_seen_at = Some(seen_at);
        if let Some(version) = firmware_version {
            entry.record.firmware_version = Some(version.to_owned());
        }
        Ok(())
    }
}
