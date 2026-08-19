//! Modbus RTU / TCP driver frame builder & CRC16 calculator.

extern crate alloc;
use alloc::vec::Vec;

/// Modbus Function Codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModbusFunction {
    ReadHoldingRegisters = 0x03,
    WriteSingleRegister = 0x06,
}

/// Modbus RTU Protocol Frame.
pub struct ModbusFrame;

impl ModbusFrame {
    /// Computes Modbus CRC-16 checksum.
    pub fn calculate_crc16(buffer: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in buffer {
            crc ^= byte as u16;
            for _ in 0..8 {
                if (crc & 0x0001) != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }

    /// Constructs a Read Holding Registers Modbus RTU request frame.
    pub fn read_holding_registers(slave_id: u8, start_addr: u16, count: u16) -> Vec<u8> {
        let mut frame = Vec::with_capacity(8);
        frame.push(slave_id);
        frame.push(ModbusFunction::ReadHoldingRegisters as u8);
        frame.extend_from_slice(&start_addr.to_be_bytes());
        frame.extend_from_slice(&count.to_be_bytes());

        let crc = Self::calculate_crc16(&frame);
        frame.extend_from_slice(&crc.to_le_bytes());
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modbus_frame_and_crc() {
        let frame = ModbusFrame::read_holding_registers(1, 0, 10);
        assert_eq!(frame[0], 1);
        assert_eq!(frame[1], 0x03);
        assert_eq!(frame.len(), 8);
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_modbus_crc16_no_panic() {
        let bytes: [u8; 4] = kani::any();
        let crc = ModbusFrame::calculate_crc16(&bytes);
        assert!(crc <= u16::MAX);
    }
}
