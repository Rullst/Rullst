#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_iot::modbus::ModbusFrame;

fuzz_target!(|data: &[u8]| {
    let _ = ModbusFrame::calculate_crc16(data);
});
