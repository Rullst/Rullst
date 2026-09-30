//! I2C transaction byte builder; no bus or register access is included.

extern crate alloc;
use alloc::vec::Vec;

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
    /// general-call address.
    pub fn build_read_frame(device_addr: u8, reg_addr: u8, len: usize) -> Vec<u8> {
        if !is_device_address(device_addr) {
            return Vec::new();
        }
        let mut frame = Vec::with_capacity(3 + len);
        frame.push(device_addr << 1); // Write mode
        frame.push(reg_addr);
        frame.push((device_addr << 1) | 1); // Read mode
        frame.resize(3 + len, 0x00);
        frame
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
}
