//! I2C transaction byte builder; no bus or register access is included.

extern crate alloc;
use alloc::vec::Vec;
use core::fmt;

/// Longest read accepted by the I2C frame builders, matching the Linux
/// `i2c-dev` per-message limit.
pub const MAX_I2C_READ_BYTES: usize = 8_192;
/// Address-write, register and address-read bytes before the read buffer.
const FRAME_HEADER_BYTES: usize = 3;

/// Fail-closed I2C read-frame construction errors.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I2cFrameError {
    /// The address is above `0x7F` or in a reserved range (`0x00..=0x07`,
    /// `0x78..=0x7F`).
    InvalidAddress,
    /// The read length exceeds [`MAX_I2C_READ_BYTES`].
    ReadTooLong,
    /// The frame buffer could not be allocated.
    AllocationFailed,
}

impl fmt::Display for I2cFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidAddress => "I2C address is not a non-reserved 7-bit device address",
            Self::ReadTooLong => "I2C read length exceeds the frame limit",
            Self::AllocationFailed => "I2C frame buffer could not be allocated",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for I2cFrameError {}

/// Helper for constructing transaction bytes for a platform I2C adapter.
pub struct I2cHelper;

/// Reports whether `address` is a 7-bit device address outside the ranges
/// that the I2C specification reserves (`0x00..=0x07` and `0x78..=0x7F`).
const fn is_device_address(address: u8) -> bool {
    matches!(address, 0x08..=0x77)
}

impl I2cHelper {
    /// Constructs a register read transaction frame.
    ///
    /// `device_addr` is the unshifted 7-bit address. An address above `0x7F`
    /// (such as an 8-bit datasheet form) or in a reserved range yields an empty
    /// frame, because shifting it would silently address another device or the
    /// general-call address. A `len` above [`MAX_I2C_READ_BYTES`], or one whose
    /// frame cannot be allocated, also yields an empty frame instead of
    /// panicking or aborting. [`Self::try_build_read_frame`] reports the reason.
    pub fn build_read_frame(device_addr: u8, reg_addr: u8, len: usize) -> Vec<u8> {
        Self::try_build_read_frame(device_addr, reg_addr, len).unwrap_or_default()
    }

    /// Constructs a register read transaction frame or reports why it cannot.
    ///
    /// The frame is the write-mode address byte, `reg_addr`, the read-mode
    /// address byte and `len` zero bytes for the data to be read.
    pub fn try_build_read_frame(
        device_addr: u8,
        reg_addr: u8,
        len: usize,
    ) -> Result<Vec<u8>, I2cFrameError> {
        if !is_device_address(device_addr) {
            return Err(I2cFrameError::InvalidAddress);
        }
        if len > MAX_I2C_READ_BYTES {
            return Err(I2cFrameError::ReadTooLong);
        }
        let frame_len = FRAME_HEADER_BYTES + len;
        let mut frame = Vec::new();
        frame
            .try_reserve_exact(frame_len)
            .map_err(|_| I2cFrameError::AllocationFailed)?;
        frame.push(device_addr << 1); // Write mode
        frame.push(reg_addr);
        frame.push((device_addr << 1) | 1); // Read mode
        frame.resize(frame_len, 0x00);
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i2c_frame_builder() {
        let frame = I2cHelper::build_read_frame(0x68, 0x3B, 2);
        assert_eq!(frame[0], 0xD0);
        assert_eq!(frame[1], 0x3B);
        assert_eq!(frame[2], 0xD1);
        assert_eq!(frame.len(), 5);
    }

    #[test]
    fn non_7_bit_and_reserved_addresses_produce_no_frame() {
        // 0x80 would become the general-call write address 0x00, and the
        // 8-bit datasheet form 0xD0 of 0x68 would address 0x50 instead.
        for address in [0x00, 0x07, 0x78, 0x7F, 0x80, 0xA0, 0xD0, 0xFF] {
            assert!(
                I2cHelper::build_read_frame(address, 0x06, 2).is_empty(),
                "address {address:#04x}"
            );
        }
        for address in [0x08, 0x50, 0x68, 0x77] {
            let frame = I2cHelper::build_read_frame(address, 0x3B, 2);
            assert_eq!(frame, [address << 1, 0x3B, (address << 1) | 1, 0, 0]);
        }
    }

    #[test]
    fn oversized_read_lengths_produce_no_frame() {
        assert_eq!(I2cHelper::build_read_frame(0x68, 0x3B, 8_192).len(), 8_195);
        for len in [8_193, 0x0010_0000, usize::MAX - 1, usize::MAX] {
            assert!(
                I2cHelper::build_read_frame(0x68, 0x3B, len).is_empty(),
                "len {len}"
            );
        }
    }

    #[test]
    fn fallible_builder_reports_why_no_frame_was_built() {
        assert_eq!(
            I2cHelper::try_build_read_frame(0x68, 0x3B, 2),
            Ok(I2cHelper::build_read_frame(0x68, 0x3B, 2))
        );
        assert_eq!(
            I2cHelper::try_build_read_frame(0xD0, 0x3B, 2),
            Err(I2cFrameError::InvalidAddress)
        );
        assert_eq!(
            I2cHelper::try_build_read_frame(0x68, 0x3B, MAX_I2C_READ_BYTES + 1),
            Err(I2cFrameError::ReadTooLong)
        );
        assert_eq!(
            I2cHelper::try_build_read_frame(0x68, 0x3B, MAX_I2C_READ_BYTES)
                .map(|frame| frame.len()),
            Ok(MAX_I2C_READ_BYTES + 3)
        );
        assert!(!I2cFrameError::InvalidAddress.to_string().is_empty());
    }
}
