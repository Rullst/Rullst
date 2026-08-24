//! I2C transaction byte builder; no bus or register access is included.

extern crate alloc;
use alloc::vec::Vec;

/// Helper for constructing transaction bytes for a platform I2C adapter.
pub struct I2cHelper;

impl I2cHelper {
    /// Constructs a register read transaction frame.
    pub fn build_read_frame(device_addr: u8, reg_addr: u8, len: usize) -> Vec<u8> {
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
}
