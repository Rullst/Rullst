use super::RollbackCounterError;
use core::fmt;

/// Errors returned by the signed OTA gate.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OtaError {
    EmptyTarget,
    EmptyVersion,
    EmptyFirmware,
    ManifestFieldTooLong,
    FirmwareTooLarge,
    InvalidTrustedKey,
    InvalidSignatureEncoding,
    SignatureInvalid,
    FirmwareLengthMismatch { expected: u64, actual: u64 },
    FirmwareHashMismatch,
    TargetMismatch,
    RollbackDetected { current: u64, proposed: u64 },
    RollbackCounterStore(RollbackCounterError),
    NoVerifiedUpdate,
    LegacyApiUnsupported { replacement: &'static str },
}

impl fmt::Display for OtaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTarget => formatter.write_str("OTA target must not be empty"),
            Self::EmptyVersion => formatter.write_str("firmware version must not be empty"),
            Self::EmptyFirmware => formatter.write_str("firmware payload must not be empty"),
            Self::ManifestFieldTooLong => {
                formatter.write_str("OTA manifest target or version exceeds the encoded limit")
            }
            Self::FirmwareTooLarge => {
                formatter.write_str("firmware length cannot be represented by this platform")
            }
            Self::InvalidTrustedKey => formatter.write_str("trusted Ed25519 public key is invalid"),
            Self::InvalidSignatureEncoding => {
                formatter.write_str("Ed25519 signature must contain exactly 64 bytes")
            }
            Self::SignatureInvalid => formatter.write_str("Ed25519 signature is invalid"),
            Self::FirmwareLengthMismatch { expected, actual } => write!(
                formatter,
                "firmware length mismatch: manifest declares {expected} bytes, received {actual}"
            ),
            Self::FirmwareHashMismatch => {
                formatter.write_str("firmware SHA-256 digest does not match the signed manifest")
            }
            Self::TargetMismatch => {
                formatter.write_str("firmware manifest targets a different device class")
            }
            Self::RollbackDetected { current, proposed } => write!(
                formatter,
                "rollback counter must increase: current {current}, proposed {proposed}"
            ),
            Self::RollbackCounterStore(error) => write!(formatter, "{error}"),
            Self::NoVerifiedUpdate => formatter.write_str("no verified firmware update is pending"),
            Self::LegacyApiUnsupported { replacement } => write!(
                formatter,
                "legacy OTA API is fail-closed; use {replacement}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OtaError {}

impl From<RollbackCounterError> for OtaError {
    fn from(error: RollbackCounterError) -> Self {
        Self::RollbackCounterStore(error)
    }
}
